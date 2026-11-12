//! 全局共享状态：命令、调度线程、托盘事件都要拿到同一份配置和同一个 AppHandle。

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::AppHandle;

use crate::config::Config;

pub struct AppState {
    pub cfg: Mutex<Config>,
    pub path: PathBuf,
}

/// 全局 AppHandle：托盘事件 / 单实例信号 / 调度线程都要用它拿窗口。
static APP_HANDLE: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

pub fn init_handle(h: AppHandle) {
    let _ = APP_HANDLE.set(h);
}

pub fn handle() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}
