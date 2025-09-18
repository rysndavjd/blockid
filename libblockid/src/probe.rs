use std::{
    fmt,
    fs::File,
    io::{BufReader, Error as IoError, ErrorKind as IoErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use bitflags::bitflags;
use rustix::{
    fd::AsFd,
    fs::{Dev, FileType, Mode, fstat, major, minor},
};
use uuid::Uuid;
use zerocopy::FromBytes;

#[cfg(target_os = "linux")]
use crate::ioctl::{OpalStatusFlags, ioctl_blkgetzonesz, ioctl_ioc_opal_get_status};
use crate::ioctl::{device_size_bytes, logical_block_size};

use crate::{
    BlockidError,
    containers::luks::{LUKS_OPAL_ID_INFO, LUKS1_ID_INFO, LUKS2_ID_INFO},
    filesystems::{
        apfs::APFS_ID_INFO,
        exfat::EXFAT_ID_INFO,
        ext::{EXT2_ID_INFO, EXT3_ID_INFO, EXT4_ID_INFO, JBD_ID_INFO},
        linux_swap::{LINUX_SWAP_V0_ID_INFO, LINUX_SWAP_V1_ID_INFO, SWSUSPEND_ID_INFO},
        ntfs::NTFS_ID_INFO,
        squashfs::{SQUASHFS_ID_INFO, SQUASHFS3_ID_INFO},
        vfat::VFAT_ID_INFO,
        volume_id::{VolumeId32, VolumeId64},
        xfs::XFS_ID_INFO,
    },
    partitions::{
        dos::DOS_PT_ID_INFO,
        //gpt::GPT_PT_ID_INFO
    },
};

/// Probe table defining the order of detection attempts.
#[rustfmt::skip]
pub const PROBES: &[(ProbeFilter, ProbeFilter, BlockidIdinfo)] = &[
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS1, LUKS1_ID_INFO),
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS2, LUKS2_ID_INFO),
    (ProbeFilter::SKIP_CONT, ProbeFilter::SKIP_LUKS_OPAL, LUKS_OPAL_ID_INFO),
    (ProbeFilter::SKIP_PT, ProbeFilter::SKIP_DOS, DOS_PT_ID_INFO),
    //(ProbeFilter::SKIP_PT, ProbeFilter::SKIP_GPT, GPT_PT_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_APFS, APFS_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXFAT, EXFAT_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT2, EXT2_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT3, EXT3_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_EXT4, EXT4_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_JBD, JBD_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_LINUX_SWAP_V0, LINUX_SWAP_V0_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_LINUX_SWAP_V1, LINUX_SWAP_V1_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_SWSUSPEND, SWSUSPEND_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_NTFS, NTFS_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_VFAT, VFAT_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_XFS, XFS_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_SQUASHFS3, SQUASHFS3_ID_INFO),
    (ProbeFilter::SKIP_FS, ProbeFilter::SKIP_SQUASHFS, SQUASHFS_ID_INFO),
];

const SUPPORTED_TYPE: &[BlockType] = &[
    BlockType::LUKS1,
    BlockType::LUKS2,
    BlockType::LUKSOpal,
    BlockType::Dos,
    BlockType::Exfat,
    BlockType::Apfs,
    BlockType::Ext2,
    BlockType::Ext3,
    BlockType::Ext4,
    BlockType::Jbd,
    BlockType::LinuxSwapV0,
    BlockType::LinuxSwapV1,
    BlockType::Ntfs,
    BlockType::Vfat,
    BlockType::Xfs,
    BlockType::Squashfs3,
    BlockType::Squashfs,
];

const SUPPORTED_STR: &[&str] = &[
    "LUKS1",
    "LUKS2",
    "LUKS Opal",
    "DOS",
    "GPT",
    "EXFAT",
    "JBD",
    "APFS",
    "EXT2",
    "EXT3",
    "EXT4",
    "NTFS",
    "Linux Swap V0",
    "Linux Swap V1",
    "Swap Suspend",
    "VFAT",
    "XFS",
    "SquashFS",
    "SquashFS3",
];

