#[derive(Debug)]
pub struct Error(ErrorKind);

impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Self(e)
    }
}

#[derive(Debug)]
pub enum ErrorKind {}
