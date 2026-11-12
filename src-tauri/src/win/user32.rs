//! user32：WM_SETTINGCHANGE 广播、高对比度探测、窗口查找与显示。

use super::types::*;

pub const WM_SETTINGCHANGE: UINT = 0x001A;
pub const HWND_BROADCAST: HWND = 0xFFFF;
pub const SMTO_ABORTIFHUNG: UINT = 0x0002;

pub const SPI_GETHIGHCONTRAST: UINT = 0x0042;
pub const HCF_HIGHCONTRASTON: DWORD = 0x0001;

#[link(name = "user32")]
extern "system" {
    pub fn SendMessageTimeoutW(
        hWnd: HWND,
        Msg: UINT,
        wParam: usize,
        lParam: isize,
        fuFlags: UINT,
        uTimeout: UINT,
        lpdwResult: *mut usize,
    ) -> isize;
    pub fn SystemParametersInfoW(uiAction: UINT, uiParam: UINT, pvParam: LPVOID, fWinIni: UINT) -> BOOL;
    pub fn FindWindowW(lpClassName: LPCWSTR, lpWindowName: LPCWSTR) -> HWND;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
}
