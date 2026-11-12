//! kernel32：时间换算、错误码、句柄、互斥量 / 事件、多字节转换。

use std::ffi::c_void;

use super::types::*;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetSystemTime(lpSystemTime: *mut SYSTEMTIME);
    pub fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
    pub fn SystemTimeToFileTime(lpSystemTime: *const SYSTEMTIME, lpFileTime: *mut FILETIME) -> BOOL;
    pub fn FileTimeToSystemTime(lpFileTime: *const FILETIME, lpSystemTime: *mut SYSTEMTIME) -> BOOL;
    pub fn TzSpecificLocalTimeToSystemTime(
        lpTimeZoneInformation: LPVOID,
        lpLocalTime: *const SYSTEMTIME,
        lpUniversalTime: *mut SYSTEMTIME,
    ) -> BOOL;
    pub fn SystemTimeToTzSpecificLocalTime(
        lpTimeZoneInformation: LPVOID,
        lpUniversalTime: *const SYSTEMTIME,
        lpLocalTime: *mut SYSTEMTIME,
    ) -> BOOL;

    pub fn GetLastError() -> DWORD;
    pub fn SetLastError(dwErrCode: DWORD);
    pub fn GetCurrentProcess() -> HANDLE;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;

    /// 设置本进程的「错误对话框模式」，子进程会继承。
    /// 用它压掉 schtasks 等子进程启动失败时系统弹出的
    /// 「应用程序无法正常启动(0xc0000142)」模态框。
    pub fn SetErrorMode(uMode: UINT) -> UINT;

    pub fn MultiByteToWideChar(
        CodePage: UINT,
        dwFlags: DWORD,
        lpMultiByteStr: *const u8,
        cbMultiByte: i32,
        lpWideCharStr: *mut u16,
        cchWideChar: i32,
    ) -> i32;

    pub fn CreateMutexW(
        lpMutexAttributes: *const c_void,
        bInitialOwner: BOOL,
        lpName: LPCWSTR,
    ) -> HANDLE;
    pub fn CreateEventW(
        lpEventAttributes: *const c_void,
        bManualReset: BOOL,
        bInitialState: BOOL,
        lpName: LPCWSTR,
    ) -> HANDLE;
    pub fn OpenEventW(dwDesiredAccess: DWORD, bInheritHandle: BOOL, lpName: LPCWSTR) -> HANDLE;
    pub fn SetEvent(hEvent: HANDLE) -> BOOL;
    pub fn ResetEvent(hEvent: HANDLE) -> BOOL;
    pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
}
