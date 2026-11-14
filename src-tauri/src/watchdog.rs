use crate::session::SessionState;
use anyhow::Result;
use std::path::Path;
use std::time::Duration;

// Win/Linux 崩溃安全：监视父进程（GUI），父消失且非正常退出 → 还原代理、结束内核。
pub fn run(parent_pid: u32, state_path: &Path) -> Result<()> {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        if parent_alive(parent_pid) { continue; }
        let state = SessionState::load(state_path);
        if state.proxy_set_by_us && !state.normal_exit {
            emergency_restore(&state);
            if state.kernel_pid != 0 { crate::kernel::kill(state.kernel_pid); }
        }
        return Ok(());
    }
}

fn parent_alive(pid: u32) -> bool {
    #[cfg(unix)]
    { unsafe { libc::kill(pid as i32, 0) == 0 } }
    #[cfg(windows)]
    {
        let out = crate::proc::command("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"]).output();
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()),
            Err(_) => false,
        }
    }
}

fn emergency_restore(state: &SessionState) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(b) = serde_json::from_value::<crate::linux::proxy::ProxyBackup>(state.proxy_backup.clone()) {
            let _ = crate::linux::proxy::restore_proxy(&b);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(b) = serde_json::from_value::<crate::windows::proxy::ProxyBackup>(state.proxy_backup.clone()) {
            let _ = crate::windows::proxy::restore_proxy(&b);
        }
    }
    let _ = state;
}