use crate::protocol::{Request, Response};
use crate::macos::proxy;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::Command;

pub const SOCKET_PATH: &str = "/var/run/fastgithub-gui.sock";
pub const STATE_FILE: &str = "/Library/Application Support/FastGithubGUI/daemon-state.json";
pub const PROXY_HOST: &str = "127.0.0.1";
pub const PROXY_PORT: u16 = 38457;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DaemonState {
    pub installer_uid: u32,
    pub service: String,
    pub web_backup: Option<proxy::ProxySetting>,
    pub secure_backup: Option<proxy::ProxySetting>,
    pub proxy_set_by_us: bool,
    pub kernel_dir: String,
}

impl DaemonState {
    pub fn load() -> DaemonState {
        std::fs::read_to_string(STATE_FILE).ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) -> Result<()> {
        if let Some(p) = Path::new(STATE_FILE).parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(STATE_FILE, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

// The kernel runs as a child of the GUI app (user session); on crash/exit the
// daemon reaps any orphan so a dead app never leaves the accelerator running.
fn kill_kernel(state: &DaemonState) {
    if state.kernel_dir.is_empty() { return; }
    let bin = format!("{}/fastgithub", state.kernel_dir);
    let _ = Command::new("pkill").arg("-f").arg(&bin).output();
}

fn enable(state: &mut DaemonState, kernel_dir: &str) -> Result<()> {
    state.kernel_dir = kernel_dir.into();
    state.save()?;
    // 证书信任由 GUI 侧在用户会话内完成；daemon 无授权会话，只负责系统代理。
    let service = proxy::default_service()?;
    let (web, secure) = proxy::get_proxy(&service)?;
    state.service = service.clone();
    state.web_backup = Some(web);
    state.secure_backup = Some(secure);
    proxy::set_proxy(&service, PROXY_HOST, PROXY_PORT)?;
    state.proxy_set_by_us = true;
    state.save()?;
    Ok(())
}

fn disable(state: &mut DaemonState) -> Result<()> {
    if state.proxy_set_by_us {
        if let (Some(w), Some(s)) = (&state.web_backup, &state.secure_backup) {
            let _ = proxy::restore_proxy(&state.service, w, s);
        }
        state.proxy_set_by_us = false;
    }
    kill_kernel(state);
    state.save()?;
    Ok(())
}

fn uninstall_cleanup(state: &mut DaemonState) {
    let _ = disable(state);
}

fn cleanup(state: &mut DaemonState) {
    let _ = disable(state);
}

fn peer_uid(stream: &UnixStream) -> Result<u32> {
    use std::os::unix::io::AsRawFd;
    let mut uid: libc::uid_t = 0; let mut gid: libc::gid_t = 0;
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0 { bail!("getpeereid 失败"); }
    Ok(uid)
}

fn handle_connection(stream: UnixStream, state: &mut DaemonState) {
    let mut writer = stream.try_clone().expect("clone stream");
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let req: Request = match serde_json::from_str(&line) { Ok(r) => r, Err(_) => continue };
        let resp = match req {
            Request::Enable { kernel_dir } => match enable(state, &kernel_dir) {
                Ok(_) => Response::ok("on"),
                Err(e) => { let _ = disable(state); Response::err("off", format!("{e:#}")) }
            },
            Request::Disable => match disable(state) { Ok(_) => Response::ok("off"), Err(e) => Response::err("off", format!("{e:#}")) },
            Request::Status => Response::ok(if state.proxy_set_by_us {"on"} else {"off"}),
            Request::Uninstall => { uninstall_cleanup(state); Response::ok("off") }
        };
        let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap());
        let _ = writer.flush();
    }
}

pub fn run() -> Result<()> {
    let mut state = DaemonState::load();
    cleanup(&mut state);
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH).context("绑定 socket 失败")?;
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o666))?;
    }
    for stream in listener.incoming() {
        let stream = stream?;
        let uid = peer_uid(&stream)?;
        if state.installer_uid == 0 { state.installer_uid = uid; state.save()?; }
        else if uid != state.installer_uid { continue; }
        handle_connection(stream, &mut state);
        let _ = disable(&mut state);
    }
    Ok(())
}