//! 进程枚举（Toolhelp32）与提权拉起（shell32）。

use super::kernel::{CloseHandle, SetErrorMode};
use super::types::*;

pub const TH32CS_SNAPPROCESS: DWORD = 0x0000_0002;

#[repr(C)]
#[derive(Clone)]
pub struct PROCESSENTRY32W {
    pub dwSize: DWORD,
    pub cntUsage: DWORD,
    pub th32ProcessID: DWORD,
    pub th32DefaultHeapID: usize,
    pub th32ModuleID: DWORD,
    pub cntThreads: DWORD,
    pub th32ParentProcessID: DWORD,
    pub pcPriClassBase: i32,
    pub dwFlags: DWORD,
    pub szExeFile: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    pub fn CreateToolhelp32Snapshot(dwFlags: DWORD, th32ProcessID: DWORD) -> HANDLE;
    pub fn Process32FirstW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
    pub fn Process32NextW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
}

#[link(name = "shell32")]
extern "system" {
    pub fn ShellExecuteW(
        hwnd: HWND,
        lpOperation: LPCWSTR,
        lpFile: LPCWSTR,
        lpParameters: LPCWSTR,
        lpDirectory: LPCWSTR,
        nShowCmd: i32,
    ) -> isize;
}

fn exe_name(entry: &PROCESSENTRY32W) -> String {
    let len = entry
        .szExeFile
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(entry.szExeFile.len());
    String::from_utf16_lossy(&entry.szExeFile[..len])
}

/// 枚举快照里的进程，`want` 为 None 时返回全部。
fn scan_processes(want: Option<u32>) -> Vec<PROCESSENTRY32W> {
    let mut out = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == 0 || snap == -1 {
            return out;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as DWORD;
        let mut ok = Process32FirstW(snap, &mut entry);
        while ok != 0 {
            if want.map_or(true, |w| entry.th32ProcessID == w) {
                out.push(entry.clone());
                if want.is_some() {
                    break;
                }
            }
            ok = Process32NextW(snap, &mut entry);
        }
        CloseHandle(snap);
    }
    out
}

/// 谁启动了我们？Low IL 是「继承」来的，查父进程最直接：
/// 父进程若是 explorer.exe，说明双击链路本身有问题；
/// 若是某个终端 / 沙箱宿主，那就换了它再启动一次。
pub fn parent_process() -> Option<(u32, String)> {
    let me = scan_processes(Some(std::process::id())).pop()?;
    let parent_id = me.th32ParentProcessID;
    let name = scan_processes(Some(parent_id))
        .pop()
        .map(|e| exe_name(&e))
        .unwrap_or_default();
    Some((parent_id, name))
}

/// 枚举同名进程（按 exe 文件名比较），返回第一个不是自己的 `(pid, exe 名)`。
///
/// Toolhelp 枚举不需要打开目标进程，所以跨完整性级别也能看见 —— 这正是互斥量
/// 打不开时唯一可靠的「是否已有实例」判断。（Low IL 下高权限进程的名字会是空的，
/// 所以只用来打日志，不用来做判定。）
pub fn find_same_name_process() -> Option<(u32, String)> {
    let exe = std::env::current_exe().ok()?;
    let want = exe.file_name()?.to_string_lossy().to_lowercase();
    let me = std::process::id();
    scan_processes(None)
        .into_iter()
        .map(|e| (e.th32ProcessID, exe_name(&e)))
        .find(|(pid, name)| *pid != me && name.to_lowercase() == want)
}

/// 压掉子进程启动失败时系统弹出的模态错误框。
///
/// 默认情况下 `CreateProcess` 失败（如 0xc0000142 DLL 初始化失败）
/// Windows 会弹一个「应用程序无法正常启动」的系统对话框，把用户挡住。
/// `SEM_FAILCRITICALERRORS` 让失败静默返回错误码，子进程同样继承这个设置。
pub fn suppress_child_error_dialogs() {
    const SEM_FAILCRITICALERRORS: UINT = 0x0001;
    const SEM_NOGPFAULTERRORBOX: UINT = 0x0002;
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    }
}

/// 用 `runas` 动词重新拉起自己（会弹一次 UAC）。
pub fn run_elevated(args: &str) -> Result<(), String> {
    unsafe {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let file = wide(&exe.to_string_lossy());
        let op = wide("runas");
        let params = wide(args);
        // SW_SHOWNORMAL = 1；返回值 > 32 才算成功
        let r = ShellExecuteW(0, op.as_ptr(), file.as_ptr(), params.as_ptr(), std::ptr::null(), 1);
        if r > 32 {
            Ok(())
        } else {
            Err(format!("提权请求被拒绝或取消（ShellExecuteW 返回 {r}）"))
        }
    }
}
