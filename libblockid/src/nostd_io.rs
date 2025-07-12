use core::fmt::{self, Debug};

use alloc::{boxed::Box, string::{String, ToString}};
use rustix::{fd::{AsFd, BorrowedFd, OwnedFd}, 
    fs::{open as rustix_open, seek, Mode, OFlags},
    io::{read as rustix_read, Errno}, path::Arg};

// Copied from std::io
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    HostUnreachable,
    NetworkUnreachable,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    NetworkDown,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    NotADirectory,
    IsADirectory,
    DirectoryNotEmpty,
    ReadOnlyFilesystem,
    StaleNetworkFileHandle,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    StorageFull,
    NotSeekable,
    QuotaExceeded,
    FileTooLarge,
    ResourceBusy,
    ExecutableFileBusy,
    Deadlock,
    CrossesDevices,
    TooManyLinks,
    InvalidFilename,
    ArgumentListTooLong,
    Interrupted,
    Unsupported,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "Not Found"),
            Self::PermissionDenied => write!(f, "Permission Denied"),
            Self::ConnectionRefused => write!(f, "Connection Refused"),
            Self::ConnectionReset => write!(f, "Connection Reset"),
            Self::HostUnreachable => write!(f, "Host Unreachable"),
            Self::NetworkUnreachable => write!(f, "Network Unreachable"),
            Self::ConnectionAborted => write!(f, "Connection Aborted"),
            Self::NotConnected => write!(f, "Not Connected"),
            Self::AddrInUse => write!(f, "Addr In Use"),
            Self::AddrNotAvailable => write!(f, "Addr Not Available"),
            Self::NetworkDown => write!(f, "Network Down"),
            Self::BrokenPipe => write!(f, "Broken Pipe"),
            Self::AlreadyExists => write!(f, "Already Exists"),
            Self::WouldBlock => write!(f, "Would Block"),
            Self::NotADirectory => write!(f, "Not A Directory"),
            Self::IsADirectory => write!(f, "Is A Directory"),
            Self::DirectoryNotEmpty => write!(f, "Directory Not Empty"),
            Self::ReadOnlyFilesystem => write!(f, "Read Only Filesystem"),
            Self::StaleNetworkFileHandle => write!(f, "Stale Network File Handle"),
            Self::InvalidInput => write!(f, "Invalid Input"),
            Self::InvalidData => write!(f, "Invalid Data"),
            Self::TimedOut => write!(f, "Timed Out"),
            Self::WriteZero => write!(f, "Write Zero"),
            Self::StorageFull => write!(f, "Storage Full"),
            Self::NotSeekable => write!(f, "Not Seekable"),
            Self::QuotaExceeded => write!(f, "Quota Exceeded"),
            Self::FileTooLarge => write!(f, "File Too Large"),
            Self::ResourceBusy => write!(f, "Resource Busy"),
            Self::ExecutableFileBusy => write!(f, "Executable File Busy"),
            Self::Deadlock => write!(f, "Deadlock"),
            Self::CrossesDevices => write!(f, "Crosses Devices"),
            Self::TooManyLinks => write!(f, "Too Many Links"),
            Self::InvalidFilename => write!(f, "Invalid Filename"),
            Self::ArgumentListTooLong => write!(f, "Argument List Too Long"),
            Self::Interrupted => write!(f, "Interrupted"),
            Self::Unsupported => write!(f, "Unsupported"),
            Self::UnexpectedEof => write!(f, "Unexpected End of File"),
            Self::OutOfMemory => write!(f, "Out Of Memory"),
            Self::Other => write!(f, "Other"),
        }
    }   
}   