/// Represents a probe session on a file or block device.
///
/// A [`Probe`] provides access to the underlying file or device and stores
/// the results of container, partition table, or filesystem detection.
/// It encapsulates both the device metadata and the probe state.
///
/// The probe can optionally use buffered I/O to reduce system calls when
/// reading multiple sectors.
///
/// # Fields
/// - `file`: The open [`File`] or block device being probed.
/// - `path`: Path to the file or device.
/// - `buffer`: Optional buffered reader (`BufReader`) for optimized I/O.
/// - `offset`: Starting offset in bytes for the probe.
/// - `size`: Total size in bytes of the file or device.
/// - `io_size`: Recommended I/O block size (`st_blksize` from [`fstat`](rustix::fs::fstat)).
/// - `devno`: Device number of the file (`st_rdev`).
/// - `disk_devno`: Device number of the disk containing the file (`st_dev`).
/// - `sector_size`: Logical block size in bytes.
/// - `mode`: File mode bits (`Mode`) used to determine file type.
///
/// # Platform-specific
/// - `zone_size` (Linux only): Optional zone size of the block device, queried
///   via the `BLKGETZONESZ` ioctl. `None` if the file is not a block device
///   or on non-Linux platforms.
///
/// - `flags`: Current [`ProbeFlags`] set for this probe.
/// - `filter`: Active [`ProbeFilter`] restricting which probes are run.
/// - `value`: The detected [`ProbeResult`] after running `probe_values()`.
#[derive(Debug)]
pub struct Probe {
    file: File,
    path: PathBuf,
    buffer: Option<BufReader<File>>,
    offset: u64,
    size: u64,
    io_size: i64,

    devno: Dev,
    disk_devno: Dev,
    sector_size: u64,
    mode: Mode,
    #[cfg(target_os = "linux")]
    zone_size: Option<u64>,

    flags: ProbeFlags,
    filter: ProbeFilter,
    value: Option<ProbeResult>,
}

