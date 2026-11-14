use std::path::Path;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const PROXY_KEY: &str = "http.https://github.com.proxy";
pub const SSL_KEY: &str = "http.https://github.com.sslCAInfo";
pub const PROXY_VALUE: &str = "http://127.0.0.1:38457";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitBackup { pub proxy: Option<String>, pub ssl_ca_info: Option<String> }

pub fn git_available() -> bool {
    crate::proc::command("git").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn get_config(key: &str) -> Option<String> {
    let out = crate::proc::command("git").args(["config", "--global", "--get", key]).output().ok()?;
    if out.status.success() { Some(String::from_utf8_lossy(&out.stdout).trim().to_string()) } else { None }
}

pub fn read_backup() -> GitBackup {
    GitBackup { proxy: get_config(PROXY_KEY), ssl_ca_info: get_config(SSL_KEY) }
}

pub fn config_args(key: &str, value: Option<&str>) -> Vec<String> {
    let mut args = vec!["config".to_string(), "--global".to_string()];
    match value {
        Some(v) => { args.push(key.to_string()); args.push(v.to_string()); }
        None => { args.push("--unset".to_string()); args.push(key.to_string()); }
    }
    args
}

fn run_args(args: &[String]) -> Result<()> {
    crate::proc::command("git").args(args).status().context("运行 git 失败")?;
    Ok(())
}

pub fn apply(cacert: &Path) -> Result<()> {
    run_args(&config_args(PROXY_KEY, Some(PROXY_VALUE)))?;
    run_args(&config_args(SSL_KEY, Some(&cacert.to_string_lossy())))?;
    Ok(())
}

pub fn restore(backup: &GitBackup) -> Result<()> {
    run_args(&config_args(PROXY_KEY, backup.proxy.as_deref()))?;
    run_args(&config_args(SSL_KEY, backup.ssl_ca_info.as_deref()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn set_args() {
        assert_eq!(config_args("k", Some("v")), vec!["config","--global","k","v"]);
    }
    #[test]
    fn unset_args() {
        assert_eq!(config_args("k", None), vec!["config","--global","--unset","k"]);
    }
}