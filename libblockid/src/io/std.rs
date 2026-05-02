pub use std::{
    fs::File,
    io::{Error as IoError, SeekFrom},
};

use rustix::io::Errno;

use crate::error::Error;

impl From<IoError> for Error<IoError> {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<Errno> for Error<IoError> {
    fn from(e: Errno) -> Self {
        match e {
            
        }
    }
}