impl Probe {
    /// Returns all supported superblocks as strings in a array
    pub fn supported_string() -> &'static [&'static str] {
        SUPPORTED_STR
    }

    /// Returns all supported superblocks in a array
    pub fn supported_type() -> &'static [BlockType] {
        SUPPORTED_TYPE
    }

    /// Create a probe from a [`File`].
    ///
    /// - Reads file metadata via [`fstat`](rustix::fs::fstat).
    /// - If the file is a block device:
    ///   - queries the logical block size and total size in bytes using kernel ioctls.
    /// - If the file is not a block device:
    ///   - defaults logical block size to `512` bytes,
    ///   - uses the file size from [`fstat`](rustix::fs::fstat).
    ///
    /// # Platform-specific
    /// On **Linux**:
    /// - An additional ioctl (`BLKGETZONESZ`) is used to query the device’s zone size.
    pub fn new(
        file: File,
        path: &Path,
        offset: u64,
        flags: ProbeFlags,
        filter: ProbeFilter,
    ) -> Result<Probe, BlockidError> {
        let stat = fstat(file.as_fd())?;

        #[cfg(target_os = "linux")]
        let (sector_size, size, zone_size) =
            if FileType::from_raw_mode(stat.st_mode).is_block_device() {
                (
                    u64::from(logical_block_size(file.as_fd())?),
                    device_size_bytes(file.as_fd())?,
                    Some(u64::from(ioctl_blkgetzonesz(file.as_fd())? << 9)),
                )
            } else {
                (512, stat.st_size as u64, None)
            };

        #[cfg(not(target_os = "linux"))]
        let (sector_size, size) = if FileType::from_raw_mode(stat.st_mode).is_block_device() {
            (
                u64::from(logical_block_size(file.as_fd())?),
                device_size_bytes(file.as_fd())?,
            )
        } else {
            (512, stat.st_size as u64)
        };

        Ok(Self {
            file,
            path: path.to_path_buf(),
            buffer: None,
            offset,
            size,
            /* Some architectures uses different integer size in blksize in its stat field */
            #[allow(clippy::useless_conversion)]
            io_size: stat.st_blksize.into(),
            devno: stat.st_rdev,
            disk_devno: stat.st_dev,
            sector_size,
            mode: Mode::from(stat.st_mode),
            #[cfg(target_os = "linux")]
            zone_size,
            flags,
            filter,
            value: None,
        })
    }

    /// Enable buffered I/O on the underlying [`File`].
    ///
    /// Creates a [`BufReader`] with defined capacity.
    ///
    /// # Errors
    /// Returns [`IoError`] if cloning the file descriptor fails.
    pub fn enable_buffering_with_capacity(&mut self, capacity: usize) -> Result<(), IoError> {
        let clone = self.file.try_clone()?;
        self.buffer = Some(BufReader::with_capacity(capacity, clone));
        return Ok(());
    }

    /// Enable buffered I/O on the underlying [`File`].
    ///
    /// Creates a [`BufReader`] with capacity equal to the device’s reported
    /// I/O block size.
    ///
    /// # Errors
    /// Returns [`IoError`] if cloning the file descriptor fails.
    pub fn enable_buffering(&mut self) -> Result<(), IoError> {
        self.enable_buffering_with_capacity(self.io_size as usize)?;
        return Ok(());
    }

    pub(crate) fn seek(&mut self, pos: SeekFrom) -> Result<u64, IoError> {
        if let Some(buffer) = &mut self.buffer {
            return buffer.seek(pos);
        } else {
            return self.file.seek(pos);
        }
    }

    pub(crate) fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), IoError> {
        if let Some(buffer) = &mut self.buffer {
            return buffer.read_exact(buf);
        } else {
            return self.file.read_exact(buf);
        }
    }

    pub(crate) fn read_exact_at<const S: usize>(
        &mut self,
        offset: u64,
    ) -> Result<[u8; S], IoError> {
        let mut buffer = [0u8; S];
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(&mut buffer)?;

        return Ok(buffer);
    }

    pub(crate) fn read_vec_at(&mut self, offset: u64, buf_size: usize) -> Result<Vec<u8>, IoError> {
        let mut buffer = vec![0u8; buf_size];
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(&mut buffer)?;

        return Ok(buffer);
    }

    pub(crate) fn map_from_file<T: FromBytes>(&mut self, offset: u64) -> Result<T, IoError> {
        let mut buffer = vec![0u8; core::mem::size_of::<T>()];
        self.seek(SeekFrom::Start(offset))?;
        self.read_exact(&mut buffer)?;

        let data = T::read_from_bytes(&buffer).map_err(|_| IoErrorKind::UnexpectedEof)?;

        return Ok(data);
    }

    pub(crate) fn read_sector_at(&mut self, sector: u64) -> Result<[u8; 512], IoError> {
        return self.read_exact_at::<512>(sector << 9);
    }

    /// Look up and validate a block magic.
    ///
    /// Seeks to each offset in [`BlockidIdinfo::magics`], reads up to [`BlockidMagic::len`] bytes
    /// of each magic and compares against the expected pattern.
    ///
    /// # Returns
    /// - `Ok(Some(BlockidMagic))` if a match is found.
    /// - `Ok(None)` if no magics are defined.
    /// - `Err(IoError)` if I/O fails or no match is found.
    ///
    /// # Panics
    /// - Each [`BlockidMagic`] must have [`BlockidMagic::len`] `<= 16`.
    pub(crate) fn get_magic(
        &mut self,
        id_info: &BlockidIdinfo,
    ) -> Result<Option<BlockidMagic>, IoError> {
        /*
         * This avoids allocating a buffer on the stack everytime and or
         * doing a heap allocation for each magic.
         */
        let mut buffer = [0u8; 16];
        match id_info.magics {
            Some(magics) => {
                for magic in magics {
                    self.seek(SeekFrom::Start(magic.b_offset))?;

                    assert!(magic.len <= 16);

                    self.read_exact(&mut buffer[..magic.len])?;

                    if &buffer[..magic.len] == magic.magic {
                        return Ok(Some(*magic));
                    }
                }
            }
            None => return Ok(None),
        }

        return Err(IoErrorKind::NotFound.into());
    }

    /// Run all detection probes and populate the [`Probe`] with the first
    /// successful result.
    ///
    /// Probes are executed in the order defined by the static [`PROBES`](crate::probe::PROBES) table,
    /// unless restricted by the probe’s [`ProbeFilter`].
    ///
    /// # Errors
    /// Returns [`BlockidError::ProbesExhausted`] if no supported container,
    /// partition table, or filesystem could be identified.
    ///
    /// # Panics
    /// Will panic if more then 1 result is found. In normal cases this should never happen
    /// if probing logic is correct and sane.
    pub fn probe_values(&mut self) -> Result<(), BlockidError> {
        if self.filter.is_empty() {
            for info in PROBES {
                let result = match self.get_magic(&info.2) {
                    Ok(magic) => match magic {
                        Some(t) => {
                            log::debug!(
                                "probe_values - BLOCKIDMAGIC: Correct Magic\nInfo: \"{:?}\"\n",
                                info.2
                            );
                            (info.2.probe_fn)(self, t)
                        }
                        None => {
                            log::debug!(
                                "probe_values - BLOCKIDMAGIC: Empty Magic\nInfo: \"{:?}\"\n",
                                info.2
                            );
                            (info.2.probe_fn)(self, BlockidMagic::EMPTY_MAGIC)
                        }
                    },
                    Err(e) => {
                        log::error!(
                            "probe_values - BLOCKIDMAGIC: Wrong Magic\nInfo: \"{:?}\",\nError: {:?}\n",
                            info.2,
                            e
                        );
                        continue;
                    }
                };

                if result.is_ok() {
                    return Ok(());
                }
            }
            return Err(BlockidError::ProbesExhausted);
        }

        let filtered_probe: Vec<BlockidIdinfo> = PROBES
            .iter()
            .filter_map(|&(catagory, item, id_info)| {
                if !self.filter.contains(catagory) && !self.filter.contains(item) {
                    return Some(id_info);
                } else {
                    return None;
                }
            })
            .collect();

        for info in filtered_probe {
            let result = match self.get_magic(&info) {
                Ok(magic) => match magic {
                    Some(t) => (info.probe_fn)(self, t),
                    None => (info.probe_fn)(self, BlockidMagic::EMPTY_MAGIC),
                },
                Err(_) => continue,
            };

            if result.is_ok() {
                return Ok(());
            }
        }

        return Err(BlockidError::ProbesExhausted);
    }

    pub(crate) fn push_result(&mut self, result: ProbeResult) {
        if self.value.is_some() {
            log::error!(
                "Probe already has a result, first: {:?}, second: {result:?}",
                self.value
            );
            // If a probe has multiple results there is a serious issue with the probing logic
            panic!("Probe already has a result");
        }
        self.value = Some(result)
    }

    /// Create a probe from a file path.
    pub fn from_filename(
        filename: &Path,
        flags: ProbeFlags,
        filter: ProbeFilter,
        offset: u64,
    ) -> Result<Probe, BlockidError> {
        let file = File::open(filename)?;

        let probe = Probe::new(file, filename, offset, flags, filter)?;

        return Ok(probe);
    }

    #[allow(dead_code)]
    pub(crate) fn inner_result(&self) -> Option<&ProbeResult> {
        self.value.as_ref()
    }

    /// Returns the result of the probe, if any.
    ///
    /// The returned value is a reference to [`ProbeResult`].  
    /// You can inspect it manually by matching on variants:
    /// - [`ProbeResult::Container`](ContainerResultView)
    /// - [`ProbeResult::PartTable`](PartTableResultView)
    /// - [`ProbeResult::Filesystem`](FilesystemResultView)
    ///
    /// # Convenience helpers
    /// Instead of matching manually, you can use the provided helper methods:
    /// - [`Probe::as_container()`] → returns [`ContainerResultView`] if the result is a container.
    /// - [`Probe::as_part_table()`] → returns [`PartTableResultView`] if the result is a partition table.
    /// - [`Probe::as_filesystem()`] → returns [`FilesystemResultView`] if the result is a filesystem.
    pub fn result(&self) -> Option<&ProbeResult> {
        self.value.as_ref()
    }

    /// Returns a [`ContainerResultView`] if the probe detected a container.
    pub fn as_container(&self) -> Option<ContainerResultView<'_>> {
        match self.result() {
            Some(ProbeResult::Container(c)) => Some(ContainerResultView { inner: c }),
            _ => None,
        }
    }

    /// Returns a [`PartTableResultView`] if the probe detected a partition table.
    pub fn as_part_table(&self) -> Option<PartTableResultView<'_>> {
        match self.result() {
            Some(ProbeResult::PartTable(p)) => Some(PartTableResultView { inner: p }),
            _ => None,
        }
    }

    /// Returns a [`FilesystemResultView`] if the probe detected a filesystem.
    pub fn as_filesystem(&self) -> Option<FilesystemResultView<'_>> {
        match self.result() {
            Some(ProbeResult::Filesystem(f)) => Some(FilesystemResultView { inner: f }),
            _ => None,
        }
    }

    /// Returns the path of the probed file or device as a [`Path`].
    #[inline]
    pub fn path(&self) -> &Path {
        return self.path.as_path();
    }

    /// Returns the total size in bytes of the probed file or device.
    #[inline]
    pub fn size(&self) -> u64 {
        return self.size;
    }

    /// Returns the starting offset in bytes used for this probe.
    #[inline]
    pub fn offset(&self) -> u64 {
        return self.offset;
    }

    /// Returns the logical sector size in bytes of the device.
    #[inline]
    pub fn ssz(&self) -> u64 {
        return self.sector_size;
    }

    #[cfg(target_os = "linux")]
    /// Returns the zone size in bytes of the block device (Linux only).
    ///
    /// `None` if the probe is not on a block device or the zone size could not be
    /// determined.
    #[inline]
    pub fn zsz(&self) -> Option<u64> {
        return self.zone_size;
    }

    /// Returns the device number of the probed file.
    #[inline]
    pub fn devno(&self) -> Dev {
        return self.devno;
    }

    /// Returns the major number of the probed device.
    #[inline]
    pub fn devno_maj(&self) -> u32 {
        return major(self.devno);
    }

    /// Returns the minor number of the probed device.
    #[inline]
    pub fn devno_min(&self) -> u32 {
        return minor(self.devno);
    }

    /// Returns the device number of the disk containing the probed file.
    #[inline]
    pub fn disk_devno(&self) -> Dev {
        return self.disk_devno;
    }

    /// Returns the major number of the disk containing the probed file.
    #[inline]
    pub fn disk_devno_maj(&self) -> u32 {
        return major(self.disk_devno);
    }

    /// Returns the minor number of the disk containing the probed file.
    #[inline]
    pub fn disk_devno_min(&self) -> u32 {
        return minor(self.disk_devno);
    }

    /// Returns if the probed file is a block device.
    #[inline]
    pub fn is_block_device(&self) -> bool {
        return FileType::from_raw_mode(self.mode.as_raw_mode()).is_block_device();
    }

    /// Returns if the probed file is a regular file.
    #[inline]
    pub fn is_regular_file(&self) -> bool {
        return FileType::from_raw_mode(self.mode.as_raw_mode()).is_file();
    }

    /// On Linux only:
    /// - queries OPAL device status via ioctl (if not already checked).
    /// - sets `ProbeFlags::OPAL_CHECKED` and conditionally `OPAL_LOCKED`.
    /// - returns whether the device is currently OPAL locked.
    ///
    /// When building on non-Linux platforms opal locked check is skipped and a warning is logged
    #[cfg(target_os = "linux")]
    pub(crate) fn is_opal_locked(&mut self) -> Result<bool, rustix::io::Errno> {
        if !self.flags.contains(ProbeFlags::OPAL_CHECKED) {
            let status = ioctl_ioc_opal_get_status(self.file.as_fd())?;

            if status.flags.contains(OpalStatusFlags::OPAL_FL_LOCKED) {
                self.flags.insert(ProbeFlags::OPAL_LOCKED);
            }

            self.flags.insert(ProbeFlags::OPAL_CHECKED);
        }

        return Ok(self.flags.contains(ProbeFlags::OPAL_LOCKED));
    }

    /// Returns current Probe filters.
    pub fn filters(&self) -> ProbeFilter {
        self.filter
    }

    /// Returns current Probe flags.
    pub fn flags(&self) -> ProbeFlags {
        self.flags
    }

    /// Returns [`File`] being probed.
    pub fn file(&mut self) -> &File {
        &self.file
    }
}

