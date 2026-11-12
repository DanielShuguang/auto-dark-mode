//! 主题读写。
//!
//! Windows 没有公开的主题切换 API，唯一可行路径是写
//! `HKCU\...\Themes\Personalize` 下两个 DWORD 再广播 WM_SETTINGCHANGE。
//! 两个键都在 HKCU，全程不需要管理员权限。

use crate::win;

const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
const V_APPS: &str = "AppsUseLightTheme";
const V_SYSTEM: &str = "SystemUsesLightTheme";

fn open_key(access: u32) -> Result<win::HKEY, u32> {
    let mut h: win::HKEY = 0;
    let sub = win::wide(PERSONALIZE);
    let rc = unsafe { win::RegOpenKeyExW(win::HKEY_CURRENT_USER, sub.as_ptr(), 0, access, &mut h) };
    if rc == win::ERROR_SUCCESS {
        Ok(h)
    } else {
        Err(rc)
    }
}

unsafe fn read_dword(h: win::HKEY, name: &str) -> Option<u32> {
    let n = win::wide(name);
    let mut ty = 0u32;
    let mut buf = [0u8; 4];
    let mut sz: u32 = 4;
    let rc = win::RegQueryValueExW(
        h,
        n.as_ptr(),
        std::ptr::null_mut(),
        &mut ty,
        buf.as_mut_ptr(),
        &mut sz,
    );
    if rc == win::ERROR_SUCCESS && ty == win::REG_DWORD && sz >= 4 {
        Some(u32::from_le_bytes(buf))
    } else {
        None
    }
}

/// 应用是否为深色（AppsUseLightTheme == 0）
pub fn is_dark() -> Option<bool> {
    let h = open_key(win::KEY_QUERY_VALUE).ok()?;
    let v = unsafe { read_dword(h, V_APPS) };
    unsafe { win::RegCloseKey(h) };
    v.map(|x| x == 0)
}

/// 任务栏/开始菜单是否为深色
pub fn is_system_dark() -> Option<bool> {
    let h = open_key(win::KEY_QUERY_VALUE).ok()?;
    let v = unsafe { read_dword(h, V_SYSTEM) };
    unsafe { win::RegCloseKey(h) };
    v.map(|x| x == 0)
}

/// 高对比度模式下必须放弃切换，否则会破坏无障碍设置
pub fn high_contrast() -> bool {
    let mut hc = win::HIGHCONTRASTW {
        cbSize: std::mem::size_of::<win::HIGHCONTRASTW>() as u32,
        dwFlags: 0,
        lpszDefault: std::ptr::null_mut(),
    };
    let ok = unsafe {
        win::SystemParametersInfoW(
            win::SPI_GETHIGHCONTRAST,
            hc.cbSize,
            &mut hc as *mut _ as win::LPVOID,
            0,
        )
    };
    ok != 0 && (hc.dwFlags & win::HCF_HIGHCONTRASTON) != 0
}

/// 应用主题。先写应用键、再写系统键、最后广播（顺序反了会留下
/// “应用已变、任务栏没变”的中间态）。
pub fn set_theme(dark: bool, system_ui: bool) -> Result<(), String> {
    if high_contrast() {
        return Err("系统处于高对比度模式，已跳过本次切换".into());
    }

    let h = open_key(win::KEY_SET_VALUE).map_err(|rc| {
        if rc == win::ERROR_ACCESS_DENIED {
            "写入主题注册表被拒绝（ACCESS_DENIED）".to_string()
        } else {
            format!("无法打开主题注册表键 rc={rc}")
        }
    })?;

    let value: u32 = if dark { 0 } else { 1 };
    let bytes = value.to_le_bytes();

    let apps = win::wide(V_APPS);
    let rc_apps = unsafe { win::RegSetValueExW(h, apps.as_ptr(), 0, win::REG_DWORD, bytes.as_ptr(), 4) };

    if system_ui {
        let system = win::wide(V_SYSTEM);
        let rc_sys = unsafe { win::RegSetValueExW(h, system.as_ptr(), 0, win::REG_DWORD, bytes.as_ptr(), 4) };
        if rc_sys != win::ERROR_SUCCESS {
            eprintln!("警告：写入 SystemUsesLightTheme 失败 rc={rc_sys}");
        }
    }

    unsafe { win::RegCloseKey(h) };

    if rc_apps != win::ERROR_SUCCESS {
        return Err(format!("写入 AppsUseLightTheme 失败 rc={rc_apps}"));
    }

    broadcast();
    Ok(())
}

/// 必须用 SendMessageTimeout：任一个 hung 窗口都会让 SendMessage 永久阻塞
pub fn broadcast() {
    let payload = win::wide("ImmersiveColorSet");
    let mut result: usize = 0;
    unsafe {
        win::SendMessageTimeoutW(
            win::HWND_BROADCAST,
            win::WM_SETTINGCHANGE,
            0,
            payload.as_ptr() as isize,
            win::SMTO_ABORTIFHUNG,
            5000,
            &mut result,
        );
    }
}
