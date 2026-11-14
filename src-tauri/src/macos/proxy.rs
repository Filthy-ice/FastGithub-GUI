use anyhow::{Context, Result, bail};
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProxySetting { pub enabled: bool, pub server: String, pub port: u16 }

pub fn parse_getwebproxy(output: &str) -> ProxySetting {
    let mut enabled = false; let mut server = String::new(); let mut port = 0u16;
    for line in output.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("Enabled:") { enabled = v.trim().eq_ignore_ascii_case("yes"); }
        else if let Some(v) = line.strip_prefix("Server:") { server = v.trim().to_string(); }
        else if let Some(v) = line.strip_prefix("Port:") { port = v.trim().parse().unwrap_or(0); }
    }
    ProxySetting { enabled, server, port }
}

pub fn parse_default_interface(route_output: &str) -> Option<String> {
    for line in route_output.lines() {
        if let Some(v) = line.trim().strip_prefix("interface:") { return Some(v.trim().to_string()); }
    }
    None
}

pub fn parse_service_order(output: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut pending: Option<String> = None;
    for line in output.lines() {
        let line = line.trim();
        if line.starts_with('(') && !line.starts_with("(Hardware") {
            if let Some(idx) = line.find(')') {
                let name = line[idx+1..].trim().to_string();
                if !name.is_empty() { pending = Some(name); }
            }
        } else if line.starts_with("(Hardware Port:") {
            if let Some(dev_idx) = line.find("Device:") {
                let dev = line[dev_idx+7..].trim().trim_end_matches(')').trim().to_string();
                if let Some(name) = pending.take() { result.push((name, dev)); }
            }
        }
    }
    result
}

pub fn service_for_interface(services: &[(String, String)], iface: &str) -> Option<String> {
    services.iter().find(|(_, d)| d == iface).map(|(n, _)| n.clone())
}

fn run(cmd: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(cmd).args(args).output().with_context(|| format!("运行 {cmd} 失败"))?;
    if !out.status.success() { bail!("{cmd} 失败: {}", String::from_utf8_lossy(&out.stderr)); }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn default_service() -> Result<String> {
    let route = run("route", &["-n", "get", "default"])?;
    let iface = parse_default_interface(&route).context("无法解析默认网卡")?;
    let order = run("networksetup", &["-listnetworkserviceorder"])?;
    service_for_interface(&parse_service_order(&order), &iface).context("无法映射网络服务")
}

pub fn get_proxy(service: &str) -> Result<(ProxySetting, ProxySetting)> {
    let web = parse_getwebproxy(&run("networksetup", &["-getwebproxy", service])?);
    let secure = parse_getwebproxy(&run("networksetup", &["-getsecurewebproxy", service])?);
    Ok((web, secure))
}

pub fn set_proxy(service: &str, host: &str, port: u16) -> Result<()> {
    let p = port.to_string();
    run("networksetup", &["-setwebproxy", service, host, &p])?;
    run("networksetup", &["-setsecurewebproxy", service, host, &p])?;
    run("networksetup", &["-setwebproxystate", service, "on"])?;
    run("networksetup", &["-setsecurewebproxystate", service, "on"])?;
    Ok(())
}

pub fn restore_proxy(service: &str, web: &ProxySetting, secure: &ProxySetting) -> Result<()> {
    if !web.server.is_empty() { run("networksetup", &["-setwebproxy", service, &web.server, &web.port.to_string()])?; }
    run("networksetup", &["-setwebproxystate", service, if web.enabled {"on"} else {"off"}])?;
    if !secure.server.is_empty() { run("networksetup", &["-setsecurewebproxy", service, &secure.server, &secure.port.to_string()])?; }
    run("networksetup", &["-setsecurewebproxystate", service, if secure.enabled {"on"} else {"off"}])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_proxy() {
        let s = "Enabled: Yes\nServer: 127.0.0.1\nPort: 38457\nAuthenticated Proxy Enabled: 0\n";
        let p = parse_getwebproxy(s);
        assert!(p.enabled); assert_eq!(p.server, "127.0.0.1"); assert_eq!(p.port, 38457);
    }
    #[test]
    fn parse_proxy_off() {
        let p = parse_getwebproxy("Enabled: No\nServer:\nPort: 0\n");
        assert!(!p.enabled); assert_eq!(p.port, 0);
    }
    #[test]
    fn parse_iface() {
        let s = "   route to: default\n destination: default\n  interface: en0\n";
        assert_eq!(parse_default_interface(s).unwrap(), "en0");
    }
    #[test]
    fn parse_services() {
        let s = "(1) Wi-Fi\n(Hardware Port: Wi-Fi, Device: en0)\n\n(2) Ethernet\n(Hardware Port: Ethernet, Device: en1)\n";
        let v = parse_service_order(s);
        assert_eq!(v, vec![("Wi-Fi".into(),"en0".into()),("Ethernet".into(),"en1".into())]);
        assert_eq!(service_for_interface(&v, "en0").unwrap(), "Wi-Fi");
    }
}