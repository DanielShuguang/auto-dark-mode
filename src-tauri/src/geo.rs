//! IP 兜底定位（WinHTTP，无第三方 crate）。
//!
//! 隐私：会把本机出口 IP 发给第三方定位服务，仅在
//! 「日出日落模式 + 定位=自动 + 坐标过期」时触发。城市级精度对日出日落
//! 影响 < 2 分钟。

use crate::win;

#[derive(Debug, Clone)]
pub struct GeoResult {
    pub lat: f64,
    pub lon: f64,
    pub source: String,
}

pub fn fetch() -> Result<GeoResult, String> {
    // 首选 ipwho.is：HTTPS、免密钥
    if let Ok(body) = http_get("ipwho.is", "/?fields=success,latitude,longitude", 443, true) {
        if !body.contains("\"success\":false") {
            if let (Some(lat), Some(lon)) = (
                number_after(&body, "\"latitude\""),
                number_after(&body, "\"longitude\""),
            ) {
                return Ok(GeoResult { lat, lon, source: "ipwho.is".into() });
            }
        }
    }

    // 兜底 ip-api.com（免费版仅 HTTP）
    let body = http_get("ip-api.com", "/json/?fields=status,lat,lon", 80, false)?;
    if body.contains("\"status\":\"fail\"") {
        return Err("IP 定位服务返回失败".into());
    }
    let lat = number_after(&body, "\"lat\"").ok_or_else(|| "解析 lat 失败".to_string())?;
    let lon = number_after(&body, "\"lon\"").ok_or_else(|| "解析 lon 失败".to_string())?;
    Ok(GeoResult { lat, lon, source: "ip-api.com".into() })
}

fn http_get(host: &str, path: &str, port: u16, secure: bool) -> Result<String, String> {
    unsafe {
        let agent = win::wide("AutoDarkMode/0.1");
        let session = win::WinHttpOpen(
            agent.as_ptr(),
            win::WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            std::ptr::null(),
            std::ptr::null(),
            0,
        );
        if session == 0 {
            return Err("WinHttpOpen 失败".into());
        }
        win::WinHttpSetTimeouts(session, 3000, 3000, 3000, 5000);

        let host_w = win::wide(host);
        let conn = win::WinHttpConnect(session, host_w.as_ptr(), port, 0);
        if conn == 0 {
            win::WinHttpCloseHandle(session);
            return Err("WinHttpConnect 失败".into());
        }

        let verb = win::wide("GET");
        let obj = win::wide(path);
        let flags = if secure { win::WINHTTP_FLAG_SECURE } else { 0 };
        let req = win::WinHttpOpenRequest(
            conn,
            verb.as_ptr(),
            obj.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            flags,
        );
        if req == 0 {
            win::WinHttpCloseHandle(conn);
            win::WinHttpCloseHandle(session);
            return Err("WinHttpOpenRequest 失败".into());
        }

        let body = read_body(req);

        win::WinHttpCloseHandle(req);
        win::WinHttpCloseHandle(conn);
        win::WinHttpCloseHandle(session);

        body
    }
}

unsafe fn read_body(req: win::HANDLE) -> Result<String, String> {
    let ok = win::WinHttpSendRequest(req, std::ptr::null(), 0, std::ptr::null_mut(), 0, 0, 0);
    if ok == 0 {
        return Err("WinHttpSendRequest 失败".into());
    }
    let ok = win::WinHttpReceiveResponse(req, std::ptr::null_mut());
    if ok == 0 {
        return Err("WinHttpReceiveResponse 失败".into());
    }

    let mut body: Vec<u8> = Vec::new();
    loop {
        let mut buf = [0u8; 4096];
        let mut read: u32 = 0;
        let ok = win::WinHttpReadData(
            req,
            buf.as_mut_ptr() as win::LPVOID,
            buf.len() as u32,
            &mut read,
        );
        if ok == 0 || read == 0 {
            break;
        }
        body.extend_from_slice(&buf[..read as usize]);
        if body.len() > 64 * 1024 {
            break;
        }
    }

    if body.is_empty() {
        return Err("响应为空".into());
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// 从 JSON 文本里取 `"key": <number>`
fn number_after(json: &str, key: &str) -> Option<f64> {
    let i = json.find(key)?;
    let mut s = &json[i + key.len()..];
    s = s.trim_start().strip_prefix(':')?.trim_start();
    s = s.trim_start_matches('"');
    let end = s
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E'))
        .unwrap_or(s.len());
    s[..end].parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::number_after;

    #[test]
    fn parse_numbers() {
        let j = r#"{"success":true,"latitude":31.2304,"longitude":121.4737}"#;
        assert_eq!(number_after(j, "\"latitude\""), Some(31.2304));
        assert_eq!(number_after(j, "\"longitude\""), Some(121.4737));

        let j2 = r#"{"lat": -33.8688 , "lon":151.2}"#;
        assert_eq!(number_after(j2, "\"lat\""), Some(-33.8688));
        assert_eq!(number_after(j2, "\"lon\""), Some(151.2));
    }
}
