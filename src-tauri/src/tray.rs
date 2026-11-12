//! 托盘图标与右键菜单。
//!
//! 左键单击 = 打开设置窗口（用户最直觉的操作）；右键 = 菜单。

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::theme;
use crate::window::show_main;

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "设置…", true, None::<&str>)?;
    let dark = MenuItem::with_id(app, "dark", "立即切换到深色", true, None::<&str>)?;
    let light = MenuItem::with_id(app, "light", "立即切换到浅色", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let s1 = PredefinedMenuItem::separator(app)?;
    let s2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&show, &s1, &dark, &light, &s2, &quit])?;

    let builder = TrayIconBuilder::with_id("main-tray").menu(&menu);
    let builder = match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone()),
        None => builder,
    };

    builder
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "dark" => {
                let _ = theme::set_theme(true, true);
            }
            "light" => {
                let _ = theme::set_theme(false, true);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}
