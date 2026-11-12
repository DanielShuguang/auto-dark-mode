//! 主窗口：懒创建 + 跨权限唤醒。
//!
//! 为什么不让 Tauri 在 `setup()` 里自动建窗口：
//! 1. 自动建时 WebView2 数据目录固定为 `%LOCALAPPDATA%\<标识符>`，`create_dir_all`
//!    失败会以 `Failed to setup app: 拒绝访问。(os error 5)` 直接 panic，且发生在
//!    用户 `setup` 闭包之前，日志都来不及写；
//! 2. 后台挂机时根本不需要窗口，不建就没有 WebView2 进程，内存少一半。

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};

use crate::{diag, theme, win};

/// 显示主窗口：已经建好就叫到前台，还没建就按需创建（失败会退避重试）。
///
/// 开机自启带 `--background` 时我们不预建窗口，所以点托盘的那一刻可能才第一次
/// 建窗口 —— 而登录瞬间 WebView2 还没就绪，实测会返回
/// `failed to create webview: ... HRESULT(0x80070490) 找不到元素`，
/// 因此这里必须重试，而不是一次失败就放弃。
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }

    use std::sync::atomic::{AtomicBool, Ordering};
    static CREATING: AtomicBool = AtomicBool::new(false);
    if CREATING.swap(true, Ordering::SeqCst) {
        return; // 已经有一次创建流程在跑，别重复建
    }

    let handle = app.clone();
    std::thread::spawn(move || {
        let delays = [0u64, 500, 1500, 4000, 8000];
        for (i, delay) in delays.into_iter().enumerate() {
            if delay > 0 {
                std::thread::sleep(Duration::from_millis(delay));
            }
            if handle.get_webview_window("main").is_some() {
                break;
            }
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                // 窗口只能在主线程建；这里再判一次，避免并发重复创建
                if h.get_webview_window("main").is_some() {
                    return;
                }
                match create_main_window(&h) {
                    Ok(w) => {
                        diag::log(&format!("按需创建主窗口成功（第 {} 次）", i + 1));
                        let _ = w.show();
                        let _ = w.unminimize();
                        let _ = w.set_focus();
                    }
                    Err(e) => diag::log(&format!("按需创建主窗口失败（第 {} 次）: {e}", i + 1)),
                }
            });
            std::thread::sleep(Duration::from_millis(400));
        }
        CREATING.store(false, Ordering::SeqCst);
    });
}

/// 自己创建主窗口，显式指定一个探测过可写的 WebView2 数据目录。
pub fn create_main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    let cfg = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
        .ok_or_else(|| "tauri.conf.json 中缺少 label=main 的窗口配置".to_string())?;

    let (dir, notes) = diag::pick_webview_dir();
    for n in &notes {
        diag::log(&format!("WebView 数据目录候选不可用: {n}"));
    }
    diag::log(&format!("WebView2 数据目录: {}", dir.display()));

    let window = WebviewWindowBuilder::from_config(app, &cfg)
        .map_err(|e| format!("from_config: {e}"))?
        .data_directory(dir)
        .build()
        .map_err(|e| format!("build: {e}"))?;

    diag::log(&format!("主窗口已创建: {}", window.label()));

    // 先把窗口底色设成当前主题的底色，避免页面加载完之前闪一下白
    let dark = theme::is_dark().unwrap_or(false);
    let color = if dark {
        tauri::window::Color(0x1b, 0x1d, 0x21, 0xff)
    } else {
        tauri::window::Color(0xf6, 0xf7, 0xf9, 0xff)
    };
    if let Err(e) = window.set_background_color(Some(color)) {
        diag::log(&format!("设置窗口底色失败: {e}"));
    }

    // 关窗口 = 隐藏到托盘；系统主题变化转发给前端，界面立刻跟着变。
    // 注意：这个监听必须绑在窗口创建时，因为窗口是懒创建的（可能在 setup 之后才出现）。
    {
        let w = window.clone();
        let handle = app.clone();
        window.on_window_event(move |event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = w.hide();
            }
            tauri::WindowEvent::ThemeChanged(theme) => {
                let _ = handle.emit("system-theme", *theme == tauri::Theme::Dark);
            }
            _ => {}
        });
    }
    Ok(window)
}

/// 轮询「显示窗口」请求文件：低权限实例没法直接唤醒高权限实例
/// （UIPI 拦消息、MAC 拦内核对象），只能靠这个文件传请求。
/// 每 2 秒一次 stat，开销可以忽略。
pub fn spawn_show_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let path = win::show_request_path();
        let mut last = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        loop {
            std::thread::sleep(Duration::from_millis(2000));
            let now = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            if now.is_some() && now != last {
                last = now;
                diag::log("收到显示窗口请求，正在打开设置界面");
                show_main(&app);
            }
        }
    });
}
