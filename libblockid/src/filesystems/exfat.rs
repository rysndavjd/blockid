use bytemuck::{Pod, Zeroable};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RawExFatBs {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
    pub reserved_0: [u8; 53],
    pub partition_offset: u64,
    pub volume_length: u64,
    pub fat_offset: u32,
    pub fat_length: u32,
    pub clustor_heap_offset: u32,
    pub clustor_count: u32,
    pub first_clustor_of_root: u32,
    pub volume_serial: u32,
    pub revision: u16,
    pub volume_flags: u16,
    pub bytes_per_sector_shift: u8,
    pub sectors_per_cluster_shift: u8,
    pub number_of_fats: u8,
    pub drive_select: u8,
    pub percent_in_use: u8,
    pub reserved_1: [u8; 7],
    pub executable_code: [u8; 390],
    pub boot_flag: u16,
}


#[derive(Debug, Clone)]
pub struct ExFatHeader {
    pub bootjmp: [u8; 3],
    pub oem_name: String,
    pub reserved_0: [u8; 53],
    pub partition_offset: u64,
    pub volume_length: u64,
    pub fat_offset: u32,
    pub fat_length: u32,
    pub clustor_heap_offset: u32,
    pub clustor_count: u32,
    pub first_clustor_of_root: u32,
    pub volume_serial: u32,
    pub revision: u16,
    pub volume_flags: u16,
    pub bytes_per_sector_shift: u8,
    pub sectors_per_cluster_shift: u8,
    pub number_of_fats: u8,
    pub drive_select: u8,
    pub percent_in_use: u8,
    pub reserved_1: [u8; 7],
    pub executable_code: [u8; 390],
    pub boot_flag: u16,
    pub volume_label: String,
}