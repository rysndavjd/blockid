`libblockid_sys` extends `libblockid_core` with topology and system-specific 
block device information. It requires a supported kernel and a memory 
allocator, supporting both `std` and `no_std` environments on Linux, macOS, 
and FreeBSD.