#[cfg(feature = "std")]
impl From<ErrorKind> for std::io::ErrorKind {
    fn from(err: ErrorKind) -> Self {
        match err {
                ErrorKind::NotFound => std::io::ErrorKind::NotFound,
                ErrorKind::PermissionDenied => std::io::ErrorKind::PermissionDenied,
                ErrorKind::ConnectionRefused => std::io::ErrorKind::ConnectionRefused,
                ErrorKind::ConnectionReset => std::io::ErrorKind::ConnectionReset,
                ErrorKind::HostUnreachable => std::io::ErrorKind::HostUnreachable,
                ErrorKind::NetworkUnreachable => std::io::ErrorKind::NetworkUnreachable,
                ErrorKind::ConnectionAborted => std::io::ErrorKind::ConnectionAborted,
                ErrorKind::NotConnected => std::io::ErrorKind::NotConnected,
                ErrorKind::AddrInUse => std::io::ErrorKind::AddrInUse,
                ErrorKind::AddrNotAvailable => std::io::ErrorKind::AddrNotAvailable,
                ErrorKind::NetworkDown => std::io::ErrorKind::NetworkDown,
                ErrorKind::BrokenPipe => std::io::ErrorKind::BrokenPipe,
                ErrorKind::AlreadyExists => std::io::ErrorKind::AlreadyExists,
                ErrorKind::WouldBlock => std::io::ErrorKind::WouldBlock,
                ErrorKind::NotADirectory => std::io::ErrorKind::NotADirectory,
                ErrorKind::IsADirectory => std::io::ErrorKind::IsADirectory,
                ErrorKind::DirectoryNotEmpty => std::io::ErrorKind::DirectoryNotEmpty,
                ErrorKind::ReadOnlyFilesystem => std::io::ErrorKind::ReadOnlyFilesystem,
                ErrorKind::StaleNetworkFileHandle => std::io::ErrorKind::StaleNetworkFileHandle,
                ErrorKind::InvalidInput => std::io::ErrorKind::InvalidInput,
                ErrorKind::InvalidData => std::io::ErrorKind::InvalidData,
                ErrorKind::TimedOut => std::io::ErrorKind::TimedOut,
                ErrorKind::WriteZero => std::io::ErrorKind::WriteZero,
                ErrorKind::StorageFull => std::io::ErrorKind::StorageFull,
                ErrorKind::NotSeekable => std::io::ErrorKind::NotSeekable,
                ErrorKind::QuotaExceeded => std::io::ErrorKind::QuotaExceeded,
                ErrorKind::FileTooLarge => std::io::ErrorKind::FileTooLarge,
                ErrorKind::ResourceBusy => std::io::ErrorKind::ResourceBusy,
                ErrorKind::ExecutableFileBusy => std::io::ErrorKind::ExecutableFileBusy,
                ErrorKind::Deadlock => std::io::ErrorKind::Deadlock,
                ErrorKind::CrossesDevices => std::io::ErrorKind::CrossesDevices,
                ErrorKind::TooManyLinks => std::io::ErrorKind::TooManyLinks,
                ErrorKind::InvalidFilename => std::io::ErrorKind::InvalidFilename,
                ErrorKind::ArgumentListTooLong => std::io::ErrorKind::ArgumentListTooLong,
                ErrorKind::Interrupted => std::io::ErrorKind::Interrupted,
                ErrorKind::Unsupported => std::io::ErrorKind::Unsupported,
                ErrorKind::UnexpectedEof => std::io::ErrorKind::UnexpectedEof,
                ErrorKind::OutOfMemory => std::io::ErrorKind::OutOfMemory,
                ErrorKind::Other => std::io::ErrorKind::Other,
                _ => std::io::ErrorKind::Other
        }
    }
}

impl From<ErrorKind> for NoStdIoError {
    fn from(err: ErrorKind) -> Self {
        return NoStdIoError::Kind(err);
    }
}

impl From<Errno> for NoStdIoError {
    fn from(err: Errno) -> Self {
        return NoStdIoError::NixError(err);
    }
}

// Took from std::io
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

impl From<SeekFrom> for rustix::fs::SeekFrom {
    fn from(pos: SeekFrom) -> Self {
        match pos {
            SeekFrom::Start(n) => rustix::fs::SeekFrom::Start(n),
            SeekFrom::End(n) => rustix::fs::SeekFrom::End(n),
            SeekFrom::Current(n) => rustix::fs::SeekFrom::Current(n),
        }
    }
}

impl TryFrom<rustix::fs::SeekFrom> for SeekFrom {
    type Error = ();

    fn try_from(pos: rustix::fs::SeekFrom) -> Result<Self, Self::Error> {
        match pos {
            rustix::fs::SeekFrom::Start(n) => Ok(SeekFrom::Start(n)),
            rustix::fs::SeekFrom::End(n) => Ok(SeekFrom::End(n)),
            rustix::fs::SeekFrom::Current(n) => Ok(SeekFrom::Current(n)),
            _ => Err(()),
        }
    }
}

#[cfg(feature = "std")]
impl From<SeekFrom> for std::io::SeekFrom {
    fn from(pos: SeekFrom) -> Self {
        match pos {
            SeekFrom::Start(n) => std::io::SeekFrom::Start(n),
            SeekFrom::End(n) => std::io::SeekFrom::End(n),
            SeekFrom::Current(n) => std::io::SeekFrom::Current(n),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::SeekFrom> for SeekFrom {
    fn from(pos: std::io::SeekFrom) -> Self {
        match pos {
            std::io::SeekFrom::Start(n) => SeekFrom::Start(n),
            std::io::SeekFrom::End(n) => SeekFrom::End(n),
            std::io::SeekFrom::Current(n) => SeekFrom::Current(n),
        }
    }
}

// Took from std::io
pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, NoStdIoError>;

    fn rewind(&mut self) -> Result<(), NoStdIoError> {
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    fn stream_len(&mut self) -> Result<u64, NoStdIoError> {
        let old_pos = self.stream_position()?;
        let len = self.seek(SeekFrom::End(0))?;

        if old_pos != len {
            self.seek(SeekFrom::Start(old_pos))?;
        }

        return Ok(len);
    }

    fn stream_position(&mut self) -> Result<u64, NoStdIoError> {
        return Ok(self.seek(SeekFrom::Current(0))?);
    }

    fn seek_relative(&mut self, offset: i64) -> Result<(), NoStdIoError> {
        let _ = self.seek(SeekFrom::Current(offset))?;
        return Ok(());
    }
}

// took from embedded-io
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, NoStdIoError>;
    
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), NoStdIoError> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    buf = &mut buf[n..];
                }
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() { 
            return Err(ErrorKind::UnexpectedEof.into());
        } else { 
            return Ok(());
        }
    }
}

