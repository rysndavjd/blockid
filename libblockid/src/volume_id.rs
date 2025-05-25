use std::fmt::{self};
use byteorder::{BigEndian, ByteOrder, LittleEndian};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeId32(Option<[u8; 4]>);

/* Will do 64 bit version when i implement exfat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VolumeId64(Option<u64>);
*/

impl VolumeId32 {
    pub fn empty() -> Self {
        VolumeId32(None)
    }

    pub fn new(value: [u8; 4]) -> Self {
        VolumeId32(Some(value))
    }

    pub fn from_u32_le(value: u32) -> VolumeId32 {
        VolumeId32(Some(value.to_le_bytes()))
    }
    
    pub fn from_u32_be(value: u32) -> VolumeId32 {
        VolumeId32(Some(value.to_be_bytes()))
    }

}

/* Will do 64 bit version when i implement exfat
impl VolumeId64 {
    pub fn empty() -> Self {
        VolumeId64(None)
    }

    pub fn new(value: u64) -> Self {
        VolumeId64(Some(value))
    }

}
*/

impl fmt::Display for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02X}{:02X}-{:02X}{:02X}", bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}
