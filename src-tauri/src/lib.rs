//! Tauri 后端入口：模块装配、单实例、`setup`。
//!
//! 具体实现按职责分在同目录的几个模块里：
//! - `state`     —— 共享状态（配置互斥锁 + 全局 AppHandle）
//! - `window`    —— 主窗口懒创建、跨权限唤醒
//! - `tray`      —— 托盘图标与菜单
//! - `scheduler` —— 调度线程、开机自启同步
//! - `commands`  —— 前端可调用的命令
//! - `win`       —— 手写 Win32 FFI（按 DLL 再分子模块）

mod autostart_admin;
mod civil;
mod commands;
mod config;
mod diag;
mod geo;
mod schedule;
mod scheduler;
mod state;
mod sun;
mod theme;
mod tray;
mod win;
mod window;

use std::sync::Mutex;

use tauri::Manager;

use config::Config;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 第一件事：让子进程启动失败时静默返回，而不是弹系统的错误模态框
    win::suppress_child_error_dialogs();

    // 提权辅助模式：只负责注册计划任务，做完立即退出（不建窗口、不起托盘）
    if std::env::args().any(|a| a == "--enable-admin-autostart") {
        let outcome = autostart_admin::create_task();
        match &outcome {
            Ok(msg) => {
                win::delete_run_value(autostart_admin::RUN_VALUE);
                scheduler::mark_admin_autostart_in_config();
                diag::log(msg);
            }
            Err(e) => diag::log(&format!("启用管理员自启失败: {e}")),
        }
        std::process::exit(if outcome.is_ok() { 0 } else { 1 });
    }

    // 单实例：第二个实例把已有窗口调出来，然后直接退出，避免多个托盘图标 + 多个调度线程
    if !win::ensure_single_instance("AutoDarkMode 设置") {
        diag::log("已有实例在运行，已请求其显示窗口，本次启动退出");
        std::process::exit(0);
    }
    // 先把「显示窗口」命名事件建好，第二个实例双击图标时才有地方发信号
    win::create_show_event();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            // 开机自启时带这个参数，让窗口保持隐藏、静默挂机
            Some(vec!["--background"]),
        ))
        .setup(|app| {
            diag::log(&format!(
                "==== 启动 pid={} exe={}",
                std::process::id(),
                std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            ));
            match win::integrity_level() {
                Some(l) => diag::log(&format!(
                    "进程完整性级别: {} (0x{l:X}) —— Low 时注册表写入必然被拒",
                    win::integrity_label(l)
                )),
                None => diag::log("进程完整性级别: 读取失败"),
            }
            match win::parent_process() {
                Some((pid, name)) => diag::log(&format!("父进程: {name} (pid {pid})")),
                None => diag::log("父进程: 未知"),
            }

            // 配置目录解析失败不应阻止启动（例如受限账户 / 沙箱环境）
            let path = match app.path().app_config_dir() {
                Ok(dir) => dir.join("config.json"),
                Err(e) => {
                    diag::log(&format!("无法解析配置目录（{e}），改用临时目录"));
                    std::env::temp_dir().join("autodarkmode-config.json")
                }
            };
            let first_run = !path.exists();
            let cfg = Config::load(&path);
            // 启动那一刻应当处于的状态（模式为"关闭"时是 None）
            let startup_target = schedule::current_target(&cfg);
            let system_ui = cfg.system_ui;
            diag::log(&format!(
                "配置: {} (首次运行={first_run}, autostart={}, mode={:?})",
                path.display(),
                cfg.autostart,
                cfg.mode
            ));

            // 让系统启动项与配置保持一致。会调 schtasks 起子进程，放后台线程，
            // 否则窗口要等它跑完才出来。
            {
                let handle = app.handle().clone();
                let snapshot = cfg.clone();
                std::thread::spawn(move || {
                    let note = scheduler::sync_autostart(&handle, &snapshot);
                    diag::log(&format!("自启: {note}"));
                });
            }

            app.manage(AppState {
                cfg: Mutex::new(cfg),
                path,
            });
            state::init_handle(app.handle().clone());

            // 启动即纠正一次：开机时若已过切换点，不能等到下一个边界才变。
            // （例如 21:00 开机，19:00 变暗、次日 07:00 变亮 —— 期望立刻是深色）
            if let Some(t) = startup_target {
                let want = t == schedule::Target::Dark;
                if theme::is_dark() == Some(want) {
                    diag::log(&format!("启动时已是{}，无需纠正", t.label()));
                } else {
                    match theme::set_theme(want, system_ui) {
                        Ok(()) => diag::log(&format!("启动时纠正为{}", t.label())),
                        Err(e) => diag::log(&format!("启动时纠正失败: {e}")),
                    }
                }
            }

            // 窗口不在这里建：后台挂机时不预建（省内存 + 避开登录瞬间 WebView2 失败），
            // 交给 window::show_main 按需创建；即使创建失败也只是记录，不会在 setup 里 panic。

            // 托盘失败（如 Explorer 未就绪 / 权限受限）只记录，不中断启动。
            // ADM_NO_TRAY=1 可临时跳过，便于排查与窗口无关的问题。
            if std::env::var("ADM_NO_TRAY").is_err() {
                match tray::build_tray(app.handle()) {
                    Ok(()) => diag::log("托盘图标已创建"),
                    Err(e) => diag::log(&format!("托盘图标创建失败: {e}")),
                }
            } else {
                diag::log("ADM_NO_TRAY 已设置，跳过托盘");
            }

            scheduler::spawn_scheduler(app.handle().clone());

            // 单实例信号：另一个实例双击桌面图标时，用它把窗口叫出来
            win::wait_show_event(|| {
                if let Some(h) = state::handle() {
                    window::show_main(h);
                }
            });
            window::spawn_show_watcher(app.handle().clone());

            // 开机自启带 --background 时静默挂机：不预建窗口，首次点托盘 / 双击图标时再建。
            let background = std::env::args().any(|a| a == "--background");
            if background && !first_run {
                diag::log("窗口策略: 静默挂机（不预建窗口，点托盘时创建）");
            } else {
                window::show_main(app.handle());
                diag::log(&format!(
                    "窗口策略: 显示{}",
                    if first_run { "（首次运行）" } else { "" }
                ));
            }

            diag::log("setup 完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::apply_theme,
            commands::get_status,
            commands::fetch_geo,
            commands::set_autostart,
            commands::get_autostart,
            commands::enable_admin_autostart,
            commands::hide_window,
            commands::quit_app
        ])
        .run(tauri::generate_context!())
        .expect("AutoDarkMode 启动失败");
}
