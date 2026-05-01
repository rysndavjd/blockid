#[cfg(feature = "std")]
pub use std::io::SeekFrom;

#[cfg(feature = "no_std")]
pub use embedded_io::{ErrorKind, SeekFrom};

use crate::{error::Error, probe::Magic};

pub trait BlockIo: crate::std::fmt::Debug {
    type Error: crate::std::fmt::Debug;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error>;

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error>;
}

#[cfg(feature = "std")]
impl<R: std::io::Read + std::io::Seek + std::fmt::Debug> BlockIo for R {
    type Error = std::io::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_exact(buf)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.seek(pos)
    }
}

#[cfg(feature = "no_std")]
impl<
    E: From<embedded_io::ErrorKind> + core::fmt::Debug,
    R: embedded_io::Read + embedded_io::Seek<Error = E> + core::fmt::Debug,
> BlockIo for R
where
    embedded_io::ErrorKind: core::convert::From<E>,
{
    type Error = R::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.read_exact(buf).map_err(|e| match e {
            embedded_io::ReadExactError::UnexpectedEof => ErrorKind::InvalidInput.into(),
            embedded_io::ReadExactError::Other(e) => e,
        })
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.seek(pos)
    }
}

#[derive(Debug)]
pub struct Reader<IO: BlockIo>(IO);

impl<IO: BlockIo> Reader<IO> {
    pub fn new(reader: IO) -> Self {
        Self(reader)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, IO::Error> {
        self.0.read(buf)
    }

    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), IO::Error> {
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(buf)?;
        Ok(())
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), IO::Error> {
        self.0.read_exact(buf)
    }

    pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, IO::Error> {
        self.0.seek(pos)
    }