bitflags! {
    /// Flags controlling the behavior of a [`Probe`].
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ProbeFlags: u64 {
        /// Indicates the device is small and may require special handling.
        const TINY_DEV = 1 << 0;
        /// Marks that the OPAL status has been checked.
        const OPAL_CHECKED = 1 << 1;
        /// Marks that the device is OPAL locked.
        const OPAL_LOCKED = 1 << 2;
        /// Forces GPT detection even if a protective MBR is present.
        const FORCE_GPT_PMBR = 1 << 3;
    }

    /// Filters used to skip specific probe categories or items.
    ///
    /// Can be combined to restrict probing to certain types of containers,
    /// partition tables, or filesystems.
    #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct ProbeFilter: u64 {
        /// Skip container probes.
        const SKIP_CONT = 1 << 0;
        /// Skip partition table probes.
        const SKIP_PT = 1 << 1;
        /// Skip filesystem probes.
        const SKIP_FS = 1 << 2;
        /// Skip LUKS1 container probe.
        const SKIP_LUKS1 = 1 << 3;
        /// Skip LUKS2 container probe.
        const SKIP_LUKS2 = 1 << 4;
        /// Skip LUKS OPAL container probe.
        const SKIP_LUKS_OPAL = 1 << 5;
        /// Skip DOS partition table probe.
        const SKIP_DOS = 1 << 6;
        /// Skip GPT partition table probe.
        const SKIP_GPT = 1 << 7;
        /// Skip exFAT filesystem probe.
        const SKIP_EXFAT = 1 << 8;
        /// Skip JBD filesystem probe.
        const SKIP_JBD = 1 << 9;
        /// Skip EXT2 filesystem probe.
        const SKIP_EXT2 = 1 << 10;
        /// Skip EXT3 filesystem probe.
        const SKIP_EXT3 = 1 << 11;
        /// Skip EXT4 filesystem probe.
        const SKIP_EXT4 = 1 << 12;
        /// Skip Linux Swap version 0 probe.
        const SKIP_LINUX_SWAP_V0 = 1 << 13;
        /// Skip Linux Swap version 1 probe.
        const SKIP_LINUX_SWAP_V1 = 1 << 14;
        /// Skip hibernation/swsuspend probe.
        const SKIP_SWSUSPEND = 1 << 15;
        /// Skip NTFS filesystem probe.
        const SKIP_NTFS = 1 << 16;
        /// Skip VFAT filesystem probe.
        const SKIP_VFAT = 1 << 17;
        /// Skip XFS filesystem probe.
        const SKIP_XFS = 1 << 18;
        /// Skip APFS filesystem probe.
        const SKIP_APFS = 1 << 19;
        /// Skip SQUASHFS3 filesystem probe.
        const SKIP_SQUASHFS3 = 1 << 20;
        /// Skip SQUASHFS filesystem probe.
        const SKIP_SQUASHFS = 1 << 21;
    }
}

