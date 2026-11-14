use anyhow::{Context, Result, bail};
use std::net::TcpStream;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

pub fn spawn(kernel_dir: &Path) -> Result<u32> {
    let bin = kernel_dir.join(kernel_bin_name());
    let log = kernel_dir.parent().unwrap().join("logs").join("fastgithub.log");
    if let Some(p) = log.parent() { std::fs::create_dir_all(p)?; }
    let logf = std::fs::File::create(&log).context("创建日志文件失败")?;
    let child = crate::proc::command(&bin)
        .current_dir(kernel_dir)
        .stdout(Stdio::from(logf.try_clone()?))
        .stderr(Stdio::from(logf))
        .spawn().with_context(|| format!("启动内核失败: {}", bin.display()))?;
    Ok(child.id())
}

pub fn wait_port(port: u16, secs: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() { return Ok(()); }
        std::thread::sleep(Duration::from_millis(300));
    }
    bail!("内核未在 {secs}s 内监听端口 {port}")
}

pub fn kill(pid: u32) {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, libc::SIGTERM); }
        std::thread::sleep(Duration::from_millis(500));
        unsafe { libc::kill(pid as i32, libc::SIGKILL); }
    }
    #[cfg(windows)]
    {
        let _ = crate::proc::command("taskkill").args(["/F", "/T", "/PID", &pid.to_string()]).output();
    }
}

#[cfg(unix)]
pub fn exit_status(pid: u32) -> String {
    let mut status: libc::c_int = 0;
    let r = unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) };
    if r == 0 { return "仍在运行".into(); }
    if r < 0 { return format!("waitpid 错误: {}", std::io::Error::last_os_error()); }
    if libc::WIFSIGNALED(status) { return format!("被信号 {} 杀死", libc::WTERMSIG(status)); }
    if libc::WIFEXITED(status) { return format!("退出码 {}", libc::WEXITSTATUS(status)); }
    format!("status {status}")
}

pub fn extract_resources(resource_kernel: &Path, dest: &Path) -> Result<()> {
    // 重建目录而非原地覆盖：旧目录可能被安全软件标记导致内核 exec 即被 SIGKILL，
    // 换新 inode 可恢复；同时清掉升级残留的旧文件。
    if dest.exists() { std::fs::remove_dir_all(dest)?; }
    copy_dir(resource_kernel, dest)
}

fn kernel_bin_name() -> &'static str {
    if cfg!(target_os = "windows") { "fastgithub.exe" } else { "fastgithub" }
}

pub fn needs_extract(resource_kernel: &Path, dest: &Path) -> Result<bool> {
    let name = kernel_bin_name();
    let dst = dest.join(name);
    if !dst.exists() { return Ok(true); }
    Ok(file_sha256(&resource_kernel.join(name))? != file_sha256(&dst)?)
}

fn file_sha256(path: &Path) -> Result<[u8; 32]> {
    use sha2::{Sha256, Digest};
    use std::io::Read;
    let mut f = std::fs::File::open(path).with_context(|| format!("读取失败: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() { copy_dir(&entry.path(), &to)?; }
        else { std::fs::copy(entry.path(), &to)?; }
    }
    Ok(())
}