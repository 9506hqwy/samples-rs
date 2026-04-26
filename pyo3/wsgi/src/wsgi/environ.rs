use httparse::Request;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::io::Read;
use std::net::{SocketAddr, TcpStream};
use std::str::{Utf8Error, from_utf8};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct WsgiEnviron {
    request_method: String,
    script_name: String,
    path_info: String,
    query_string: Option<String>,
    content_type: Option<String>,
    content_length: Option<String>,
    server_name: String,
    server_port: String,
    server_protocol: String,
    remote_addr: String,
    remote_host: String,
    https: bool,
    http_headers: HashMap<String, String>,
    debug: bool,
}

impl WsgiEnviron {
    pub fn new(req: &Request, remote_addr: String, debug: bool) -> Result<WsgiEnviron, Utf8Error> {
        let mut headers = HashMap::new();
        let mut content_type = None;
        let mut content_length = None;
        let mut local_addr = "127.0.0.1:80".parse::<SocketAddr>().unwrap();
        for header in req.headers.iter() {
            let name = header.name.to_uppercase();
            if name == "CONTENT-TYPE" {
                content_type = Some(from_utf8(header.value)?.to_string());
                continue;
            }

            if name == "CONTENT-LENGTH" {
                content_length = Some(from_utf8(header.value)?.to_string());
                continue;
            }

            if name == "HOST" {
                local_addr = from_utf8(header.value)?.parse().unwrap_or(local_addr);
                continue;
            }

            let h = format!("HTTP_{}", header.name.replace("-", "_").to_uppercase());
            let v = from_utf8(header.value)?.to_string();

            headers.insert(h, v);
        }

        let mut p2q = req.path.unwrap_or("/").splitn(2, '?');

        let env = WsgiEnviron {
            request_method: req.method.unwrap_or("GET").to_string(),
            script_name: "".to_string(),
            path_info: p2q.nth(0).unwrap_or("/").to_string(),
            query_string: p2q.nth(0).map(|q| q.to_string()),
            content_type,
            content_length,
            server_name: local_addr.ip().to_string(),
            server_port: local_addr.port().to_string(),
            server_protocol: format!("HTTP/1.{}", req.version.unwrap_or(0)),
            remote_addr: remote_addr.clone(),
            remote_host: remote_addr,
            https: false,
            http_headers: headers,
            debug,
        };

        Ok(env)
    }

    pub fn to_dict(&self, py: Python, stream: TcpStream) -> PyResult<Py<PyDict>> {
        let d = PyDict::new(py);
        d.set_item("REQUEST_METHOD", self.request_method.clone())?;
        d.set_item("SCRIPT_NAME", self.script_name.clone())?;
        d.set_item("PATH_INFO", self.path_info.clone())?;

        if let Some(qs) = &self.query_string {
            d.set_item("QUERY_STRING", qs.clone())?;
        }

        if let Some(ct) = &self.content_type {
            d.set_item("CONTENT_TYPE", ct.clone())?;
        }

        if let Some(cl) = &self.content_length {
            d.set_item("CONTENT_LENGTH", cl.clone())?;
        }

        d.set_item("SERVER_NAME", self.server_name.clone())?;
        d.set_item("SERVER_PORT", self.server_port.clone())?;
        d.set_item("SERVER_PROTOCOL", self.server_protocol.clone())?;
        d.set_item("REMOTE_ADDR", self.remote_addr.clone())?;
        d.set_item("REMOTE_HOST", self.remote_host.clone())?;
        d.set_item("HTTPS", if self.https { "on" } else { "off" })?;

        for (key, value) in &self.http_headers {
            d.set_item(key.clone(), value.clone())?;
        }

        let length = self
            .content_length
            .clone()
            .unwrap_or_else(|| "0".to_string())
            .parse()
            .unwrap();

        let input = Py::new(py, WsgiInput::new(stream, length, self.debug))?;
        d.set_item("wsgi.input", input)?;

        let errors = Py::new(py, WsgiError {})?;
        d.set_item("wsgi.errors", errors)?;

        d.set_item("wsgi.version", (1, 0))?;
        d.set_item("wsgi.url_scheme", if self.https { "https" } else { "http" })?;
        d.set_item("wsgi.multithread", true)?;
        d.set_item("wsgi.multiprocess", false)?;
        d.set_item("wsgi.run_once", false)?;

        Ok(d.into())
    }
}

#[pyclass]
pub struct WsgiError;

#[pymethods]
impl WsgiError {
    fn flush(&self) -> PyResult<()> {
        Ok(())
    }

    fn write(&self, error: String) -> PyResult<()> {
        eprintln!("{}", error);
        Ok(())
    }

    fn writelines(&self, errors: Vec<String>) -> PyResult<()> {
        for error in errors {
            eprintln!("{}", error);
        }
        Ok(())
    }
}

#[pyclass]
pub struct WsgiInput {
    stream: TcpStream,
    length: usize,
    read_length: AtomicUsize,
    debug: bool,
}

impl WsgiInput {
    pub fn new(stream: TcpStream, length: usize, debug: bool) -> Self {
        WsgiInput {
            stream,
            length,
            read_length: AtomicUsize::new(0),
            debug,
        }
    }
}

#[pymethods]
impl WsgiInput {
    fn read(&mut self, size: usize) -> PyResult<Vec<u8>> {
        let mut buf = vec![0; size];
        self.stream.read_exact(&mut buf).ok();

        *self.read_length.get_mut() += size;

        if self.debug {
            println!(
                "readed bytes length: {}(+{})",
                self.read_length.load(Ordering::SeqCst),
                size
            );
        }

        Ok(buf)
    }

    #[pyo3(signature = (size=None))]
    fn readline(&mut self, size: Option<usize>) -> PyResult<Vec<u8>> {
        let limit = size.unwrap_or(self.length);

        let mut buf = Vec::new();
        let mut b = [0u8; 1];
        while self.stream.read_exact(&mut b).is_ok() {
            buf.extend_from_slice(&b);

            if b[0] == b'\n' {
                break;
            }

            if limit == buf.len() {
                break;
            }
        }

        *self.read_length.get_mut() += buf.len();

        if self.debug {
            println!(
                "readed line length: {}(+{}/{})",
                self.read_length.load(Ordering::SeqCst),
                buf.len(),
                limit,
            );
        }

        Ok(buf)
    }

    fn readlines(&mut self, _hint: u64) -> PyResult<Vec<Vec<u8>>> {
        let mut lines = vec![];

        loop {
            let line = self.readline(None)?;
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }

        Ok(lines)
    }

    // https://pyo3.rs/v0.28.3/class/protocols#iterable-objects
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> PyResult<Option<Vec<u8>>> {
        if self.read_length.load(Ordering::SeqCst) < self.length {
            return Ok(Some(self.readline(None)?));
        }

        Ok(None)
    }
}
