use std::ffi::OsStr;
use std::process::Command;

// Windows 上以 CREATE_NO_WINDOW 启动子进程，避免弹出控制台窗口。
// 看门狗每秒调用 tasklist 检查父进程，若不隐藏窗口会反复闪烁 cmd 窗口。
// 其它平台行为不变。
pub fn command<S: AsRef<OsStr>>(program: S) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn command_runs() {
        let out = command("cmd").args(["/C", "exit", "0"]).output().expect("启动 cmd 失败");
        assert!(out.status.success());
    }
}