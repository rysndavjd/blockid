#[derive(Debug)]
pub struct Error(ErrorKind);

impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Self(e)
    }
}

impl From<crate::io::Error> for Error {
    fn from(e: crate::io::Error) -> Self {
        Self(ErrorKind::IoError(e))
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    IoError(crate::io::Error),
}

impl From<crate::io::Error> for ErrorKind {
    fn from(e: crate::io::Error) -> Self {
        Self::IoError(e)
    }
}