/// Represents the result of a [`Probe`].
///
/// A probe may detect one of the following types:
/// - [`ContainerResult`]: a container or encrypted volume (e.g., LUKS).
/// - [`PartTableResult`]: a partition table (e.g., DOS, GPT).
/// - [`FilesystemResult`]: a filesystem (e.g., EXT4, NTFS, XFS).
///
/// Use the helper methods on [`Probe`] to access each variant:
/// - [`Probe::as_container()`]
/// - [`Probe::as_part_table()`]
/// - [`Probe::as_filesystem()`]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ProbeResult {
    /// Container results.
    Container(ContainerResult),
    /// Partition table results.
    PartTable(PartTableResult),
    /// Filesystem results.
    Filesystem(FilesystemResult),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ContainerResult {
    pub btype: Option<BlockType>,
    pub sec_type: Option<SecType>,
    pub uuid: Option<BlockidUUID>,
    pub label: Option<String>,
    pub creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub endianness: Option<Endianness>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PartTableResult {
    pub btype: Option<BlockType>,
    pub sec_type: Option<SecType>,
    pub uuid: Option<BlockidUUID>,
    pub creator: Option<String>,
    pub usage: Option<UsageType>,
    pub version: Option<BlockidVersion>,
    pub partitions: Option<Vec<PartitionResults>>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub endianness: Option<Endianness>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PartitionResults {
    pub offset: Option<u64>,
    pub size: Option<u64>,
    pub partno: Option<u64>,
    pub part_uuid: Option<BlockidUUID>,
    pub name: Option<String>,
    pub entry_type: Option<PartEntryType>,
    pub entry_attributes: Option<PartEntryAttributes>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FilesystemResult {
    pub btype: Option<BlockType>,
    pub sec_type: Option<SecType>,
    pub uuid: Option<BlockidUUID>,
    pub log_uuid: Option<BlockidUUID>,
    pub ext_journal: Option<BlockidUUID>,
    pub label: Option<String>,
    pub creator: Option<String>,
    pub usage: Option<UsageType>,
    pub size: Option<u64>,
    pub fs_last_block: Option<u64>,
    pub fs_block_size: Option<u64>,
    pub block_size: Option<u64>,
    pub version: Option<BlockidVersion>,
    pub sbmagic: Option<&'static [u8]>,
    pub sbmagic_offset: Option<u64>,
    pub endianness: Option<Endianness>,
}

/// Container results returned by a [`Probe::as_container`].
///
/// Provides access to container metadata.
#[derive(Debug)]
pub struct ContainerResultView<'a> {
    inner: &'a ContainerResult,
}

impl<'a> ContainerResultView<'a> {
    /// Returns the container type.
    pub fn block_type(&self) -> Option<BlockType> {
        self.inner.btype
    }
    /// Returns the sector type.
    pub fn sec_type(&self) -> Option<SecType> {
        self.inner.sec_type
    }
    /// Returns the UUID of the container.
    pub fn uuid(&self) -> Option<BlockidUUID> {
        self.inner.uuid
    }
    /// Returns the label of the container.
    pub fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }
    /// Returns the creator identifier.
    pub fn creator(&self) -> Option<&str> {
        self.inner.creator.as_deref()
    }
    /// Returns the usage type of the container.
    pub fn usage(&self) -> Option<UsageType> {
        self.inner.usage
    }
    /// Returns the version of the container, if known.
    pub fn version(&self) -> Option<BlockidVersion> {
        self.inner.version
    }
    /// Returns the detected superblock magic bytes.
    pub fn sbmagic(&self) -> Option<&'static [u8]> {
        self.inner.sbmagic
    }
    /// Returns the offset of the superblock magic.
    pub fn sbmagic_offset(&self) -> Option<u64> {
        self.inner.sbmagic_offset
    }
    /// Returns the endianness of the container, if applicable.
    pub fn endianness(&self) -> Option<Endianness> {
        self.inner.endianness
    }
}