    pub fn read_exact_at<const S: usize>(&mut self, offset: u64) -> Result<[u8; S], IO::Error> {
        let mut buf = [0u8; S];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_vec_at(&mut self, offset: u64, size: usize) -> Result<Vec<u8>, IO::Error> {
        let mut buf = vec![0u8; size];
        self.0.seek(SeekFrom::Start(offset))?;
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn get_magic(
        &mut self,
        magics: &'static [Magic],
    ) -> Result<Option<Magic>, Error<IO::Error>> {
        let mut buf = [0u8; 16];

        for magic in magics {
            debug_assert!(
                magic.len <= buf.len(),
                "Magic should not be greater then `buf`"
            );

            self.read_at(magic.b_offset, &mut buf)
                .map_err(|e| Error::Io(e))?;

            if &buf[..magic.len] == magic.magic {
                return Ok(Some(*magic));
            }
        }

        return Ok(None);
    }
}

#[cfg(feature = "std")]
pub use std::{fs::File, io::Error as IoError};

#[cfg(all(feature = "no_std", target_family = "unix"))]
pub use impl_unix::{Error as IoError, File};

#[cfg(all(feature = "no_std", target_family = "unix"))]
mod impl_unix {
    use embedded_io::{
        Error as EmbeddedError, ErrorKind, ErrorType as EmbeddedErrorType, Read, Seek,
        SeekFrom as EmbeddedSeekFrom,
    };
    use rustix::{
        fd::{AsFd, BorrowedFd, OwnedFd},
        fs::{Mode, OFlags, SeekFrom as RustixSeekFrom, open, seek},
        io::{Errno, read},
    };

    use crate::path::SysPath;

    #[derive(Debug)]
    pub struct Error(Errno);

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "os error {}", self.0.raw_os_error())
        }
    }

    impl core::error::Error for Error {}

    impl From<Errno> for Error {
        fn from(e: Errno) -> Self {
            Self(e)
        }
    }

    impl From<ErrorKind> for Error {
        fn from(e: ErrorKind) -> Self {
            Self(match e {
                ErrorKind::NotFound => Errno::NODEV,
                ErrorKind::PermissionDenied => Errno::ACCESS,
                ErrorKind::ConnectionRefused => Errno::CONNREFUSED,
                ErrorKind::ConnectionReset => Errno::CONNRESET,
                ErrorKind::ConnectionAborted => Errno::CONNABORTED,
                ErrorKind::NotConnected => Errno::NOTCONN,
                ErrorKind::AddrInUse => Errno::ADDRINUSE,
                ErrorKind::AddrNotAvailable => Errno::ADDRNOTAVAIL,
                ErrorKind::BrokenPipe => Errno::PIPE,
                ErrorKind::AlreadyExists => Errno::EXIST,
                ErrorKind::InvalidInput => Errno::INVAL,
                ErrorKind::InvalidData => Errno::ILSEQ,
                ErrorKind::TimedOut => Errno::TIMEDOUT,
                ErrorKind::Interrupted => Errno::INTR,
                ErrorKind::Unsupported => Errno::NOTSUP,
                ErrorKind::OutOfMemory => Errno::NOMEM,
                _ => Errno::IO,
            })
        }
    }

    impl From<Error> for embedded_io::ErrorKind {
        fn from(e: Error) -> embedded_io::ErrorKind {
            e.kind()
        }
    }

    impl EmbeddedError for Error {
        fn kind(&self) -> ErrorKind {
            match self.0 {
                Errno::NOENT | Errno::NODEV | Errno::NXIO => ErrorKind::NotFound,
                Errno::PERM | Errno::ACCESS => ErrorKind::PermissionDenied,
                Errno::CONNREFUSED => ErrorKind::ConnectionRefused,
                Errno::CONNRESET => ErrorKind::ConnectionReset,
                Errno::CONNABORTED => ErrorKind::ConnectionAborted,
                Errno::NOTCONN => ErrorKind::NotConnected,
                Errno::ADDRINUSE => ErrorKind::AddrInUse,
                Errno::ADDRNOTAVAIL => ErrorKind::AddrNotAvailable,
                Errno::PIPE | Errno::NOLINK => ErrorKind::BrokenPipe,
                Errno::EXIST => ErrorKind::AlreadyExists,
                Errno::INVAL | Errno::BADF | Errno::FAULT => ErrorKind::InvalidInput,
                Errno::ILSEQ | Errno::BADMSG | Errno::PROTO => ErrorKind::InvalidData,
                Errno::TIMEDOUT => ErrorKind::TimedOut,
                Errno::INTR => ErrorKind::Interrupted,
                Errno::NOSYS | Errno::NOTSUP => ErrorKind::Unsupported,
                Errno::NOMEM => ErrorKind::OutOfMemory,
                _ => ErrorKind::Other,
            }
        }
    }

    #[derive(Debug)]
    pub struct File {
        inner: OwnedFd,
    }

    impl File {
        pub fn open<P: SysPath>(path: P) -> Result<File, Error> {
            let fd = open(path.as_ref().as_bytes(), OFlags::RDONLY, Mode::empty())?;

            Ok(Self { inner: fd })
        }
    }

    impl EmbeddedErrorType for File {
        type Error = Error;
    }

    impl Read for File {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            let out = read(&self.inner, buf)?;
            Ok(out)
        }
    }

    impl Seek for File {
        fn seek(&mut self, pos: EmbeddedSeekFrom) -> Result<u64, Self::Error> {
            let new_pos = match pos {
                EmbeddedSeekFrom::Start(pos) => RustixSeekFrom::Start(pos),
                EmbeddedSeekFrom::End(pos) => RustixSeekFrom::Current(pos),
                EmbeddedSeekFrom::Current(pos) => RustixSeekFrom::Current(pos),
            };

            let ret = seek(&self.inner, new_pos)?;
            Ok(ret)
        }
    }

    impl AsFd for File {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.inner.as_fd()
        }
    }

    impl From<OwnedFd> for File {
        fn from(fd: OwnedFd) -> Self {
            File { inner: fd }
        }
    }
}

#[cfg(all(feature = "no_std", target_family = "windows"))]
mod windows {}
