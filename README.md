Blockid
=======

**Blockid** is a tool for identifying various types of superblocks like filesystems and partition tables.
> *NOTE:* `blockid` currently is just a test method for `libblockid` and is unusable for how blkid would be used.

This project is a copy of `libblkid` and the `blkid` utility from [util-linux](https://github.com/util-linux/util-linux/) but written in Rust. It's currently HIGHLY experimental and incomplete.

Libblockid
======
[![Latest version](https://img.shields.io/crates/v/libblockid.svg)](https://crates.io/crates/libblockid)
[![Documentation](https://docs.rs/libblockid/badge.svg)](https://docs.rs/libblockid)
![License](https://img.shields.io/crates/l/libblockid.svg)

`libblockid` serves as the core library for superblock detection. Below is a list of supported and inprogress block types.

> ⚠️ **NOTE:** `libblockid` is under Major development. Its API is unstable and subject to change without notice.

| Block Type | Status | Category         |
|------------|--------|------------------|
| APFS       | Todo   | Filesystem       |
| Btrfs      | Todo   | Filesystem       |
| BSD        | Todo   | Partition Table  |
| DOS        | Works   | Partition Table  |
| GPT        | Next   | Partition Table  |
| Mac        | Todo   | Partition Table  |
| ExFAT      | Works  | Filesystem       |
| Ext2/3/4   | Works  | Filesystem       |
| LUKS       | Works   | Container        |
| NTFS       | Works   | Filesystem       |
| VFat       | Works  | Filesystem       |
| XFS        | Todo   | Filesystem       |
| ZFS        | Todo   | Filesystem       |
| Swap        | Works   | Filesystem       |

### Status

- **Todo**: Planned, but not yet started.
- **Next**: Currently being implemented.
- **Works**: Functionally implemented; successfully identifies and parses most metadata.
- **Complete**: Fully implemented with complete parsing and edge case handling.

## License

This project is dual-licensed under either:

- [MIT License](LICENSE-MIT), or
- [Apache License 2.0](LICENSE-APACHE)

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions. 