/// Partition Table results returned by a [`Probe::as_part_table`].
///
/// Provides access to partition table metadata and partition entries.
#[derive(Debug)]
pub struct PartTableResultView<'a> {
    inner: &'a PartTableResult,
}

impl<'a> PartTableResultView<'a> {
    /// Returns the container type.
    pub fn block_type(&self) -> Option<BlockType> {
        self.inner.btype
    }
    /// Returns the sector type.
    pub fn sec_type(&self) -> Option<SecType> {
        self.inner.sec_type
    }
    /// Returns the UUID of the container.
    pub fn uuid(&self) -> Option<BlockidUUID> {
        self.inner.uuid
    }
    /// Returns list of partitions.
    pub fn partitions(&self) -> impl Iterator<Item = &PartitionResults> {
        self.inner.partitions.as_deref().into_iter().flatten()
    }
    /// Returns the detected superblock magic bytes.
    pub fn sbmagic(&self) -> Option<&'static [u8]> {
        self.inner.sbmagic
    }
    /// Returns the offset of the superblock magic.
    pub fn sbmagic_offset(&self) -> Option<u64> {
        self.inner.sbmagic_offset
    }
    /// Returns the endianness of the container, if applicable.
    pub fn endianness(&self) -> Option<Endianness> {
        self.inner.endianness
    }
}

