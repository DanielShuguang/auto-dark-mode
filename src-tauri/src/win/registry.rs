//! advapi32 注册表：主题键读写所需的全部原语。

use super::types::*;

pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001u32 as i64 as HKEY;
pub const KEY_QUERY_VALUE: DWORD = 0x0001;
pub const KEY_SET_VALUE: DWORD = 0x0002;
pub const REG_SZ: DWORD = 1;
pub const REG_DWORD: DWORD = 4;

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(
        hKey: HKEY,
        lpSubKey: LPCWSTR,
        ulOptions: DWORD,
        samDesired: DWORD,
        phkResult: *mut HKEY,
    ) -> DWORD;
    pub fn RegSetValueExW(
        hKey: HKEY,
        lpValueName: LPCWSTR,
        Reserved: DWORD,
        dwType: DWORD,
        lpData: *const u8,
        cbData: DWORD,
    ) -> DWORD;
    pub fn RegQueryValueExW(
        hKey: HKEY,
        lpValueName: LPCWSTR,
        lpReserved: *mut DWORD,
        lpType: *mut DWORD,
        lpData: *mut u8,
        lpcbData: *mut DWORD,
    ) -> DWORD;
    pub fn RegCloseKey(hKey: HKEY) -> DWORD;
    pub fn RegDeleteValueW(hKey: HKEY, lpValueName: LPCWSTR) -> DWORD;
}

/// 删掉 HKCU\...\Run 下的自启项 —— 改用计划任务后必须清掉它，否则开机会启动两次。
pub fn delete_run_value(name: &str) -> bool {
    unsafe {
        let mut key: HKEY = 0;
        let sub = wide(r"Software\Microsoft\Windows\CurrentVersion\Run");
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_SET_VALUE, &mut key) != 0 {
            return false;
        }
        let value = wide(name);
        let rc = RegDeleteValueW(key, value.as_ptr());
        RegCloseKey(key);
        rc == 0
    }
}
