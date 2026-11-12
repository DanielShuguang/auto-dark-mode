//! 单实例：互斥量 + 跨完整性级别的「显示窗口」信号。
//!
//! 三种唤醒通道并存，因为没有任何一种能覆盖所有情况：
//! 1. 同权限级别 → `FindWindowW` 直接 ShowWindow；
//! 2. 跨权限级别 → 命名事件 `Local\AutoDarkMode.ShowWindow`；
//! 3. 事件也打不开（High 实例创建的事件对象）→ `%TEMP%` 请求文件，由对方轮询。

use std::sync::atomic::{AtomicUsize, Ordering};

use super::kernel::*;
use super::proc::find_same_name_process;
use super::types::*;
use super::user32::*;

const ERROR_ALREADY_EXISTS: DWORD = 183;
const SHOW_EVENT_NAME: &str = "Local\\AutoDarkMode.ShowWindow";
const EVENT_MODIFY_STATE: DWORD = 0x0002;
const WAIT_INFINITE: DWORD = 0xFFFF_FFFF;

static SHOW_EVENT: AtomicUsize = AtomicUsize::new(0);

/// 保证只有一个实例：第二个实例把已有窗口唤到前台，然后自己退出。
///
/// 拿到锁时**故意不关闭句柄**——它要活到进程结束，系统会在退出时回收。
/// 关键细节：`CreateMutexW` 成功时不会清零 LastError，所以必须先 `SetLastError(0)`，
/// 否则会读到上一个 API 残留的 183，误判成「已有实例」（这个坑之前在自绘托盘版本里踩过）。
pub fn ensure_single_instance(window_title: &str) -> bool {
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;

    unsafe {
        let name = wide("Local\\AutoDarkMode.SingleInstance");
        SetLastError(0);
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        let err = GetLastError();
        if handle == 0 {
            // 建不出互斥量最常见的原因：已有实例以更高的完整性级别在跑（开机自启的
            // 计划任务是 Highest）。低权限进程连它的内核对象都打不开，直接 ACCESS_DENIED。
            // 这个名字只有我们自己用，所以「被拒绝」基本等于「已有实例」。
            crate::diag::log(&format!("单实例: 互斥量创建失败 (err={err})"));
            if err == ERROR_ACCESS_DENIED {
                if let Some((pid, exe)) = find_same_name_process() {
                    crate::diag::log(&format!("单实例: 已有实例 {exe} (pid {pid})"));
                }
                request_show();
                return false;
            }
            return true;
        }
        if err != ERROR_ALREADY_EXISTS {
            crate::diag::log(&format!("单实例: 已获得锁 (err={err})"));
            return true;
        }
        crate::diag::log("单实例: 互斥量已存在（同权限级别的实例）");

        // 已有窗口就直接唤到前台；窗口还没建（后台挂机 / 创建失败）就发信号，
        // 让已有实例自己把窗口建出来再显示。
        let title = wide(window_title);
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            ShowWindow(hwnd, SW_SHOW);
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        } else {
            signal_show();
        }
        // 事件信号可能因为跨完整性级别而失败，文件信号兜底（高权限实例会轮询）
        request_show();
        false
    }
}

/// 跨完整性级别的「显示窗口」请求文件。
///
/// 低权限进程既不能给高权限窗口发消息（UIPI 拦截），也打不开高权限实例的内核对象
/// （MAC 拦截，`CreateMutexW` 直接返回 ERROR_ACCESS_DENIED），但同一个用户的
/// `%TEMP%` 两边都写得进去 —— 于是用文件时间戳当信号，由高权限实例轮询。
pub fn show_request_path() -> std::path::PathBuf {
    std::env::temp_dir().join("autodarkmode-show.req")
}

/// 请求已有实例显示窗口（不管它在哪个完整性级别）。
pub fn request_show() {
    let _ = std::fs::write(show_request_path(), b"show");
}

/// 单实例赢家调用：建好命名事件，留给 `wait_show_event` 等信号。
pub fn create_show_event() {
    unsafe {
        let name = wide(SHOW_EVENT_NAME);
        // 手动重置 + 初始未触发
        let h = CreateEventW(std::ptr::null(), 1, 0, name.as_ptr());
        SHOW_EVENT.store(h as usize, Ordering::SeqCst);
    }
}

/// 第二个实例调用：通知已有实例显示窗口。返回是否送达。
pub fn signal_show() -> bool {
    unsafe {
        let name = wide(SHOW_EVENT_NAME);
        let h = OpenEventW(EVENT_MODIFY_STATE, 0, name.as_ptr());
        if h == 0 {
            return false;
        }
        let ok = SetEvent(h) != 0;
        CloseHandle(h);
        ok
    }
}

/// 已有实例调用：起一个线程等信号，收到后执行 `f`（可重复触发）。
pub fn wait_show_event<F>(f: F)
where
    F: Fn() + Send + 'static,
{
    let h = SHOW_EVENT.load(Ordering::SeqCst) as HANDLE;
    if h == 0 {
        return;
    }
    std::thread::spawn(move || loop {
        let r = unsafe { WaitForSingleObject(h, WAIT_INFINITE) };
        if r != 0 {
            break;
        }
        unsafe {
            ResetEvent(h);
        }
        f();
    });
}
