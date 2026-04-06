#[cfg(target_os = "freebsd")]
mod freebsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/* Note:
 * The rustix::ioctl::opcode::read function seems to calculate different values
 * on different systems. For example:
 *
 *   read::<u32>(b'd', 24) == 2147771416 on Linux
 *   read::<u32>(b'd', 24) == 1074029592 on macOS
 */
