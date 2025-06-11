pub mod dos;
pub mod gpt;
pub mod mac;
pub mod bsd;
pub mod aix;
pub mod solaris_x86;
pub mod unixware;
pub mod minix;

use crate::BlockidUUID;
use uuid::Uuid;
use std::rc::{Rc, Weak};
use std::cell::RefCell;

/*
  PTTYPE:               partition table type (dos, gpt, etc.).
  PTUUID:               partition table id (uuid for gpt, hex for dos).
  PART_ENTRY_SCHEME:    partition table type
  PART_ENTRY_NAME:      partition name (gpt and mac only)
  PART_ENTRY_UUID:      partition UUID (gpt, or pseudo IDs for MBR)
  PART_ENTRY_TYPE:      partition type, 0xNN (e.g. 0x82) or type UUID (gpt only) or type string (mac)
  PART_ENTRY_FLAGS:     partition flags (e.g. boot_ind) or  attributes (e.g. gpt attributes)
  PART_ENTRY_NUMBER:    partition number
  PART_ENTRY_OFFSET:    the begin of the partition
  PART_ENTRY_SIZE:      size of the partition
  PART_ENTRY_DISK:      whole-disk maj:min
*/

#[derive(Debug)]
pub struct BlockidPartTable {
    pub pttype: PTType,
    pub offset: u64,
    pub id: BlockidUUID,

    pub num_parts: usize,
    pub parent: Option<Weak<RefCell<BlockidPartition>>>,

    pub partitions: Vec<Rc<RefCell<BlockidPartition>>>,

    pub next: Option<Rc<RefCell<BlockidPartTable>>>,
    pub prev: Option<Weak<RefCell<BlockidPartTable>>>,
}

#[derive(Debug)]
struct BlockidPartition {
    start: u64,
    size: u64,
    
    pttype: PTType,
    pttype_str: Option<Uuid>,

    flags: Option<u64>,

    partno: usize,
    uuid: Option<Uuid>,
    name: String,

    table: BlockidPartTable,
}

#[derive(Debug)]
pub struct BlockidPartList {
    pub next_partno: usize,
    pub next_parent: Option<Weak<RefCell<BlockidPartition>>>,

    pub partitions: Vec<Rc<RefCell<BlockidPartition>>>,

    pub head: Option<Rc<RefCell<BlockidPartTable>>>,
    pub tail: Option<Rc<RefCell<BlockidPartTable>>>,
    
}

impl BlockidPartList {
    pub fn empty() -> Self {
        BlockidPartList {
            next_partno: 0,
            next_parent: None,
            partitions: Vec::new(),
            head: None,
            tail: None,
        }
    }

    pub fn new(
            next_partno: usize, 
            next_parent: Option<Weak<RefCell<BlockidPartition>>>,
            partitions: Vec<Rc<RefCell<BlockidPartition>>>,
            head: Option<Rc<RefCell<BlockidPartTable>>>,
            tail: Option<Rc<RefCell<BlockidPartTable>>>,
        ) -> Self 
    {
        BlockidPartList {
            next_partno: next_partno,
            next_parent: next_parent,
            partitions: partitions,
            head: head,
            tail: tail,
        }
    }

    pub fn new_parttable(
            &mut self,
            pttype: PTType,
            offset: u64,
            id: BlockidUUID
        ) -> Rc<RefCell<BlockidPartTable>>
    {
        let tab = Rc::new(RefCell::new(BlockidPartTable {
            pttype: pttype,
            offset: offset,
            id: id,
            num_parts: 0,
            parent: self.next_parent.clone(),
            partitions: Vec::new(),
            next: None,
            prev: None,
        }));

        match self.tail.take() {
            Some(old_tail) => {
                old_tail.borrow_mut().next = Some(Rc::clone(&tab));
                tab.borrow_mut().prev = Some(Rc::downgrade(&old_tail));
                self.tail = Some(Rc::clone(&tab));
            }
            None => {
                self.head = Some(Rc::clone(&tab));
                self.tail = Some(Rc::clone(&tab));
            }
        }

        tab
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PTType {
    #[cfg(feature = "dos")]
    Dos,
    #[cfg(feature = "gpt")]
    Gpt,
    #[cfg(feature = "mac")]
    Mac,
    #[cfg(feature = "bsd")]
    Bsd,
    Unknown(String), 
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PTflags {
    #[cfg(feature = "dos")]
    Dos,
    #[cfg(feature = "gpt")]
    Gpt,
    #[cfg(feature = "mac")]
    Mac,
    #[cfg(feature = "bsd")]
    Bsd,
    Unknown(String), 
}
