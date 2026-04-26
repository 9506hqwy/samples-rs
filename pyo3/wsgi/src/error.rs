use pyo3::PyErr;
use std::convert::From;
use std::io;
use std::str;

#[derive(Debug)]
pub enum CPyError {
    Decode(str::Utf8Error),
    Socket(io::Error),
    PyErr(PyErr),
}

impl From<str::Utf8Error> for CPyError {
    fn from(e: str::Utf8Error) -> Self {
        CPyError::Decode(e)
    }
}

impl From<io::Error> for CPyError {
    fn from(e: io::Error) -> Self {
        CPyError::Socket(e)
    }
}

impl From<PyErr> for CPyError {
    fn from(e: PyErr) -> Self {
        CPyError::PyErr(e)
    }
}
