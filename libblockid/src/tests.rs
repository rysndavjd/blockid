use std::{fs::File, path::PathBuf};
use uuid::Uuid;

use crate::filesystems::xfs::*;
use crate::probe::*;

#[test]
fn xfs_probe_test() {
    let xfs_path = PathBuf::from("./tests/xfs.bin");
    let mut probe = Probe::new(
        File::open(&xfs_path).unwrap(),
        &xfs_path,
        0,
        ProbeFlags::empty(),
        ProbeFilter::empty(),
    )
    .unwrap();

    probe_xfs(&mut probe, XFS_ID_INFO.magics.unwrap()[0]).unwrap();

    let r = probe.as_filesystem().unwrap();

    assert_eq!(r.block_type(), Some(BlockType::Xfs));
    assert_eq!(r.sec_type(), None);
    assert_eq!(
        r.uuid(),
        Some(BlockidUUID::Uuid(Uuid::from_bytes([
            0xd6, 0x5b, 0x25, 0x5e, 0xb2, 0x33, 0x43, 0x3c, 0x82, 0x22, 0xfa, 0x3c, 0xa6, 0x55,
            0xa4, 0xbf,
        ])))
    );
    assert_eq!(r.log_uuid(), None);
    assert_eq!(r.ext_journal(), None);
    assert_eq!(r.label(), Some("blockidXfs2"));
    assert_eq!(r.creator(), None);
    assert_eq!(r.usage(), Some(UsageType::Filesystem));
    assert_eq!(r.size(), Some(248512512));
    assert_eq!(r.last_block(), Some(77056));
    assert_eq!(r.fs_block_size(), Some(4096));
    assert_eq!(r.block_size(), Some(512));
    assert_eq!(r.version(), None);
    assert_eq!(r.sbmagic(), Some(XFS_ID_INFO.magics.unwrap()[0].magic));
    assert_eq!(
        r.sbmagic_offset(),
        Some(XFS_ID_INFO.magics.unwrap()[0].b_offset)
    );
    assert_eq!(r.endianness(), None);
}
