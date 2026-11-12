//! 前端可调用的命令。
//!
//! 规矩：凡是会起子进程、写注册表、发 `WM_SETTINGCHANGE`（内部最多等 5 秒）
//! 或走网络的，一律 `async` + `spawn_blocking`。Tauri 的同步命令跑在窗口主线程上，
//! 堵一下就是整个界面假死 —— 保存时的卡顿就是这么来的。

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::config::Config;
use crate::scheduler::sync_autostart;
use crate::state::AppState;
use crate::{autostart_admin, diag, geo, schedule, theme, win};

#[derive(Serialize)]
pub struct Status {
    dark: Option<bool>,
    system_dark: Option<bool>,
    high_contrast: bool,
    next: Option<String>,
    now_state: Option<String>,
    /// 进程完整性级别；0x1000 = Low，此时注册表写入必然被拒
    integrity: Option<u32>,
    /// 当前是否已注册「最高权限」计划任务
    autostart_task: bool,
    /// 当前是否还有 HKCU\Run 自启项
    autostart_runkey: bool,
}

#[derive(Serialize)]
pub struct GeoDto {
    lat: f64,
    lon: f64,
    source: String,
}

/// 坐标保留 4 位小数（≈11 米）。完整精度在输入框里会被截断，
/// 而对日出日落的影响远小于 1 秒。
fn tidy_coord(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

#[tauri::command]
pub fn get_config(state: State<AppState>) -> Config {
    state.cfg.lock().map(|g| g.clone()).unwrap_or_default()
}

#[tauri::command]
pub async fn save_config(
    mut cfg: Config,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    cfg.lat = tidy_coord(cfg.lat);
    cfg.lon = tidy_coord(cfg.lon);
    cfg.save(&state.path)?;
    if let Ok(mut guard) = state.cfg.lock() {
        *guard = cfg.clone();
    }

    // 下面全是同步阻塞操作：schtasks 子进程、注册表写入、WM_SETTINGCHANGE 广播。
    // 放在阻塞线程池里跑，绝不能占着窗口主线程——否则保存时界面会假死。
    tauri::async_runtime::spawn_blocking(move || {
        let t0 = std::time::Instant::now();
        let auto_note = sync_autostart(&app, &cfg);
        diag::log(&format!(
            "保存：自启同步耗时 {} ms（在阻塞线程池，不占主线程）",
            t0.elapsed().as_millis()
        ));

        // 立即按新时间表纠正一次
        if let Some(target) = schedule::current_target(&cfg) {
            let want = target == schedule::Target::Dark;
            if theme::is_dark() != Some(want) {
                let _ = theme::set_theme(want, cfg.system_ui);
            }
        }

        let next = schedule::next_switch(&cfg)
            .map(|s| s.describe())
            .unwrap_or_else(|| "无".to_string());
        format!("{next}；{auto_note}")
    })
    .await
    .map_err(|e| format!("保存时后台任务异常: {e}"))
}

#[tauri::command]
pub async fn apply_theme(dark: bool, state: State<'_, AppState>) -> Result<(), String> {
    let system_ui = state.cfg.lock().map(|g| g.system_ui).unwrap_or(true);
    // set_theme 里有 5 秒超时的 SendMessageTimeoutW，必须离开主线程
    tauri::async_runtime::spawn_blocking(move || theme::set_theme(dark, system_ui))
        .await
        .map_err(|e| format!("切换主题失败: {e}"))?
}

#[tauri::command]
pub async fn get_status(app: AppHandle, state: State<'_, AppState>) -> Result<Status, String> {
    let cfg = state.cfg.lock().map(|g| g.clone()).unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || Status {
        dark: theme::is_dark(),
        system_dark: theme::is_system_dark(),
        high_contrast: theme::high_contrast(),
        next: schedule::next_switch(&cfg).map(|s| s.describe()),
        now_state: schedule::current_target(&cfg).map(|t| t.label().to_string()),
        integrity: win::integrity_level(),
        // 这两项要查计划任务（起子进程）和注册表，也是阻塞的
        autostart_task: autostart_admin::task_exists(),
        autostart_runkey: app.autolaunch().is_enabled().unwrap_or(false),
    })
    .await
    .map_err(|e| format!("读取状态失败: {e}"))
}

#[tauri::command]
pub async fn fetch_geo(state: State<'_, AppState>) -> Result<GeoDto, String> {
    // 走 HTTP，可能几秒，必须离开主线程
    let fetched = tauri::async_runtime::spawn_blocking(geo::fetch)
        .await
        .map_err(|e| format!("定位线程异常: {e}"))?;
    let g = fetched?;

    if let Ok(mut cfg) = state.cfg.lock() {
        cfg.lat = g.lat;
        cfg.lon = g.lon;
        cfg.coords_known = true;
        cfg.geo_updated = schedule::now_unix();
        cfg.lat = tidy_coord(cfg.lat);
        cfg.lon = tidy_coord(cfg.lon);
        let _ = cfg.save(&state.path);
    }
    Ok(GeoDto {
        lat: tidy_coord(g.lat),
        lon: tidy_coord(g.lon),
        source: g.source,
    })
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn enable_admin_autostart(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 已提权：直接把任务建好，顺手清掉 Run 键与旧配置
    if win::is_elevated() {
        let msg = tauri::async_runtime::spawn_blocking(autostart_admin::create_task)
            .await
            .map_err(|e| format!("创建任务线程异常: {e}"))??;
        let _ = app.autolaunch().disable();
        win::delete_run_value(autostart_admin::RUN_VALUE);
        if let Ok(mut cfg) = state.cfg.lock() {
            cfg.autostart = true;
            cfg.autostart_admin = true;
            let _ = cfg.save(&state.path);
        }
        return Ok(msg);
    }

    // 未提权：拉起一个提权副本去建任务，本次会话继续
    win::run_elevated("--enable-admin-autostart")?;
    Ok("已发起管理员授权请求，请在弹出的 UAC 窗口点『是』；完成后再点一次『保存』即可看到状态"
        .to_string())
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
