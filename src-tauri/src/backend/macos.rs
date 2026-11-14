use crate::backend::PlatformBackend;
use crate::protocol::{Request, Response};
use crate::macos::{install, daemon, cert};
use crate::{gitcfg, kernel, paths};
use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Mutex;

struct Conn { writer: UnixStream, reader: BufReader<UnixStream> }

pub struct MacBackend {
    conn: Mutex<Option<Conn>>,
    kernel_pid: Mutex<Option<u32>>,
}

impl MacBackend {
    pub fn new() -> Self { MacBackend { conn: Mutex::new(None), kernel_pid: Mutex::new(None) } }

    fn ensure_conn(&self) -> Result<()> {
        let mut g = self.conn.lock().unwrap();
        if g.is_some() { return Ok(()); }
        // daemon may still be binding its socket right after install; retry briefly
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let stream = loop {
            match UnixStream::connect(daemon::SOCKET_PATH) {
                Ok(s) => break s,
                Err(e) => {
                    if std::time::Instant::now() >= deadline {
                        return Err(anyhow::anyhow!("连接后台守护进程失败（组件未安装？）: {e}"));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
        };
        let reader = BufReader::new(stream.try_clone()?);
        *g = Some(Conn { writer: stream, reader });
        Ok(())
    }

    fn send(&self, req: &Request) -> Result<Response> {
        self.ensure_conn()?;
        let mut g = self.conn.lock().unwrap();
        let c = g.as_mut().unwrap();
        writeln!(c.writer, "{}", serde_json::to_string(req)?)?;
        c.writer.flush()?;
        let mut line = String::new();
        c.reader.read_line(&mut line)?;
        Ok(serde_json::from_str(&line).context("解析守护进程响应失败")?)
    }

    fn stop_kernel(&self) {
        if let Some(pid) = self.kernel_pid.lock().unwrap().take() { kernel::kill(pid); }
    }
}

impl PlatformBackend for MacBackend {
    fn components_installed(&self) -> bool { install::helper_installed() }

    fn install_components(&self, resource_dir: &Path) -> Result<()> {
        *self.conn.lock().unwrap() = None;
        install::install_helper(&resource_dir.join("resources").join("helper").join("fastgithub-gui-helper"))
    }

    fn enable(&self, kernel_dir: &Path) -> Result<()> {
        if !self.components_installed() { bail!("后台组件未安装"); }
        // Kernel needs the user's GUI/keychain session (.NET AppleCrypto), so it
        // runs as a child of this app, not inside the root daemon.
        let pid = kernel::spawn(kernel_dir)?;
        *self.kernel_pid.lock().unwrap() = Some(pid);
        let r = (|| {
            if let Err(e) = kernel::wait_port(daemon::PROXY_PORT, 20) {
                let log = paths::app_paths().logs_dir.join("fastgithub.log");
                let text = std::fs::read_to_string(&log).unwrap_or_default();
                let tail: Vec<&str> = text.lines().rev().take(6).collect();
                let tail: String = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
                bail!("{e}\n内核进程: {}\n内核日志:\n{tail}", kernel::exit_status(pid));
            }
            // 证书信任写入用户登录钥匙串：GUI 在用户会话内可直接写，无需管理员授权；
            // root daemon 无授权会话，无法设置系统钥匙串信任。
            let cer = kernel_dir.join("cacert").join("fastgithub.cer");
            if !cert::is_trusted(&cer) { cert::install_trusted(&cer)?; }
            let resp = self.send(&Request::Enable { kernel_dir: kernel_dir.to_string_lossy().into() })?;
            if !resp.ok { bail!(resp.message); }
            if gitcfg::git_available() {
                let backup = gitcfg::read_backup();
                let p = paths::app_paths();
                std::fs::write(&p.git_backup_file, serde_json::to_vec(&backup)?)?;
                gitcfg::apply(&p.cacert)?;
            }
            Ok(())
        })();
        if r.is_err() { self.stop_kernel(); }
        r
    }

    fn disable(&self) -> Result<()> {
        let resp = self.send(&Request::Disable);
        self.stop_kernel();
        if gitcfg::git_available() {
            let p = paths::app_paths();
            let backup: gitcfg::GitBackup = std::fs::read_to_string(&p.git_backup_file).ok()
                .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            gitcfg::restore(&backup)?;
        }
        match resp { Ok(r) if !r.ok => bail!(r.message), Err(e) => return Err(e), _ => Ok(()) }
    }

    fn status(&self) -> Result<String> {
        if !self.components_installed() { return Ok("off".into()); }
        Ok(self.send(&Request::Status)?.state)
    }

    fn uninstall(&self) -> Result<()> {
        if self.components_installed() {
            let _ = self.send(&Request::Uninstall);
            *self.conn.lock().unwrap() = None;
            install::uninstall_helper()?;
        }
        self.stop_kernel();
        let p = paths::app_paths();
        let cer = p.kernel_dir.join("cacert").join("fastgithub.cer");
        if let Ok(cn) = cert::cert_cn(&cer) { let _ = cert::remove_trusted(&cn); }
        if gitcfg::git_available() { let _ = gitcfg::restore(&gitcfg::GitBackup::default()); }
        Ok(())
    }
}