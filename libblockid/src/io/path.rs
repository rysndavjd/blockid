// #[cfg(feature = "std")]
// pub trait SysPath: AsRef<std::path::Path> {}

// #[cfg(feature = "std")]
// impl<T: AsRef<std::path::Path>> SysPath for T {}

#[cfg(feature = "std")]
pub use std::path::{Path, PathBuf};

// #[cfg(feature = "no_std")]
// pub trait SysPath: AsRef<Path> {}

// #[cfg(feature = "no_std")]
// impl<T: AsRef<Path>> SysPath for T {}
#[cfg(feature = "no_std")]
pub use no_std::{Path, PathBuf};

#[cfg(feature = "no_std")]
mod no_std {
    use alloc::borrow::Borrow;
    use core::ops::Deref;

    #[repr(transparent)]
    #[derive(Debug)]
    pub struct Path {
        inner: [u8],
    }

    impl Path {
        pub fn new<S: AsRef<[u8]> + ?Sized>(s: &S) -> &Path {
            unsafe { &*(s.as_ref() as *const [u8] as *const Path) }
        }

        pub fn as_bytes(&self) -> &[u8] {
            &self.inner
        }

        pub fn as_mut_bytes(&mut self) -> &mut [u8] {
            &mut self.inner
        }

        pub fn to_path_buf(&self) -> PathBuf {
            PathBuf::from(self.inner.to_vec())
        }
    }

    impl AsRef<[u8]> for Path {
        #[inline]
        fn as_ref(&self) -> &[u8] {
            &self.inner
        }
    }

    impl AsRef<Path> for [u8] {
        #[inline]
        fn as_ref(&self) -> &Path {
            Path::new(self)
        }
    }

    impl AsRef<Path> for PathBuf {
        #[inline]
        fn as_ref(&self) -> &Path {
            self
        }
    }

    impl AsRef<Path> for Vec<u8> {
        #[inline]
        fn as_ref(&self) -> &Path {
            Path::new(self)
        }
    }

    #[derive(Debug)]
    pub struct PathBuf {
        inner: Vec<u8>,
    }

    impl PathBuf {
        pub fn new() -> PathBuf {
            PathBuf { inner: Vec::new() }
        }

        pub fn into_inner(self) -> Vec<u8> {
            self.inner
        }

        pub fn as_bytes(&self) -> &[u8] {
            &self.inner
        }

        pub fn as_mut_bytes(&mut self) -> &mut [u8] {
            &mut self.inner
        }
    }

    impl From<PathBuf> for Vec<u8> {
        #[inline]
        fn from(path_buf: PathBuf) -> Vec<u8> {
            path_buf.inner
        }
    }

    impl From<Vec<u8>> for PathBuf {
        #[inline]
        fn from(string: Vec<u8>) -> PathBuf {
            PathBuf { inner: string }
        }
    }

    impl From<&[u8]> for PathBuf {
        fn from(buf: &[u8]) -> Self {
            let null = buf.iter().position(|b| b == &0).unwrap_or(buf.len());

            return PathBuf {
                inner: buf[..null].to_vec(),
            };
        }
    }

    impl Borrow<Path> for PathBuf {
        #[inline]
        fn borrow(&self) -> &Path {
            self.deref()
        }
    }

    impl Deref for PathBuf {
        type Target = Path;
        #[inline]
        fn deref(&self) -> &Path {
            Path::new(&self.inner)
        }
    }
}
