# Blockid

**Blockid** is a tool for identifying various types of superblocks, including filesystems and partition tables.  
> **Note:** `blockid` is currently a test method for `libblockid` and is **not usable as a replacement for `blkid`** currently.

This project is a Rust implementation of `libblkid` and the `blkid` utility from [util-linux](https://github.com/util-linux/util-linux/). It is **highly experimental** and incomplete.

---

# Libblockid

[![Latest version](https://img.shields.io/crates/v/libblockid.svg)](https://crates.io/crates/libblockid)
[![Documentation](https://docs.rs/libblockid/badge.svg)](https://docs.rs/libblockid)
![License](https://img.shields.io/crates/l/libblockid.svg)

`libblockid` is the core library for superblock detection. Below is a list of supported and inprogress block types.

> ⚠️ **Note:** `libblockid` is under active development. Its API is unstable and may change without notice.

| Block Type | Status  | Category         |
|------------|---------|-----------------|
| APFS       | Works   | Filesystem      |
| Btrfs      | Todo    | Filesystem      |
| BSD        | Todo    | Partition Table |
| DOS        | Works   | Partition Table |
| GPT        | Broke   | Partition Table |
| Mac        | Todo    | Partition Table |
| ExFAT      | Works   | Filesystem      |
| Ext2/3/4   | Works   | Filesystem      |
| LUKS       | Works   | Container       |
| NTFS       | Works   | Filesystem      |
| VFat       | Works   | Filesystem      |
| XFS        | Works   | Filesystem      |
| ZFS        | Todo    | Filesystem      |
| Swap       | Works   | Filesystem      |

### Status Definitions

- **Broke**: Implementation exists but is currently broken.  
- **Todo**: Planned, not yet started.  
- **Works**: Functionally implemented; identifies and parses most metadata.  
- **Complete**: Fully implemented, including parsing and handling edge cases.

---

# Supported OS 

`libblockid` and `blockid` are planned to support Linux, FreeBSD, and macOS. While compiling `libblockid` from Git should work on all three platforms, some functionality may be broken on FreeBSD and macOS, as development primarily occurs on Linux. 
# Architecture
`libblockid` and `blockid` mainly supports [x86_64](https://en.wikipedia.org/wiki/X86-64) with planned support for [AArch64](https://en.wikipedia.org/wiki/AArch64). Things like hardware acceleration may break in crates like [sha2](https://docs.rs/sha2/latest/sha2/), [crc-fast](https://docs.rs/crc-fast/latest/crc_fast/) and may need to be disabled when compiling to other architectures. 

> ⚠️ **Note:** `libblockid` does not aim to support 32-bit architectures, as the effort required outweighs the benefits. Users needing 32-bit support should use [libblkid](https://github.com/util-linux/util-linux/tree/master/libblkid) with Rust FFI instead.


## License

This project is dual-licensed under either:

- [MIT License](LICENSE-MIT), or
- [Apache License 2.0](LICENSE-APACHE)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions. 
