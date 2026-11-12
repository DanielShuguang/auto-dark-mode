// release 构建不挂控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    auto_dark_mode_lib::run();
}
