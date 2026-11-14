use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

const NICK: &str = "fastgithub";
const SYSTEM_CA: &str = "/usr/local/share/ca-certificates/fastgithub.crt";

fn nssdb() -> Result<String> {
    let home = std::env::var("HOME").context("HOME 未设置")?;
    Ok(format!("sql:{home}/.pki/nssdb"))
}

pub fn certutil_available() -> bool {
    Command::new("certutil").arg("-H").output().is_ok()
}

fn ensure_nssdb() -> Result<()> {
    let home = std::env::var("HOME").context("HOME 未设置")?;
    let dir = Path::new(&home).join(".pki").join("nssdb");
    std::fs::create_dir_all(&dir)?;
    if !dir.join("cert9.db").exists() {
        let db = nssdb()?;
        let out = Command::new("certutil").args(["-d", &db, "-N", "--empty-password"])
            .output().context("运行 certutil 失败")?;
        if !out.status.success() { bail!("初始化 NSS 数据库失败: {}", String::from_utf8_lossy(&out.stderr)); }
    }
    Ok(())
}

pub fn is_trusted() -> Result<bool> {
    let db = nssdb()?;
    let out = Command::new("certutil").args(["-d", &db, "-L", "-n", NICK])
        .output().context("运行 certutil 失败")?;
    Ok(out.status.success())
}

pub fn install_trusted(cer: &Path) -> Result<()> {
    ensure_nssdb()?;
    let db = nssdb()?;
    let out = Command::new("certutil").args(["-d", &db, "-A", "-t", "C,,", "-n", NICK, "-i"]).arg(cer)
        .output().context("运行 certutil 失败")?;
    if !out.status.success() { bail!("信任证书失败: {}", String::from_utf8_lossy(&out.stderr)); }
    Ok(())
}

pub fn remove_trusted() -> Result<()> {
    let db = nssdb()?;
    let _ = Command::new("certutil").args(["-d", &db, "-D", "-n", NICK]).output();
    Ok(())
}

pub fn system_ca_installed() -> bool { Path::new(SYSTEM_CA).exists() }

// 可选系统 CA（覆盖 curl 等）：pkexec 提权，用户取消则返回 Err，调用方不视为致命。
pub fn install_system_ca(cer: &Path) -> Result<()> {
    let script = format!("cp '{}' '{SYSTEM_CA}' && update-ca-certificates", cer.display());
    let out = Command::new("pkexec").args(["/bin/sh", "-c", &script])
        .output().context("运行 pkexec 失败")?;
    if !out.status.success() { bail!("系统 CA 安装取消/失败: {}", String::from_utf8_lossy(&out.stderr)); }
    Ok(())
}

pub fn remove_system_ca() -> Result<()> {
    let script = format!("rm -f '{SYSTEM_CA}' && update-ca-certificates --fresh");
    let _ = Command::new("pkexec").args(["/bin/sh", "-c", &script]).output();
    Ok(())
}