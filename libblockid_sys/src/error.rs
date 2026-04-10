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

#[cfg(all(not(feature = "std"), target_family = "unix"))]
impl From<rustix::io::Errno> for Error {
    fn from(e: rustix::io::Errno) -> Self {
        Self(ErrorKind::IoError(e.into()))
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    IoError(crate::io::Error),
    PathNotUtf8
}

impl From<crate::io::Error> for ErrorKind {
    fn from(e: crate::io::Error) -> Self {
        Self::IoError(e)
    }
}
