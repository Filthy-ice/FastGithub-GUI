use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub const PLIST: &str = "/Library/LaunchDaemons/com.fastgithub.gui.helper.plist";
pub const HELPER_DEST: &str = "/Library/PrivilegedHelperTools/fastgithub-gui-helper";

pub fn helper_installed() -> bool {
    Path::new(PLIST).exists() && Path::new(HELPER_DEST).exists()
}

pub fn helper_current(resource_helper: &Path) -> bool {
    match (std::fs::read(HELPER_DEST), std::fs::read(resource_helper)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn apple_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn osascript_admin(cmd: &str) -> Result<()> {
    let script = format!("do shell script {} with administrator privileges", apple_quote(cmd));
    let out = Command::new("osascript").arg("-e").arg(script).output().context("运行 osascript 失败")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("-128") { bail!("用户取消了授权"); }
        bail!("授权操作失败: {err}");
    }
    Ok(())
}

fn run_temp_script(script: &str) -> Result<()> {
    let tmp = std::env::temp_dir().join(format!("fastgithub-script-{}.sh", std::process::id()));
    std::fs::write(&tmp, script)?;
    let cmd = format!("/bin/sh '{}'", tmp.display());
    let r = osascript_admin(&cmd);
    let _ = std::fs::remove_file(&tmp);
    r
}

fn plist_content() -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.fastgithub.gui.helper</string>
<key>ProgramArguments</key><array><string>{HELPER_DEST}</string></array>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><true/>
<key>StandardOutPath</key><string>/Library/Application Support/FastGithubGUI/daemon.log</string>
<key>StandardErrorPath</key><string>/Library/Application Support/FastGithubGUI/daemon.log</string>
</dict></plist>"#)
}

pub fn install_helper(resource_helper: &Path) -> Result<()> {
    let src = resource_helper.to_string_lossy();
    let script = format!(
        "set -e\n\
         mkdir -p /Library/PrivilegedHelperTools '/Library/Application Support/FastGithubGUI'\n\
         cp '{src}' '{HELPER_DEST}'\n\
         chmod 755 '{HELPER_DEST}'\n\
         cat > '{PLIST}' <<'PLISTEOF'\n{plist}\nPLISTEOF\n\
         launchctl bootout system '{PLIST}' 2>/dev/null || true\n\
         launchctl bootstrap system '{PLIST}'\n",
        src = src, plist = plist_content()
    );
    run_temp_script(&script)
}

pub fn uninstall_helper() -> Result<()> {
    let script = format!(
        "launchctl bootout system '{PLIST}' 2>/dev/null || true\n\
         rm -f '{PLIST}' '{HELPER_DEST}'\n"
    );
    run_temp_script(&script)
}