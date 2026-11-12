//! 调度线程与开机自启同步。

use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::config::Config;
use crate::state::AppState;
use crate::{autostart_admin, diag, schedule, theme, win};

// ---------------------------------------------------------------- 开机自启

/// 按配置在「普通 HKCU\Run」与「计划任务（最高权限）」之间同步。
///
/// 原则：**绝不把用户搞成没有自启**。管理员任务需要提权才能创建，
/// 当前没提权且任务不存在时，先退回普通自启，等用户在界面上点授权。
pub fn sync_autostart(app: &AppHandle, cfg: &Config) -> String {
    let al = app.autolaunch();

    if !cfg.autostart {
        let _ = al.disable();
        win::delete_run_value(autostart_admin::RUN_VALUE);
        let _ = autostart_admin::delete_task();
        return "已关闭开机自启".to_string();
    }

    if cfg.autostart_admin {
        let created = if win::is_elevated() {
            match autostart_admin::create_task() {
                Ok(_) => true,
                Err(e) => {
                    diag::log(&format!("创建计划任务失败: {e}"));
                    false
                }
            }
        } else {
            autostart_admin::task_exists()
        };

        if created {
            // 两条路径同时存在会开机启动两次，必须清掉 Run 键
            let _ = al.disable();
            win::delete_run_value(autostart_admin::RUN_VALUE);
            return "管理员自启已启用（计划任务，最高权限，不弹 UAC）".to_string();
        }

        // 还没授权：退回普通自启，保证功能可用
        let _ = al.enable();
        return "管理员自启待授权（暂用普通自启）".to_string();
    }

    let _ = autostart_admin::delete_task();
    let _ = al.enable();
    "普通自启已启用（HKCU\\Run，不提权）".to_string()
}

/// 提权实例（`--enable-admin-autostart`）用：直接改配置文件里的开关。
pub fn mark_admin_autostart_in_config() {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let path = PathBuf::from(appdata)
            .join("com.autodarkmode.desktop")
            .join("config.json");
        let mut cfg = Config::load(&path);
        cfg.autostart = true;
        cfg.autostart_admin = true;
        let _ = cfg.save(&path);
    }
}

// ---------------------------------------------------------------- 调度

/// 每分钟级粒度的调度循环：睡到下一个切换点，到点再切换。
pub fn spawn_scheduler(app: AppHandle) {
    std::thread::spawn(move || loop {
        let (wait_secs, due) = {
            let state = app.state::<AppState>();
            let cfg = state.cfg.lock().map(|g| g.clone()).unwrap_or_default();
            match schedule::next_switch(&cfg) {
                Some(next) => {
                    // 最多睡到边界；远于 5 分钟就先醒来重算一次
                    let delta = (next.at - schedule::now_unix()).clamp(1, 300);
                    (delta, Some(next))
                }
                None => (300, None),
            }
        };

        std::thread::sleep(Duration::from_secs(wait_secs as u64));

        if let Some(next) = due {
            if schedule::now_unix() >= next.at {
                let system_ui = {
                    let state = app.state::<AppState>();
                    state.cfg.lock().map(|g| g.system_ui).unwrap_or(true)
                };
                let want_dark = next.target == schedule::Target::Dark;
                match theme::set_theme(want_dark, system_ui) {
                    Ok(()) => {
                        let _ = app.emit("theme-applied", next.describe());
                    }
                    Err(e) => eprintln!("自动切换失败: {e}"),
                }
            }
        }
    });
}