/// Filesystem results returned by a [`Probe::as_filesystem`].
///
/// Provides access to filesystem metadata.
#[derive(Debug)]
pub struct FilesystemResultView<'a> {
    inner: &'a FilesystemResult,
}

impl<'a> FilesystemResultView<'a> {
    /// Returns the container type.
    pub fn block_type(&self) -> Option<BlockType> {
        self.inner.btype
    }
    /// Returns the sector type.
    pub fn sec_type(&self) -> Option<SecType> {
        self.inner.sec_type
    }
    /// Returns the UUID of the filesystem.
    pub fn uuid(&self) -> Option<BlockidUUID> {
        self.inner.uuid
    }
    /// Returns the log UUID of the filesystem.
    pub fn log_uuid(&self) -> Option<BlockidUUID> {
        self.inner.log_uuid
    }
    /// Returns the external journal UUID of the filesystem.
    pub fn ext_journal(&self) -> Option<BlockidUUID> {
        self.inner.ext_journal
    }
    /// Returns the label of the filesystem.
    pub fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }
    /// Returns the creator identifier.
    pub fn creator(&self) -> Option<&str> {
        self.inner.creator.as_deref()
    }
    /// Returns the usage type of the filesystem.
    pub fn usage(&self) -> Option<UsageType> {
        self.inner.usage
    }
    /// Returns size in bytes of filesystem.
    pub fn size(&self) -> Option<u64> {
        self.inner.size
    }
    /// Returns last block of filesystem.
    pub fn last_block(&self) -> Option<u64> {
        self.inner.fs_last_block
    }
    /// Returns filesystem of block size.
    pub fn fs_block_size(&self) -> Option<u64> {
        self.inner.fs_block_size
    }
    /// Returns block size in bytes of filesystem.
    pub fn block_size(&self) -> Option<u64> {
        self.inner.block_size
    }
    /// Returns the version of the filesystem, if known.
    pub fn version(&self) -> Option<BlockidVersion> {
        self.inner.version
    }
    /// Returns the detected superblock magic bytes.
    pub fn sbmagic(&self) -> Option<&'static [u8]> {
        self.inner.sbmagic
    }
    /// Returns the offset of the superblock magic.
    pub fn sbmagic_offset(&self) -> Option<u64> {
        self.inner.sbmagic_offset
    }
    /// Returns the endianness of the filesystem, if applicable.
    pub fn endianness(&self) -> Option<Endianness> {
        self.inner.endianness
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartEntryType {
    Byte(u8),
    Uuid(Uuid),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PartEntryAttributes {
    Mbr(u8),
    Gpt(u64),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockType {
    LUKS1,
    LUKS2,
    LUKSOpal,
    Dos,
    Gpt,
    Exfat,
    Jbd,
    Apfs,
    Ext2,
    Ext3,
    Ext4,
    LinuxSwapV0,
    LinuxSwapV1,
    SwapSuspend,
    Ntfs,
    Vfat,
    Xfs,
    Squashfs,
    Squashfs3,
}

impl fmt::Display for BlockType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LUKS1 => write!(f, "LUKS1"),
            Self::LUKS2 => write!(f, "LUKS2"),
            Self::LUKSOpal => write!(f, "LUKS Opal"),
            Self::Dos => write!(f, "DOS"),
            Self::Gpt => write!(f, "GPT"),
            Self::Exfat => write!(f, "EXFAT"),
            Self::Jbd => write!(f, "JBD"),
            Self::Apfs => write!(f, "APFS"),
            Self::Ext2 => write!(f, "EXT2"),
            Self::Ext3 => write!(f, "EXT3"),
            Self::Ext4 => write!(f, "EXT4"),
            Self::Ntfs => write!(f, "NTFS"),
            Self::LinuxSwapV0 => write!(f, "Linux Swap V0"),
            Self::LinuxSwapV1 => write!(f, "Linux Swap V1"),
            Self::SwapSuspend => write!(f, "Swap Suspend"),
            Self::Vfat => write!(f, "VFAT"),
            Self::Xfs => write!(f, "XFS"),
            Self::Squashfs => write!(f, "SquashFS"),
            Self::Squashfs3 => write!(f, "SquashFS3"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SecType {
    Fat12,
    Fat16,
    Fat32,
}

impl fmt::Display for SecType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fat12 => write!(f, "Fat12"),
            Self::Fat16 => write!(f, "Fat16"),
            Self::Fat32 => write!(f, "Fat32"),
        }
    }
}

/// Unique identifier for a block.
///
/// # Variants
/// - `Uuid(Uuid)` - Uses a standard [`Uuid`] as the identifier.
/// - `VolumeId32(VolumeId32)` - Uses a 32-bit volume ID as the identifier.
/// - `VolumeId64(VolumeId64)` - Uses a 64-bit volume ID as the identifier.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockidUUID {
    /// Standard [`Uuid`] identifier.
    Uuid(Uuid),
    /// 32-bit volume identifier.
    VolumeId32(VolumeId32),
    /// 64-bit volume identifier.
    VolumeId64(VolumeId64),
}

