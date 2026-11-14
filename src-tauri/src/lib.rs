pub mod state;
pub mod paths;
pub mod protocol;
pub mod gitcfg;
pub mod kernel;
pub mod backend;
pub mod session;
pub mod watchdog;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(any(target_os = "linux", test))]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;