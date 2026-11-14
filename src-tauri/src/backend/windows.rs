use crate::backend::PlatformBackend;
use crate::windows::{proxy, cert};
use crate::{gitcfg, kernel, paths, session::SessionState};
use anyhow::Result;
use std::path::Path;
use std::sync::Mutex;

pub struct WindowsBackend { kernel_pid: Mutex<Option<u32>> }

impl WindowsBackend {
    pub fn new() -> Self { WindowsBackend { kernel_pid: Mutex::new(None) } }
    fn stop_kernel(&self) {
        if let Some(pid) = self.kernel_pid.lock().unwrap().take() { kernel::kill(pid); }
    }
    fn restore_proxy_from_state() {
        let st = SessionState::load(&paths::session_file());
        if st.proxy_set_by_us {
            if let Ok(b) = serde_json::from_value::<proxy::ProxyBackup>(st.proxy_backup.clone()) {
                let _ = proxy::restore_proxy(&b);
            }
        }
    }
}

impl PlatformBackend for WindowsBackend {
    fn components_installed(&self) -> bool { true }
    fn install_components(&self, _resource_dir: &Path) -> Result<()> { Ok(()) }

    fn enable(&self, kernel_dir: &Path) -> Result<()> {
        let pid = kernel::spawn(kernel_dir)?;
        *self.kernel_pid.lock().unwrap() = Some(pid);
        let r = (|| {
            kernel::wait_port(proxy::PORT, 20)?;
            let cer = kernel_dir.join("cacert").join("fastgithub.cer");
            if !cert::is_trusted(&cer)? { cert::install_trusted(&cer)?; }
            let backup = proxy::get_proxy()?;
            let mut state = SessionState { kernel_pid: pid, ..Default::default() };
            state.proxy_backup = serde_json::to_value(&backup)?;
            proxy::set_proxy()?;
            state.proxy_set_by_us = true;
            state.save(&paths::session_file())?;
            if gitcfg::git_available() {
                let gb = gitcfg::read_backup();
                let p = paths::app_paths();
                std::fs::write(&p.git_backup_file, serde_json::to_vec(&gb)?)?;
                gitcfg::apply(&p.cacert)?;
            }
            Ok(())
        })();
        if r.is_err() {
            Self::restore_proxy_from_state();
            self.stop_kernel();
            let _ = SessionState::default().save(&paths::session_file());
        }
        r
    }

    fn disable(&self) -> Result<()> {
        Self::restore_proxy_from_state();
        self.stop_kernel();
        if gitcfg::git_available() {
            let p = paths::app_paths();
            let backup: gitcfg::GitBackup = std::fs::read_to_string(&p.git_backup_file).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            gitcfg::restore(&backup)?;
        }
        let mut cleared = SessionState::default();
        cleared.normal_exit = true;
        cleared.save(&paths::session_file())?;
        Ok(())
    }

    fn status(&self) -> Result<String> {
        let st = SessionState::load(&paths::session_file());
        Ok(if st.proxy_set_by_us { "on".into() } else { "off".into() })
    }

    fn uninstall(&self) -> Result<()> {
        let _ = self.disable();
        let cer = paths::app_paths().cacert;
        let _ = cert::remove_trusted(&cer);
        Ok(())
    }
}