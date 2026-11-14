use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub fn parse_cn(subject: &str) -> Option<String> {
    let subject = subject.trim().strip_prefix("subject=").unwrap_or(subject.trim());
    for part in subject.split(|c| c == ',' || c == '/') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("CN") {
            let v = v.trim_start_matches(|c| c == '=' || c == ' ').trim();
            if !v.is_empty() { return Some(v.to_string()); }
        }
    }
    None
}

pub fn cert_cn(cer: &Path) -> Result<String> {
    // upstream writes PEM into the .cer file; tolerate DER too
    let mut last_err = String::new();
    for inform in ["PEM", "DER"] {
        let out = Command::new("openssl")
            .args(["x509", "-inform", inform, "-in"]).arg(cer)
            .args(["-noout", "-subject"]).output().context("运行 openssl 失败")?;
        if out.status.success() {
            return parse_cn(&String::from_utf8_lossy(&out.stdout)).context("证书无 CN");
        }
        last_err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    }
    bail!("读取证书主题失败 [{}]: {}", cer.display(), last_err)
}

// 信任写入用户登录钥匙串（user 域）：普通用户即可写，无需管理员授权会话；
// 系统钥匙串在 root daemon / 非交互会话下会报 "Write permissions error"。
fn login_keychain() -> Result<String> {
    let out = Command::new("security").arg("login-keychain").output().context("运行 security 失败")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().trim_matches('"').to_string())
}

pub fn is_trusted(cer: &Path) -> bool {
    Command::new("security").args(["verify-cert", "-c"]).arg(cer).args(["-p", "ssl"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn install_trusted(cer: &Path) -> Result<()> {
    let kc = login_keychain()?;
    let out = Command::new("security")
        .args(["add-trusted-cert", "-r", "trustRoot", "-k", &kc]).arg(cer)
        .output().context("安装证书失败")?;
    if !out.status.success() { bail!("信任证书失败: {}", String::from_utf8_lossy(&out.stderr)); }
    Ok(())
}

pub fn remove_trusted(cn: &str) -> Result<()> {
    let kc = login_keychain()?;
    let _ = Command::new("security").args(["delete-certificate", "-c", cn, "-t", &kc]).output();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cn_equals() { assert_eq!(parse_cn("subject=CN = FastGithub").unwrap(), "FastGithub"); }
    #[test]
    fn cn_slash() { assert_eq!(parse_cn("subject= /C=CN/CN=FastGithub CA").unwrap(), "FastGithub CA"); }
    #[test]
    fn cn_none() { assert!(parse_cn("subject=O=Foo").is_none()); }
}