use anyhow::Result;
use std::path::Path;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

pub trait PlatformBackend: Send + Sync {
    fn components_installed(&self) -> bool;
    fn install_components(&self, resource_dir: &Path) -> Result<()>;
    fn enable(&self, kernel_dir: &Path) -> Result<()>;
    fn disable(&self) -> Result<()>;
    fn status(&self) -> Result<String>;
    fn uninstall(&self) -> Result<()>;
}