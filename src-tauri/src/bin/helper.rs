#[cfg(target_os = "macos")]
fn main() {
    if let Err(e) = fastgithub_gui_lib::macos::daemon::run() {
        eprintln!("daemon 错误: {e:#}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("helper 仅用于 macOS");
    std::process::exit(1);
}