use log::{Record, Level, Metadata, LevelFilter};
#[cfg(not(feature = "std"))]
use rustix::{fd::{BorrowedFd}, io::write};
use alloc::format;

pub static LOGGER: Logger = Logger;

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            #[cfg(feature = "std")]
            println!("{} - {}", record.level(), record.args());
            #[cfg(not(feature = "std"))]
            {
                // SAFETY: it is up to the OS to provide valid file descriptors
                // for stdout.
                let stdout = unsafe { BorrowedFd::borrow_raw(1) };
                let message = format!("{} - {}\n", record.level(), record.args());
                let _ = write(stdout, message.as_bytes());
            }
        }
    }

    fn flush(&self) {}
}

pub fn init_logger() {
    log::set_logger(&LOGGER).unwrap();
    #[cfg(debug_assertions)]
    log::set_max_level(LevelFilter::Debug);
    #[cfg(not(debug_assertions))]
    log::set_max_level(LevelFilter::Warn);
}