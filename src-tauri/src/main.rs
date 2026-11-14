#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use fastgithub_gui_lib::{
    state::{AppState, ToggleAction},
    backend::PlatformBackend,
    paths, kernel, gitcfg,
};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    image::Image,
    Manager,
};

#[cfg(target_os = "macos")]
use fastgithub_gui_lib::backend::macos::MacBackend as Backend;
#[cfg(target_os = "linux")]
use fastgithub_gui_lib::backend::linux::LinuxBackend as Backend;
#[cfg(target_os = "windows")]
use fastgithub_gui_lib::backend::windows::WindowsBackend as Backend;

struct TrayHandles { status: MenuItem<tauri::Wry>, toggle: MenuItem<tauri::Wry> }

fn set_state(app: &tauri::AppHandle, s: AppState) {
    let label = s.label();
    let on = matches!(s, AppState::On);
    let busy = matches!(s, AppState::Starting | AppState::Stopping);
    {
        let st = app.state::<Arc<Mutex<AppState>>>();
        *st.lock().unwrap() = s;
    }
    if let Some(h) = app.try_state::<TrayHandles>() {
        let _ = h.status.set_text(&label);
        let _ = h.toggle.set_text(if on { "关闭加速" } else { "开启加速" });
        let _ = h.toggle.set_enabled(!busy);
    }
    if let Some(tray) = app.tray_by_id("main") {
        let icon = if on { Image::from_bytes(include_bytes!("../icons/tray-on.png")) }
                   else { Image::from_bytes(include_bytes!("../icons/tray-off.png")) };
        if let Ok(img) = icon { let _ = tray.set_icon(Some(img)); }
    }
}

fn dialog(app: &tauri::AppHandle, msg: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!("display dialog {} buttons {{\"好\"}} default button \"好\"",
            format!("\"{}\"", msg.replace('\\', "\\\\").replace('"', "\\\"")));
        let _ = std::process::Command::new("osascript").arg("-e").arg(script).output();
    }
    #[cfg(target_os = "windows")]
    {
        let body = msg.replace('\'', "''");
        let script = format!("(New-Object -ComObject WScript.Shell).Popup('{body}',0,'FastGithub GUI',0) | Out-Null");
        let _ = std::process::Command::new("powershell").args(["-NoProfile", "-Command", &script]).output();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if std::process::Command::new("zenity").args(["--info", "--text", msg]).output().is_err() {
            let _ = std::process::Command::new("notify-send").args(["FastGithub GUI", msg]).output();
        }
    }
    let _ = app;
}

fn finish(app: &tauri::AppHandle, r: anyhow::Result<()>, success: AppState) {
    match r {
        Ok(_) => set_state(app, success),
        Err(e) => { let msg = format!("{e:#}"); set_state(app, AppState::Error(msg.clone())); dialog(app, &msg); }
    }
}

fn handle_menu(app: &tauri::AppHandle, id: &str) {
    let backend = app.state::<Arc<Backend>>().inner().clone();
    match id {
        "toggle" => {
            let action = app.state::<Arc<Mutex<AppState>>>().lock().unwrap().toggle_target();
            let app2 = app.clone();
            match action {
                Some(ToggleAction::Enable) => {
                    set_state(app, AppState::Starting);
                    std::thread::spawn(move || {
                        let p = paths::app_paths();
                        let r: anyhow::Result<()> = (|| {
                            let rd = app2.path().resource_dir()?;
                            let need_install = {
                                #[cfg(target_os = "macos")]
                                {
                                    let helper_src = rd.join("resources").join("helper").join("fastgithub-gui-helper");
                                    !backend.components_installed()
                                        || !fastgithub_gui_lib::macos::install::helper_current(&helper_src)
                                }
                                #[cfg(not(target_os = "macos"))]
                                { !backend.components_installed() }
                            };
                            if need_install { backend.install_components(&rd)?; }
                            backend.enable(&p.kernel_dir)
                        })();
                        finish(&app2, r, AppState::On);
                    });
                }
                Some(ToggleAction::Disable) => {
                    set_state(app, AppState::Stopping);
                    std::thread::spawn(move || { let r = backend.disable(); finish(&app2, r, AppState::Off); });
                }
                None => {}
            }
        }
        "reinstall" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                if let Ok(rd) = app2.path().resource_dir() {
                    if let Err(e) = backend.install_components(&rd) { dialog(&app2, &format!("安装失败：{e:#}")); }
                }
            });
        }
        "uninstall" => {
            let app2 = app.clone();
            std::thread::spawn(move || {
                if let Err(e) = backend.uninstall() { dialog(&app2, &format!("卸载失败：{e:#}")); }
                else { dialog(&app2, "已还原设置并移除后台组件/证书。应用本体可手动删除。"); }
            });
        }
        "quit" => { let _ = backend.disable(); app.exit(0); }
        _ => {}
    }
}

fn spawn_watchdog() {
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(exe) = std::env::current_exe() {
            let pid = std::process::id();
            let state = paths::session_file();
            let _ = std::process::Command::new(exe)
                .args(["--watchdog", &pid.to_string(), &state.to_string_lossy()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--watchdog") {
        let pid: u32 = args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let state = args.get(pos + 2).map(std::path::PathBuf::from).unwrap_or_else(paths::session_file);
        return fastgithub_gui_lib::watchdog::run(pid, &state);
    }

    let backend = Arc::new(Backend::new());
    let app_state = Arc::new(Mutex::new(AppState::Off));
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .manage(app_state.clone())
        .manage(backend.clone())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let p = paths::app_paths();
            let res_kernel = app.path().resource_dir()?.join("resources").join("kernel");
            if kernel::needs_extract(&res_kernel, &p.kernel_dir)? {
                kernel::extract_resources(&res_kernel, &p.kernel_dir)?;
            }

            if backend.status().unwrap_or_else(|_| "off".into()) == "off" && gitcfg::git_available() {
                let backup: gitcfg::GitBackup = std::fs::read_to_string(&p.git_backup_file).ok()
                    .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
                let _ = gitcfg::restore(&backup);
            }

            spawn_watchdog();

            let status = MenuItem::with_id(app, "status", "已关闭", false, None::<&str>)?;
            let toggle = MenuItem::with_id(app, "toggle", "开启加速", true, None::<&str>)?;
            let reinstall = MenuItem::with_id(app, "reinstall", "重新安装组件", true, None::<&str>)?;
            let uninstall = MenuItem::with_id(app, "uninstall", "卸载…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let version = MenuItem::with_id(app, "version", "FastGithub GUI 0.1.0", false, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&status, &toggle, &sep, &reinstall, &uninstall, &quit, &version])?;
            app.manage(TrayHandles { status: status.clone(), toggle: toggle.clone() });

            TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(include_bytes!("../icons/tray-off.png"))?)
                .icon_as_template(false)
                .tooltip("FastGithub GUI")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, e| handle_menu(app, e.id.as_ref()))
                .build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())?
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() { api.prevent_exit(); }
            }
        });
    Ok(())
}