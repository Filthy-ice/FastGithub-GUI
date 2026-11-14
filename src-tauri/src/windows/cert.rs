use anyhow::{Context, Result, bail};
use std::path::Path;

// 用 PowerShell 取 SHA1 指纹：与 certutil 本地化输出无关（中文系统亦稳定）。
pub fn thumbprint(cer: &Path) -> Result<String> {
    let script = format!(
        "(New-Object System.Security.Cryptography.X509Certificates.X509Certificate2('{}')).Thumbprint",
        cer.display()
    );
    let out = crate::proc::command("powershell").args(["-NoProfile", "-Command", &script])
        .output().context("运行 powershell 失败")?;
    if !out.status.success() { bail!("读取证书指纹失败: {}", String::from_utf8_lossy(&out.stderr)); }
    let tp = String::from_utf8_lossy(&out.stdout).trim().to_uppercase();
    if tp.is_empty() { bail!("证书指纹为空"); }
    Ok(tp)
}

pub fn is_trusted(cer: &Path) -> Result<bool> {
    let tp = thumbprint(cer)?;
    let out = crate::proc::command("certutil").args(["-user", "-store", "Root", &tp])
        .output().context("运行 certutil 失败")?;
    Ok(out.status.success())
}

pub fn install_trusted(cer: &Path) -> Result<()> {
    let out = crate::proc::command("certutil").args(["-user", "-addstore", "-f", "Root"]).arg(cer)
        .output().context("运行 certutil 失败")?;
    if !out.status.success() { bail!("信任证书失败: {}", String::from_utf8_lossy(&out.stderr)); }
    Ok(())
}

pub fn remove_trusted(cer: &Path) -> Result<()> {
    if let Ok(tp) = thumbprint(cer) {
        let _ = crate::proc::command("certutil").args(["-user", "-delstore", "Root", &tp]).output();
    }
    Ok(())
}