// Took from std::fs
#[derive(Debug, Clone, Copy)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,

    custom_flags: i32,
    mode: Mode,
}

impl OpenOptions {
    pub fn new() -> Self {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            custom_flags: 0,
            mode: Mode::from(0o666),
        }
    }

    pub fn read(&mut self, read: bool) -> Self {
        self.read = read;
        *self
    }
    pub fn write(&mut self, write: bool) -> Self {
        self.write = write;
        *self
    }
    pub fn append(&mut self, append: bool) -> Self {
        self.append = append;
        *self
    }
    pub fn truncate(&mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        *self
    }
    pub fn create(&mut self, create: bool) -> Self {
        self.create = create;
        *self
    }
    pub fn create_new(&mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        *self
    }
    
    pub fn custom_flags(&mut self, flags: i32) -> Self {
        self.custom_flags = flags;
        *self
    }

    pub fn mode(&mut self, mode: u32) -> Self {
        self.mode = Mode::from(mode);
        *self
    }

    fn get_access_mode(&self) -> Result<OFlags, NoStdIoError> {
        match (self.read, self.write, self.append) {
            (true, false, false) => Ok(OFlags::RDONLY),
            (false, true, false) => Ok(OFlags::WRONLY),
            (true, true, false) => Ok(OFlags::RDWR),
            (false, _, true) => Ok(OFlags::WRONLY | OFlags::APPEND),
            (true, _, true) => Ok(OFlags::RDWR | OFlags::APPEND),
            (false, false, false) => Err(Errno::INVAL.into()),
        }
    }

    fn get_creation_mode(&self) -> Result<OFlags, NoStdIoError> {
        match (self.write, self.append) {
            (true, false) => {}
            (false, false) => {
                if self.truncate || self.create || self.create_new {
                    return Err(Errno::INVAL.into());
                }
            }
            (_, true) => {
                if self.truncate && !self.create_new {
                    return Err(Errno::INVAL.into());
                }
            }
        }

        Ok(match (self.create, self.truncate, self.create_new) {
            (false, false, false) => OFlags::from_bits_truncate(0),
            (true, false, false) => OFlags::CREATE,
            (false, true, false) => OFlags::TRUNC,
            (true, true, false) => OFlags::CREATE | OFlags::TRUNC,
            (_, _, true) => OFlags::CREATE | OFlags::EXCL,
        })
    }

    pub fn open<P: Arg>(&self, path: P) -> Result<File, NoStdIoError> {
        let flags = OFlags::CLOEXEC
            | self.get_access_mode()?
            | self.get_creation_mode()?
            | OFlags::from_bits_retain(self.custom_flags as u32 & !OFlags::ACCMODE.bits());
        
        let fd = rustix_open(path, flags, self.mode)?;

        return Ok(
            File {
                inner: fd,
            }
        );
    }
}

#[derive(Debug)]
pub struct File {
    inner: OwnedFd,
}

impl File {
    pub fn open<P: Arg>(path: P) -> Result<File, NoStdIoError> {
        OpenOptions::new().read(true).open(path)
    }

    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }
}

impl Seek for File {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, NoStdIoError> {
        return Ok(seek(self.as_fd(), pos.into())?)
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, NoStdIoError> {
        Ok(rustix_read(self.as_fd(), buf)?)
    }
}

impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.inner.as_fd()
    }
}

#[derive(Debug)]
pub enum NoStdIoError {
    Kind(ErrorKind),
    NixError(Errno),
    Custom{
        kind: ErrorKind,
        error: &'static str
    },
}

impl NoStdIoError {
    pub fn new(kind: ErrorKind, error: &'static str) -> Self {
        NoStdIoError::Custom { 
            kind: kind, 
            error: error,
        }
    }
}

impl fmt::Display for NoStdIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoStdIoError::Kind(e) => write!(f, "no_std I/O error: {:?}", e),
            NoStdIoError::NixError(e) => write!(f, "*Nix error code: {}", e),
            NoStdIoError::Custom{ kind, error } => write!(f, "Kind: {}, Error: {}", kind, error),
        }
    }
}


