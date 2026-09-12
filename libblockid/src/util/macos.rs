#[link(name = "IOKit")]
unsafe extern "C" {
    static kIOMainPortDefault: mach_port_t;
    fn IOObjectRelease(object: io_object_t) -> kern_return_t;
    fn IOObjectConformsTo(
        object: io_object_t,
        className: *const ::std::os::raw::c_char,
    ) -> boolean_t;
    fn IOIteratorNext(iterator: io_iterator_t) -> io_object_t;
    fn IOServiceGetMatchingServices(
        mainPort: mach_port_t,
        matching: CFDictionaryRef,
        existing: *mut io_iterator_t,
    ) -> kern_return_t;
    fn CFDictionarySetValue(
        theDict: CFMutableDictionaryRef,
        key: *const ::std::os::raw::c_void,
        value: *const ::std::os::raw::c_void,
    );
    fn IORegistryEntryGetParentEntry(
        entry: io_registry_entry_t,
        plane: *const ::std::os::raw::c_char,
        parent: *mut io_registry_entry_t,
    ) -> kern_return_t;
    fn IOServiceMatching(name: *const ::std::os::raw::c_char) -> CFMutableDictionaryRef;
}

type boolean_t = ::std::os::raw::c_int;
type mach_port_t = ::std::os::raw::c_uint;
type kern_return_t = ::std::os::raw::c_int;
type UInt32 = ::std::os::raw::c_uint;
type Boolean = ::std::os::raw::c_uchar;
type CFTypeID = ::std::os::raw::c_ulong;
type CFIndex = ::std::os::raw::c_long;
type CFTypeRef = *const ::std::os::raw::c_void;

#[repr(C)]
#[derive(Debug)]
struct __CFDictionary;
type CFDictionaryRef = *const __CFDictionary;
type CFMutableDictionaryRef = *mut __CFDictionary;
type IOOptionBits = UInt32;
type io_object_t = mach_port_t;
type io_name_t = [::std::os::raw::c_char; 128usize];
type io_iterator_t = io_object_t;
type io_registry_entry_t = io_object_t;
type io_service_t = io_object_t;
