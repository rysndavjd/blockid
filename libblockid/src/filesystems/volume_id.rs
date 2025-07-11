use core::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VolumeId32([u8; 4]);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VolumeId64([u8; 8]);

impl VolumeId32 {
    pub fn nil() -> Self {
        VolumeId32([0u8; 4])
    }

    pub fn max() -> Self {
        VolumeId32([0xFFu8; 4])
    }

    pub fn new(value: [u8; 4]) -> Self {
        VolumeId32(value)
    }

    pub fn from_u32_le(value: u32) -> VolumeId32 {
        VolumeId32(value.to_le_bytes())
    }
    
    pub fn from_u32_be(value: u32) -> VolumeId32 {
        VolumeId32(value.to_be_bytes())
    }

}

impl VolumeId64 {
    pub fn nil() -> Self {
        VolumeId64([0u8; 8])
    }

    pub fn max() -> Self {
        VolumeId64([0xFFu8; 8])
    }

    pub fn new(value: [u8; 8]) -> Self {
        VolumeId64(value)
    }

    pub fn from_u64_le(value: u64) -> VolumeId64 {
        VolumeId64(value.to_le_bytes())
    }
    
    pub fn from_u64_be(value: u64) -> VolumeId64 {
        VolumeId64(value.to_be_bytes())
    }
}

impl fmt::Display for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}-{:02X}{:02X}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Display for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}", self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::UpperHex for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}{:02X}{:02X}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::UpperHex for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}", self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::LowerHex for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}{:02x}{:02x}{:02x}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::LowerHex for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}", self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Octal for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02o}{:02o}-{:02o}{:02o}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Octal for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02o}{:02o}{:02o}{:02o}{:02o}{:02o}{:02o}{:02o}", self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Binary for VolumeId32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02b}{:02b}{:02b}{:02b}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Binary for VolumeId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02b}{:02b}{:02b}{:02b}{:02b}{:02b}{:02b}{:02b}", self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0])
    }
}