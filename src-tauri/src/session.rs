use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

// Win/Linux 看门狗共享的状态文件：崩溃时据此还原代理、结束内核。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub proxy_set_by_us: bool,
    pub kernel_pid: u32,
    pub normal_exit: bool,
    pub proxy_backup: serde_json::Value,
}

impl SessionState {
    pub fn load(path: &Path) -> SessionState {
        std::fs::read_to_string(path).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let s = SessionState { proxy_set_by_us: true, kernel_pid: 42, normal_exit: false,
            proxy_backup: serde_json::json!({"mode": "'none'"}) };
        let json = serde_json::to_string(&s).unwrap();
        let s2: SessionState = serde_json::from_str(&json).unwrap();
        assert!(s2.proxy_set_by_us);
        assert_eq!(s2.kernel_pid, 42);
        assert_eq!(s2.proxy_backup["mode"], "'none'");
    }
}