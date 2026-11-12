//! Win32 基本类型、结构体、通用常量与小工具函数。

use std::ffi::c_void;

use super::kernel::MultiByteToWideChar;

pub type BOOL = i32;
pub type DWORD = u32;
pub type UINT = u32;
pub type HWND = isize;
pub type HANDLE = isize;
pub type HKEY = isize;
pub type LPCWSTR = *const u16;
pub type LPWSTR = *mut u16;
pub type LPVOID = *mut c_void;

pub const TRUE: BOOL = 1;

pub const ERROR_SUCCESS: DWORD = 0;
pub const ERROR_ACCESS_DENIED: DWORD = 5;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct SYSTEMTIME {
    pub wYear: u16,
    pub wMonth: u16,
    pub wDayOfWeek: u16,
    pub wDay: u16,
    pub wHour: u16,
    pub wMinute: u16,
    pub wSecond: u16,
    pub wMilliseconds: u16,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct FILETIME {
    pub dwLowDateTime: u32,
    pub dwHighDateTime: u32,
}

impl FILETIME {
    pub fn to_u64(&self) -> u64 {
        ((self.dwHighDateTime as u64) << 32) | self.dwLowDateTime as u64
    }
    pub fn from_u64(v: u64) -> Self {
        FILETIME {
            dwLowDateTime: v as u32,
            dwHighDateTime: (v >> 32) as u32,
        }
    }
}

#[repr(C)]
pub struct HIGHCONTRASTW {
    pub cbSize: UINT,
    pub dwFlags: DWORD,
    pub lpszDefault: *mut u16,
}

/// UTF-8 → NUL 结尾的 UTF-16。调用方需持有返回值后再取 `.as_ptr()`。
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 把控制台程序输出的 ANSI（中文系统是 GBK/936）字节转成 String。
/// 直接用 `from_utf8_lossy` 会得到一堆 `����`，排查错误时完全没法看。
pub fn ansi_to_string(bytes: &[u8]) -> String {
    unsafe {
        let n = MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, std::ptr::null_mut(), 0);
        if n <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }
        let mut buf = vec![0u16; n as usize];
        let m = MultiByteToWideChar(0, 0, bytes.as_ptr(), bytes.len() as i32, buf.as_mut_ptr(), n);
        if m <= 0 {
            return String::from_utf8_lossy(bytes).to_string();
        }
        String::from_utf16_lossy(&buf[..m as usize])
    }
}