impl From<Uuid> for BlockidUUID {
    fn from(value: Uuid) -> Self {
        BlockidUUID::Uuid(value)
    }
}

impl From<VolumeId32> for BlockidUUID {
    fn from(value: VolumeId32) -> Self {
        BlockidUUID::VolumeId32(value)
    }
}

impl From<VolumeId64> for BlockidUUID {
    fn from(value: VolumeId64) -> Self {
        BlockidUUID::VolumeId64(value)
    }
}

impl fmt::Display for BlockidUUID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uuid(t) => write!(f, "{t}"),
            Self::VolumeId32(t) => write!(f, "{t}"),
            Self::VolumeId64(t) => write!(f, "{t}"),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash)]
pub struct BlockidIdinfo {
    pub name: Option<&'static str>,
    pub btype: Option<BlockType>,
    pub usage: Option<UsageType>,
    pub minsz: Option<u64>,
    pub probe_fn: ProbeFn,
    pub magics: Option<&'static [BlockidMagic]>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum UsageType {
    Filesystem,
    PartitionTable,
    Raid,
    Crypto,
    Other(&'static str),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BlockidVersion {
    Number(u64),
    DevT(Dev),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Endianness {
    Little,
    Big,
}

type ProbeFn = fn(&mut Probe, BlockidMagic) -> Result<(), BlockidError>;

/// Represents a magic identifier for a block.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BlockidMagic {
    /// The magic value as a byte slice.
    pub magic: &'static [u8],
    /// The length of the block or data segment.
    pub len: usize,
    /// Offset within the block for the magic value.
    pub b_offset: u64,
}

impl BlockidMagic {
    /// An empty [`BlockidMagic`] with zero length, zero offset, and a zero byte magic.
    pub const EMPTY_MAGIC: BlockidMagic = BlockidMagic {
        magic: &[0],
        len: 0,
        b_offset: 0,
    };
}
