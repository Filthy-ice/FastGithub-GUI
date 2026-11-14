use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::process::Command;

pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 38457;

// 备份保存 gsettings get 的原始输出（字符串/数组自带引号），还原时原样 set 回去。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyBackup {
    pub mode: String,
    pub http_host: String,
    pub http_port: i32,
    pub https_host: String,
    pub https_port: i32,
    pub ignore_hosts: String,
}

pub fn is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("GNOME"))
        .unwrap_or(false)
}

fn gsettings(args: &[&str]) -> Result<String> {
    let out = Command::new("gsettings").args(args).output().context("运行 gsettings 失败（非 GNOME？）")?;
    if !out.status.success() { bail!("gsettings 失败: {}", String::from_utf8_lossy(&out.stderr)); }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn get(schema: &str, key: &str) -> Result<String> { gsettings(&["get", schema, key]) }
fn set(schema: &str, key: &str, value: &str) -> Result<()> { gsettings(&["set", schema, key, value])?; Ok(()) }

pub fn get_proxy() -> Result<ProxyBackup> {
    Ok(ProxyBackup {
        mode: get("org.gnome.system.proxy", "mode")?,
        http_host: get("org.gnome.system.proxy.http", "host")?,
        http_port: get("org.gnome.system.proxy.http", "port")?.parse().unwrap_or(0),
        https_host: get("org.gnome.system.proxy.https", "host")?,
        https_port: get("org.gnome.system.proxy.https", "port")?.parse().unwrap_or(0),
        ignore_hosts: get("org.gnome.system.proxy", "ignore-hosts")?,
    })
}

pub fn set_proxy() -> Result<()> {
    set("org.gnome.system.proxy", "mode", "'manual'")?;
    set("org.gnome.system.proxy.http", "host", &format!("'{HOST}'"))?;
    set("org.gnome.system.proxy.http", "port", &PORT.to_string())?;
    set("org.gnome.system.proxy.https", "host", &format!("'{HOST}'"))?;
    set("org.gnome.system.proxy.https", "port", &PORT.to_string())?;
    Ok(())
}

pub fn restore_proxy(b: &ProxyBackup) -> Result<()> {
    set("org.gnome.system.proxy.http", "host", &b.http_host)?;
    set("org.gnome.system.proxy.http", "port", &b.http_port.to_string())?;
    set("org.gnome.system.proxy.https", "host", &b.https_host)?;
    set("org.gnome.system.proxy.https", "port", &b.https_port.to_string())?;
    set("org.gnome.system.proxy", "ignore-hosts", &b.ignore_hosts)?;
    set("org.gnome.system.proxy", "mode", &b.mode)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_roundtrip() {
        let b = ProxyBackup { mode: "'none'".into(), http_host: "''".into(), http_port: 0,
            https_host: "''".into(), https_port: 0, ignore_hosts: "['localhost']".into() };
        let s = serde_json::to_string(&b).unwrap();
        let b2: ProxyBackup = serde_json::from_str(&s).unwrap();
        assert_eq!(b2.mode, "'none'");
        assert_eq!(b2.ignore_hosts, "['localhost']");
    }
}