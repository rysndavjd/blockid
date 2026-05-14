#[cfg(feature = "std")]
pub use std::path::{Path, PathBuf};

#[cfg(feature = "no_std")]
pub use no_std::{Path, PathBuf};

#[cfg(feature = "no_std")]
pub mod no_std {
    use alloc::borrow::Borrow;
    use core::ops::Deref;

    /// A slice of a path, similar to [`Path`] but for `no_std`.
    ///
    /// This is used as a wrapper for `no_std` environments as
    /// unlike [`Path`], this type treats paths as raw bytes,
    /// allowing arbitrary bytes as found in UNIX paths.
    ///
    /// [`Path`]: https://doc.rust-lang.org/std/path/struct.Path.html
    #[derive(Debug)]
    #[repr(transparent)]
    pub struct Path {
        inner: [u8],
    }

    impl Path {
        pub fn new<S: AsRef<[u8]> + ?Sized>(s: &S) -> &Path {
            unsafe { &*(s.as_ref() as *const [u8] as *const Path) }
        }

        /// Returns the path as a raw byte slice.
        pub fn as_bytes(&self) -> &[u8] {
            &self.inner
        }

        /// Returns the path as a mutable raw byte slice.
        pub fn as_mut_bytes(&mut self) -> &mut [u8] {
            &mut self.inner
        }

        /// Converts this [`Path`] to an owned [`PathBuf`].
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

    /// An owned, heap allocated path.
    ///
    /// [`PathBuf`] is the owned counterpart to [`Path`].
    #[derive(Debug)]
    #[repr(transparent)]
    pub struct PathBuf {
        inner: Vec<u8>,
    }

    impl PathBuf {
        /// Creates an empty `PathBuf`.
        pub fn new() -> PathBuf {
            PathBuf { inner: Vec::new() }
        }

        /// Consumes the [`PathBuf`] and returns the underlying [`Vec<u8>`].
        pub fn into_inner(self) -> Vec<u8> {
            self.inner
        }

        /// Returns the path as a raw byte slice.
        pub fn as_bytes(&self) -> &[u8] {
            &self.inner
        }

        /// Returns the path as a mutable raw byte slice.
        pub fn as_mut_bytes(&mut self) -> &mut [u8] {
            &mut self.inner
        }
    }

    impl Default for PathBuf {
        fn default() -> Self {
            Self::new()
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
