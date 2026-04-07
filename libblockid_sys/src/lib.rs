mod io;
mod ioctl;
mod path;
mod probe;
mod topology;

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(not(feature = "std"), not(test)))]
extern crate core as std;

extern crate alloc;
