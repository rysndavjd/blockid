#[cfg(all(feature = "os_calls", target_os = "macos"))]
mod macos;

use widestring::U16String;

use crate::Endianness;
#[cfg(feature = "os_calls")]
use crate::{
    Probe,
    error::{Error, IdToDiskError, PartToDiskError},
    filesystem::{FsFilter, FsId},
    io::{IoError, Path, PathBuf},
    partition::{PartitionId, PtFilter, PtId},
};

pub(crate) fn bytes_to_u16string(bytes: &[u8], endianness: Endianness) -> U16String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| match endianness {
            Endianness::Little => u16::from_le_bytes([c[0], c[1]]),
            Endianness::Big => u16::from_be_bytes([c[0], c[1]]),
        })
        .collect();

    U16String::from_vec(units)
}

/// Gets the path of a file descriptor, returning a [`PathBuf`](crate::io::PathBuf).
///
/// # Platform-specific behavior
///
/// ## Linux
/// Uses the `/proc` filesystem via `/proc/self/fd/<fd>`, reading the symlink target.
///
/// ## macOS
/// Uses [`fcntl`] with `F_GETPATH` to retrieve the path.
///
/// ## FreeBSD
/// TODO!.
///
/// [`fcntl`]: https://docs.rs/libc/latest/libc/fn.fcntl.html
#[cfg(feature = "os_calls")]
pub fn fd_to_path<F: rustix::fd::AsRawFd>(fd: F) -> Result<PathBuf, IoError> {
    #[cfg(target_os = "linux")]
    {
        let link = rustix::fs::readlink(format!("/proc/self/fd/{}", fd.as_raw_fd()), Vec::new())?;

        #[cfg(feature = "std")]
        {
            use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

            return Ok(PathBuf::from(OsStr::from_bytes(link.as_bytes())));
        }

        #[cfg(feature = "no_std")]
        return Ok(PathBuf::from(link.as_bytes()));
    }

    #[cfg(target_os = "macos")]
    {
        use libc::{__error, F_GETPATH, PATH_MAX, fcntl};
        use rustix::io::Errno;

        let mut buf = [0u8; PATH_MAX as usize];
        let ret = unsafe { fcntl(fd.as_raw_fd(), F_GETPATH, buf.as_mut_ptr()) };

        if ret == -1 {
            return Err(Errno::from_raw_os_error(unsafe { *__error() }).into());
        }

        #[cfg(feature = "std")]
        {
            use std::ffi::OsStr;
            use std::os::unix::ffi::OsStrExt;

            return Ok(PathBuf::from(OsStr::from_bytes(&buf)));
        }

        #[cfg(feature = "no_std")]
        return Ok(PathBuf::from(buf.as_slice()));
    }

    #[cfg(target_os = "freebsd")]
    {
        todo!()
    }
}

#[cfg(feature = "os_calls")]
pub fn part_to_disk<P: AsRef<Path>>(path: P) -> Result<PathBuf, PartToDiskError<IoError>> {
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{Access, access, major, minor, readlink, stat};

        #[cfg(feature = "std")]
        let path = path.as_ref();
        #[cfg(feature = "no_std")]
        let path = path.as_ref().as_bytes();

        let dev = stat(path)?.st_rdev;

        let sys_path = format!("/sys/dev/block/{}:{}", major(dev), minor(dev));

        if access(format!("{sys_path}/partition"), Access::EXISTS).is_ok() {
            let link = readlink(sys_path, Vec::new())?;

            let last = link
                .as_bytes()
                .iter()
                .rposition(|b| b == &b'/')
                .ok_or(PartToDiskError::DiskNotPartition)?;

            let second_last = link.as_bytes()[..last]
                .iter()
                .rposition(|b| b == &b'/')
                .ok_or(PartToDiskError::DiskNotPartition)?;

            let disk = &link.as_bytes()[second_last.saturating_add(1)..last];

            #[cfg(feature = "std")]
            {
                use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

                return Ok(Path::new("/dev").join(OsStr::from_bytes(disk)));
            }
            #[cfg(feature = "no_std")]
            {
                return Ok(Path::new("/dev/").join(disk));
            }
        }
        return Err(PartToDiskError::DiskNotPartition);
    }

    #[cfg(target_os = "macos")]
    {
        // I separated the macOS code into a separate module due to the ffi
        // interfaces and other junk that would just clutter this file up.
        macos::part_to_disk(path)
    }

    #[cfg(target_os = "freebsd")]
    {
        todo!()
    }
}

