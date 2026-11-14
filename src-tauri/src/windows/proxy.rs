use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use winreg::enums::*;
use winreg::RegKey;

pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 38457;
const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyBackup {
    pub proxy_enable: u32,
    pub proxy_server: String,
    pub proxy_override: String,
    pub auto_config_url: String,
}

pub fn proxy_server_value() -> String { format!("{HOST}:{PORT}") }

fn internet_settings() -> Result<RegKey> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(INTERNET_SETTINGS)
        .context("打开 HKCU Internet Settings 失败")?;
    Ok(key)
}

pub fn get_proxy() -> Result<ProxyBackup> {
    let key = internet_settings()?;
    Ok(ProxyBackup {
        proxy_enable: key.get_value("ProxyEnable").unwrap_or(0),
        proxy_server: key.get_value("ProxyServer").unwrap_or_default(),
        proxy_override: key.get_value("ProxyOverride").unwrap_or_default(),
        auto_config_url: key.get_value("AutoConfigURL").unwrap_or_default(),
    })
}

pub fn set_proxy() -> Result<()> {
    let key = internet_settings()?;
    key.set_value("ProxyEnable", &1u32).context("写 ProxyEnable 失败")?;
    key.set_value("ProxyServer", &proxy_server_value()).context("写 ProxyServer 失败")?;
    notify();
    Ok(())
}

pub fn restore_proxy(b: &ProxyBackup) -> Result<()> {
    let key = internet_settings()?;
    key.set_value("ProxyEnable", &b.proxy_enable)?;
    key.set_value("ProxyServer", &b.proxy_server)?;
    key.set_value("ProxyOverride", &b.proxy_override)?;
    if b.auto_config_url.is_empty() { let _ = key.delete_value("AutoConfigURL"); }
    else { key.set_value("AutoConfigURL", &b.auto_config_url)?; }
    notify();
    Ok(())
}

fn notify() {
    use windows_sys::Win32::Networking::WinInet::{
        InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED,
    };
    unsafe {
        InternetSetOptionW(std::ptr::null(), INTERNET_OPTION_SETTINGS_CHANGED, std::ptr::null(), 0);
        InternetSetOptionW(std::ptr::null(), INTERNET_OPTION_REFRESH, std::ptr::null(), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_value() { assert_eq!(proxy_server_value(), "127.0.0.1:38457"); }
    #[test]
    fn backup_roundtrip() {
        let b = ProxyBackup { proxy_enable: 0, proxy_server: "127.0.0.1:7890".into(),
            proxy_override: "<local>".into(), auto_config_url: String::new() };
        let s = serde_json::to_string(&b).unwrap();
        let b2: ProxyBackup = serde_json::from_str(&s).unwrap();
        assert_eq!(b2.proxy_server, "127.0.0.1:7890");
        assert_eq!(b2.proxy_override, "<local>");
    }
}