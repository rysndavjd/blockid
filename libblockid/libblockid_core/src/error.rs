#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuilderError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Error(pub(crate) ErrorKind);

impl From<ErrorKind> for Error {
    fn from(e: ErrorKind) -> Self {
        Self(e)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Todo,
}

// impl crate::std::error::Error for Error {}
