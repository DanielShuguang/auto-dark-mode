//! winhttp：取 IP 定位只需要 GET 一个 JSON 接口，所以直接走最原始的同步调用。

use super::types::*;

pub const WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY: DWORD = 4;
pub const WINHTTP_FLAG_SECURE: DWORD = 0x0080_0000;

#[link(name = "winhttp")]
extern "system" {
    pub fn WinHttpOpen(
        pszAgentW: LPCWSTR,
        dwAccessType: DWORD,
        pszProxyW: LPCWSTR,
        pszProxyBypassW: LPCWSTR,
        dwFlags: DWORD,
    ) -> HANDLE;
    pub fn WinHttpConnect(hSession: HANDLE, pswzServerName: LPCWSTR, nServerPort: u16, dwReserved: DWORD) -> HANDLE;
    pub fn WinHttpOpenRequest(
        hConnect: HANDLE,
        pwszVerb: LPCWSTR,
        pwszObjectName: LPCWSTR,
        pwszVersion: LPCWSTR,
        pwszReferrer: LPCWSTR,
        ppwszAcceptTypes: *const LPCWSTR,
        dwFlags: DWORD,
    ) -> HANDLE;
    pub fn WinHttpSendRequest(
        hRequest: HANDLE,
        lpszHeaders: LPCWSTR,
        dwHeadersLength: DWORD,
        lpOptional: LPVOID,
        dwOptionalLength: DWORD,
        dwTotalLength: DWORD,
        dwContext: usize,
    ) -> BOOL;
    pub fn WinHttpReceiveResponse(hRequest: HANDLE, lpReserved: LPVOID) -> BOOL;
    pub fn WinHttpReadData(
        hRequest: HANDLE,
        lpBuffer: LPVOID,
        dwNumberOfBytesToRead: DWORD,
        lpdwNumberOfBytesRead: *mut DWORD,
    ) -> BOOL;
    pub fn WinHttpCloseHandle(hInternet: HANDLE) -> BOOL;
    pub fn WinHttpSetTimeouts(
        hInternet: HANDLE,
        nResolveTimeout: i32,
        nConnectTimeout: i32,
        nSendTimeout: i32,
        nReceiveTimeout: i32,
    ) -> BOOL;
}
