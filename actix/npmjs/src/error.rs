use std::convert::From;
use std::io;
use std::str;

#[derive(Debug)]
pub enum Error {
    Decode(str::Utf8Error),
    Package(io::Error),
}

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Decode(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Package(e)
    }
}