#[cfg(any(target_os = "linux", feature = "os_calls"))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Id<'a> {
    Uuid(&'a uuid::Uuid),
    VolumeId32(&'a fat_volume_id::id32::VolumeId32),
    VolumeId64(&'a fat_volume_id::id64::VolumeId64),
    /// This will change when I make a separte crate like volumeid
    Mbr(u32),
}

#[cfg(any(target_os = "linux", feature = "os_calls"))]
impl<'a> From<&'a uuid::Uuid> for Id<'a> {
    fn from(value: &'a uuid::Uuid) -> Self {
        Id::Uuid(value)
    }
}

#[cfg(any(target_os = "linux", feature = "os_calls"))]
impl<'a> From<&'a fat_volume_id::id32::VolumeId32> for Id<'a> {
    fn from(value: &'a fat_volume_id::id32::VolumeId32) -> Self {
        Id::VolumeId32(value)
    }
}

#[cfg(any(target_os = "linux", feature = "os_calls"))]
impl<'a> From<&'a fat_volume_id::id64::VolumeId64> for Id<'a> {
    fn from(value: &'a fat_volume_id::id64::VolumeId64) -> Self {
        Id::VolumeId64(value)
    }
}

#[cfg(any(target_os = "linux", feature = "os_calls"))]
#[inline]
fn find_partition(
    sys_block: &rustix::fd::OwnedFd,
    disk_name: &str,
    want_no: u64,
) -> Result<Option<String>, IdToDiskError<IoError>> {
    use lexical_core::{FormattedSize, parse};
    use rustix::{
        fs::{Dir, Mode, OFlags, openat},
        io::read,
    };

    // /sys/block/<device>/
    let part_block = openat(
        sys_block,
        disk_name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;

    for entry in Dir::read_from(&part_block)? {
        let entry = entry?;
        let part_name = entry
            .file_name()
            .to_str()
            .map_err(|_| IdToDiskError::InvalidDevName)?;

        if !part_name.starts_with(disk_name) {
            continue;
        }

        // /sys/block/<device>/<partition>/partition
        let partition = openat(
            &part_block,
            format!("{part_name}/partition"),
            OFlags::RDONLY | OFlags::CLOEXEC,
            Mode::empty(),
        )?;

        let mut buf = [0u8; u64::FORMATTED_SIZE_DECIMAL];
        let s = read(partition, &mut buf)?;

        let part_no =
            parse::<u64>(buf[..s].trim_ascii_end()).map_err(|_| IdToDiskError::InvalidPartition)?;

        if part_no == want_no {
            return Ok(Some(part_name.to_string()));
        }
    }
    Ok(None)
}

#[cfg(any(target_os = "linux", feature = "os_calls"))]
pub fn id_to_dev<'a, T: Into<Id<'a>>>(id: T) -> Result<PathBuf, IdToDiskError<IoError>> {
    use rustix::fs::{Dir, Mode, OFlags, open};

    let id = id.into();

    let sys_block = open(
        "/sys/block/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;

    for entry in Dir::read_from(&sys_block)? {
        let entry = entry?;
        let disk_name = entry
            .file_name()
            .to_str()
            .map_err(|_| IdToDiskError::InvalidDevName)?;

        if disk_name == "." || disk_name == ".." {
            continue;
        }

        let path = PathBuf::from(format!("/dev/{disk_name}"));

        let mut probe = match Probe::open(&path, 0) {
            Ok(t) => t,
            Err(Error::Io(e)) => return Err(IdToDiskError::Io(e)),
            _ => continue,
        };

        if let Ok(info) = probe.probe_part_table(PtFilter::all()) {
            if let Some(pt_id) = info.pt_id() {
                match (pt_id, id) {
                    (PtId::Uuid(f_uuid), Id::Uuid(g_uuid)) if f_uuid == *g_uuid => {
                        return Ok(path);
                    }
                    (PtId::Mbr(f_mbr), Id::Mbr(g_mbr)) if f_mbr == g_mbr => {
                        return Ok(path);
                    }
                    _ => (),
                }
            }

            if let Some(partitions) = info.partitions() {
                for part in partitions {
                    if let (PartitionId::Uuid(f_uuid), Id::Uuid(g_uuid)) = (part.partition_id, id)
                        && f_uuid == *g_uuid
                        && let Some(name) = find_partition(&sys_block, disk_name, part.part_no)?
                    {
                        return Ok(PathBuf::from(format!("/dev/{name}")));
                    }
                }
            }
        }

        if let Ok(info) = probe.probe_filesystem(FsFilter::all())
            && let Some(fs_id) = info.fs_id()
        {
            match (fs_id, id) {
                (FsId::Uuid(f_uuid), Id::Uuid(g_uuid)) if &f_uuid == g_uuid => {
                    return Ok(path);
                }
                (FsId::VolumeId32(f_volumeid32), Id::VolumeId32(g_volumeid32))
                    if &f_volumeid32 == g_volumeid32 =>
                {
                    return Ok(path);
                }
                (FsId::VolumeId64(f_volumeid64), Id::VolumeId64(g_volumeid64))
                    if &f_volumeid64 == g_volumeid64 =>
                {
                    return Ok(path);
                }
                _ => (),
            }
        }
    }

    Err(IdToDiskError::SearchExhausted)
}
