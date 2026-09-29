use alloc::ffi::CString;

use core_foundation::{
    base::{CFGetTypeID, CFIndex, CFRelease, CFTypeRef, TCFType, mach_port_t},
    boolean::CFBooleanGetTypeID,
    dictionary::{CFDictionaryRef, CFMutableDictionaryRef},
    mach_port::CFAllocatorRef,
    number::{CFBooleanGetValue, CFBooleanRef},
    string::{CFString, CFStringGetCString, CFStringRef, kCFStringEncodingUTF8},
};

use crate::{
    error::PartToDiskError,
    io::{IoError, IoErrorKind, Path, PathBuf},
    std::{
        ffi::{c_char, c_int, c_uint},
        ptr::null,
    },
};

#[link(name = "IOKit")]
unsafe extern "C" {
    static kIOMainPortDefault: mach_port_t;
    fn IOObjectRelease(object: IoObject) -> KernReturn;
    fn IOIteratorNext(iterator: IoIterator) -> IoObject;
    fn IOServiceGetMatchingServices(
        mainPort: mach_port_t,
        matching: CFDictionaryRef,
        existing: *mut IoIterator,
    ) -> KernReturn;
    fn IORegistryEntryGetParentEntry(
        entry: IoRegistryEntry,
        plane: *const c_char,
        parent: *mut IoRegistryEntry,
    ) -> KernReturn;
    fn IOBSDNameMatching(
        masterPort: mach_port_t,
        options: u32,
        bsdName: *const c_char,
    ) -> CFMutableDictionaryRef;
    fn IORegistryEntryCreateCFProperty(
        entry: IoRegistryEntry,
        key: CFStringRef,
        allocator: CFAllocatorRef,
        options: IOOptionBits,
    ) -> CFTypeRef;
}

type KernReturn = c_int;
type IoObject = mach_port_t;
type IoIterator = IoObject;
type IoRegistryEntry = IoObject;
type IOOptionBits = c_uint;

const K_IO_SERVICE_PLANE: &str = "IOService\0";
const K_IO_MEDIA_WHOLE_KEY: &str = "Whole";
const K_BSD_NAME_KEY: &str = "BSD Name";

pub fn part_to_disk<P: AsRef<Path>>(path: P) -> Result<PathBuf, PartToDiskError<IoError>> {
    #[cfg(feature = "std")]
    let c_bsd = CString::new(
        path.as_ref()
            .file_name()
            .ok_or(IoError::from(IoErrorKind::InvalidInput))?
            .as_encoded_bytes(),
    )
    .map_err(|_| IoError::from(IoErrorKind::InvalidData))?;

    #[cfg(feature = "no_std")]
    let c_bsd = CString::new(
        path.as_ref()
            .file_name()
            .ok_or(IoError::from(IoErrorKind::InvalidInput))?,
    )
    .map_err(|_| IoError::from(IoErrorKind::InvalidData))?;

    let matching =
        unsafe { IOBSDNameMatching(kIOMainPortDefault, 0, c_bsd.as_ptr()) } as CFDictionaryRef;

    if matching.is_null() {
        return Err(PartToDiskError::BSDNameMatch);
    }

    let mut service = {
        let mut iter: IoIterator = 0;

        if unsafe { IOServiceGetMatchingServices(kIOMainPortDefault, matching, &mut iter) } != 0 {
            return Err(PartToDiskError::UnableToGetService);
        }

        let service = unsafe { IOIteratorNext(iter) };
        unsafe { IOObjectRelease(iter) };
        service
    };

    if service == 0 {
        return Err(PartToDiskError::InvalidService);
    }

    let whole_key = CFString::from(K_IO_MEDIA_WHOLE_KEY).as_concrete_TypeRef();
    let bsdname_key = CFString::new(K_BSD_NAME_KEY).as_concrete_TypeRef();

    let result = loop {
        let whole_prop = unsafe { IORegistryEntryCreateCFProperty(service, whole_key, null(), 0) };

        let is_whole = if !whole_prop.is_null()
            && unsafe { CFGetTypeID(whole_prop) } == unsafe { CFBooleanGetTypeID() }
        {
            unsafe { CFBooleanGetValue(whole_prop as CFBooleanRef) }
        } else {
            false
        };

        if !whole_prop.is_null() {
            unsafe { CFRelease(whole_prop) };
        }

        if is_whole {
            let name_prop =
                unsafe { IORegistryEntryCreateCFProperty(service, bsdname_key, null(), 0) };

            if !name_prop.is_null() {
                let mut buf = [0 as c_char; 128];
                let s = if unsafe {
                    CFStringGetCString(
                        name_prop as CFStringRef,
                        buf.as_mut_ptr(),
                        buf.len() as CFIndex,
                        kCFStringEncodingUTF8,
                    )
                } != 0
                {
                    #[cfg(feature = "std")]
                    {
                        use std::ffi::{CStr, OsStr};
                        unsafe {
                            let c_str = CStr::from_ptr(buf.as_ptr());
                            Some(
                                PathBuf::from("/dev")
                                    .join(OsStr::from_encoded_bytes_unchecked(c_str.to_bytes())),
                            )
                        }
                    }
                    #[cfg(feature = "no_std")]
                    {
                        use core::ffi::CStr;

                        unsafe {
                            let c_str = CStr::from_ptr(buf.as_ptr());
                            Some(PathBuf::from("/dev/").join(c_str.to_bytes()))
                        }
                    }
                } else {
                    None
                };
                unsafe { CFRelease(name_prop) };

                break s;
            } else {
                break None;
            };
        }

        let mut parent: IoRegistryEntry = 0;
        let kr = unsafe {
            IORegistryEntryGetParentEntry(
                service,
                K_IO_SERVICE_PLANE.as_ptr() as *mut i8,
                &mut parent,
            )
        };
        unsafe { IOObjectRelease(service) };

        if kr != 0 as KernReturn || parent == 0 {
            break None;
        }
        service = parent;
    };

    unsafe { CFRelease(whole_key as CFTypeRef) };
    unsafe { CFRelease(bsdname_key as CFTypeRef) };

    if service != 0 {
        unsafe { IOObjectRelease(service) };
    }

    match result {
        Some(s) => Ok(s),
        None => Err(PartToDiskError::DiskNotFound),
    }
}
