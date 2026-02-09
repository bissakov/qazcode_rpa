#[cfg(windows)]
pub mod win32;
#[cfg(windows)]
pub use win32::*;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
