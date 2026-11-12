//! 少量手写 Win32 FFI，按依赖的 DLL 拆成子模块，统一从 `win::` 根上 re-export。
//!
//! 只保留应用真正需要的东西：注册表读写、WM_SETTINGCHANGE 广播、WinHTTP 定位、
//! 本地/UTC 时间换算、完整性级别探测、单实例与「显示窗口」信号。
//! 其余（托盘、窗口、自启）交给 Tauri。

#![allow(non_snake_case, non_camel_case_types, dead_code)]

mod http;
mod kernel;
mod proc;
mod registry;
mod single;
mod token;
mod types;
mod user32;

pub use http::*;
pub use kernel::*;
pub use proc::*;
pub use registry::*;
pub use single::*;
pub use token::*;
pub use types::*;
pub use user32::*;
