//! 进程令牌：完整性级别与提权状态。
//!
//! 这两个值决定了「写主题注册表会不会被判 ACCESS_DENIED」，
//! 所以界面上要显示它们，出问题时一眼就能看出是权限而不是逻辑。

use super::kernel::{CloseHandle, GetCurrentProcess};
use super::types::*;

#[link(name = "advapi32")]
extern "system" {
    pub fn OpenProcessToken(ProcessHandle: HANDLE, DesiredAccess: DWORD, TokenHandle: *mut HANDLE) -> BOOL;
    pub fn GetTokenInformation(
        TokenHandle: HANDLE,
        TokenInformationClass: u32,
        TokenInformation: LPVOID,
        TokenInformationLength: DWORD,
        ReturnLength: *mut DWORD,
    ) -> BOOL;
    pub fn GetSidSubAuthority(pSid: LPVOID, nSubAuthority: DWORD) -> *mut DWORD;
    pub fn GetSidSubAuthorityCount(pSid: LPVOID) -> *mut u8;
}

const TOKEN_QUERY: DWORD = 0x0008;

/// 进程完整性级别。`ACCESS_DENIED` 类故障的头号嫌疑就是它被降到了 Low。
///
/// 返回 0x1000 Low / 0x2000 Medium / 0x3000 High / 0x4000 System。
pub fn integrity_level() -> Option<u32> {
    const TOKEN_INTEGRITY_LEVEL: u32 = 25;

    #[repr(C)]
    struct SidAndAttributes {
        sid: *mut u8,
        attributes: u32,
    }

    unsafe {
        let mut token: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }
        let mut buf = [0u64; 16]; // 128 字节，足够放 TOKEN_MANDATORY_LABEL
        let mut ret: DWORD = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_INTEGRITY_LEVEL,
            buf.as_mut_ptr() as LPVOID,
            std::mem::size_of_val(&buf) as DWORD,
            &mut ret,
        );
        CloseHandle(token);
        if ok == 0 {
            return None;
        }

        let label: SidAndAttributes = std::ptr::read_unaligned(buf.as_ptr() as *const SidAndAttributes);
        if label.sid.is_null() {
            return None;
        }
        let count = *GetSidSubAuthorityCount(label.sid as LPVOID) as DWORD;
        if count == 0 {
            return None;
        }
        Some(*GetSidSubAuthority(label.sid as LPVOID, count - 1))
    }
}

pub fn integrity_label(level: u32) -> &'static str {
    match level {
        0x0000 => "Untrusted",
        0x1000 => "Low",
        0x2000 => "Medium",
        0x2100 => "Medium-Plus",
        0x3000 => "High",
        0x4000 => "System",
        _ => "Unknown",
    }
}

/// 当前进程是否已经提权（管理员）。
pub fn is_elevated() -> bool {
    const TOKEN_ELEVATION: u32 = 20;

    #[repr(C)]
    struct TokenElevation {
        elevated: DWORD,
    }

    unsafe {
        let mut token: HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut info = TokenElevation { elevated: 0 };
        let mut ret: DWORD = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut info as *mut TokenElevation as LPVOID,
            std::mem::size_of::<TokenElevation>() as DWORD,
            &mut ret,
        );
        CloseHandle(token);
        ok != 0 && info.elevated != 0
    }
}
