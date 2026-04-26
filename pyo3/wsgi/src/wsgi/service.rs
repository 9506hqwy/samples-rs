use super::WsgiEnviron;
use crate::error::CPyError;
use httparse::Request;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};

fn call(
    py: Python,
    path: &Path,
    stream: &TcpStream,
    environ: &WsgiEnviron,
    start_response: &Py<StartResponse>,
) -> PyResult<Py<PyAny>> {
    let app_name = path.file_stem().unwrap().to_str().unwrap();
    let app = Python::import(py, app_name)?;

    let cloned = stream.try_clone()?;
    let env = environ.to_dict(py, cloned)?;
    let py_env = env.bind(py);

    let py_start_response = start_response.bind(py);

    let pyobj = app.call_method1("application", (py_env, py_start_response))?;

    Ok(pyobj.into())
}

fn extract(py: Python, obj: &Py<PyAny>, stream: &mut TcpStream) -> PyResult<()> {
    let obj_ref = obj.bind(py);
    for ret in obj_ref.try_iter()? {
        let item = ret?;
        if let Ok(pyb) = item.extract::<String>() {
            stream.write_all(pyb.as_bytes()).ok();
        } else if let Ok(pyb) = item.extract::<&[u8]>() {
            stream.write_all(pyb).ok();
        } else {
            extract(py, &item.into(), stream)?;
        }
    }

    Ok(())
}

fn set_py_path(py: Python, path: &Path) -> PyResult<Py<PyAny>> {
    let sys = Python::import(py, "sys")?;
    let py_path = sys.getattr("path")?;
    let base_name = path.parent().unwrap().to_string_lossy().into_owned();
    let pyobj = py_path.call_method1("append", (base_name,))?;
    Ok(pyobj.into())
}

pub struct WsgiService {
    remote_addr: SocketAddr,
    application: PathBuf,
    debug: bool,
}

impl WsgiService {
    pub fn new(application: &Path, remote_addr: SocketAddr, debug: bool) -> WsgiService {
        WsgiService {
            application: application.to_path_buf(),
            remote_addr,
            debug,
        }
    }

    pub fn exec(&self, req: Request, stream: &mut TcpStream) -> Result<(), CPyError> {
        let environ = WsgiEnviron::new(&req, self.remote_addr.ip().to_string(), self.debug)?;
        if self.debug {
            println!("{:?}", &environ);
        }

        Python::attach(|py| {
            set_py_path(py, self.application.as_path())?;

            let cloned = stream.try_clone()?;
            let start_response = Py::new(py, StartResponse::new(cloned, self.debug))?;

            let pyobj = call(
                py,
                self.application.as_path(),
                stream,
                &environ,
                &start_response,
            )?;

            extract(py, &pyobj, stream)?;

            Ok(())
        })
    }
}

#[pyclass]
pub struct StartResponse {
    stream: TcpStream,
    exc_info: Option<Py<PyTuple>>,
    debug: bool,
}

impl StartResponse {
    pub fn new(stream: TcpStream, debug: bool) -> Self {
        StartResponse {
            stream,
            exc_info: None,
            debug,
        }
    }
}

#[pymethods]
impl StartResponse {
    #[pyo3(signature = (status, response_headers, exc_info=None))]
    fn __call__(
        &mut self,
        status: String,
        response_headers: Vec<(String, String)>,
        exc_info: Option<Py<PyTuple>>,
    ) -> PyResult<Py<WriteBody>> {
        self.exc_info = exc_info;

        let status_line = format!("HTTP/1.1 {0}\r\n", status);
        self.stream.write_all(status_line.as_bytes()).ok();

        for (n, v) in response_headers {
            let header_line = format!("{0}: {1}\r\n", n, v);
            self.stream.write_all(header_line.as_bytes()).ok();
        }

        let crlf_line = "\r\n";
        self.stream.write_all(crlf_line.as_bytes()).ok();

        let stream = self.stream.try_clone().unwrap();
        let write_body = WriteBody::new(stream, self.debug);
        Python::attach(|py| Py::new(py, write_body))
    }
}

#[pyclass]
pub struct WriteBody {
    stream: TcpStream,
    debug: bool,
}

impl WriteBody {
    pub fn new(stream: TcpStream, debug: bool) -> Self {
        WriteBody { stream, debug }
    }
}

#[pymethods]
impl WriteBody {
    fn __call__(&mut self, body_data: Py<PyAny>) -> PyResult<()> {
        Python::attach(|py| {
            if let Ok(data) = body_data.extract::<String>(py) {
                self.stream.write_all(data.as_bytes()).ok();
            } else if let Ok(data) = body_data.extract::<&[u8]>(py) {
                self.stream.write_all(data).ok();
            } else {
                let mut cloned = self.stream.try_clone().unwrap();
                extract(py, &body_data, &mut cloned).ok();
            }

            if self.debug {
                println!("Write body called");
            }

            Ok(())
        })
    }
}
