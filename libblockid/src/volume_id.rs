use std::fmt::{self};
//use byteorder::{BigEndian, ByteOrder, LittleEndian};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeId32(Option<[u8; 4]>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeId64(Option<[u8; 8]>);

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

impl VolumeId64 {
    pub fn empty() -> Self {
        VolumeId64(None)
    }

    pub fn new(value: [u8; 8]) -> Self {
        VolumeId64(Some(value))
    }

    pub fn from_u64_le(value: u64) -> VolumeId64 {
        VolumeId64(Some(value.to_le_bytes()))
    }
    
    pub fn from_u64_be(value: u64) -> VolumeId64 {
        VolumeId64(Some(value.to_be_bytes()))
    }
}

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

impl fmt::Display for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}", bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::UpperHex for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02X}{:02X}{:02X}{:02X}", bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::UpperHex for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}", bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::LowerHex for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02x}{:02x}{:02x}{:02x}", bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::LowerHex for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::Octal for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02o}{:02o}-{:02o}{:02o}", bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::Octal for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02o}{:02o}{:02o}{:02o}{:02o}{:02o}{:02o}{:02o}", bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::Binary for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02b}{:02b}{:02b}{:02b}", bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}

impl fmt::Binary for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(bytes) => {
                write!(f, "{:02b}{:02b}{:02b}{:02b}{:02b}{:02b}{:02b}{:02b}", bytes[7], bytes[6], bytes[5], bytes[4], bytes[3], bytes[2], bytes[1], bytes[0])
            },
            None => write!(f, "<empty>"),
        }
    }
}