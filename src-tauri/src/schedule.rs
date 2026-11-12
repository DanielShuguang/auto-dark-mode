//! 调度：把"下一次切换"算成一个绝对 Unix 时刻。
//!
//! 每次都从头重算，不缓存 UTC 偏移——夏令时、改时区、手动改系统时间都不会错位。

use crate::civil;
use crate::config::{Config, Mode};
use crate::sun;
use crate::win;

const SEC_1601_TO_1970: i64 = 11_644_473_600;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Dark,
    Light,
}

impl Target {
    pub fn label(self) -> &'static str {
        match self {
            Target::Dark => "深色",
            Target::Light => "浅色",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Switch {
    /// Unix 秒
    pub at: i64,
    pub target: Target,
    /// 用于展示的本地时刻
    pub local: win::SYSTEMTIME,
}

impl Switch {
    pub fn describe(&self) -> String {
        format!(
            "{:02}:{:02} → {}",
            self.local.wHour,
            self.local.wMinute,
            self.target.label()
        )
    }
}

// ---------------------------------------------------------------- 时间换算

pub fn now_unix() -> i64 {
    let mut st = win::SYSTEMTIME::default();
    let mut ft = win::FILETIME::default();
    unsafe {
        win::GetSystemTime(&mut st);
        win::SystemTimeToFileTime(&st, &mut ft);
    }
    (ft.to_u64() / 10_000_000) as i64 - SEC_1601_TO_1970
}

fn local_now() -> win::SYSTEMTIME {
    let mut st = win::SYSTEMTIME::default();
    unsafe { win::GetLocalTime(&mut st) };
    st
}

/// 本地墙钟 → Unix 秒（用当前时区规则，含夏令时）
fn st_to_unix(st: &win::SYSTEMTIME) -> Option<i64> {
    let mut utc = win::SYSTEMTIME::default();
    let mut ft = win::FILETIME::default();
    let ok = unsafe { win::TzSpecificLocalTimeToSystemTime(std::ptr::null_mut(), st, &mut utc) };
    if ok == 0 {
        return None;
    }
    let ok = unsafe { win::SystemTimeToFileTime(&utc, &mut ft) };
    if ok == 0 {
        return None;
    }
    Some((ft.to_u64() / 10_000_000) as i64 - SEC_1601_TO_1970)
}

fn unix_to_local(unix: i64) -> win::SYSTEMTIME {
    let ticks = ((unix + SEC_1601_TO_1970).max(0) as u64) * 10_000_000;
    let ft = win::FILETIME::from_u64(ticks);
    let mut utc = win::SYSTEMTIME::default();
    let mut local = win::SYSTEMTIME::default();
    unsafe {
        win::FileTimeToSystemTime(&ft, &mut utc);
        if win::SystemTimeToTzSpecificLocalTime(std::ptr::null_mut(), &utc, &mut local) == 0 {
            local = utc;
        }
    }
    local
}

fn make_local(y: i32, m: u32, d: u32, hour: u32, minute: u32) -> win::SYSTEMTIME {
    win::SYSTEMTIME {
        wYear: y as u16,
        wMonth: m as u16,
        wDayOfWeek: 0,
        wDay: d as u16,
        wHour: hour as u16,
        wMinute: minute as u16,
        wSecond: 0,
        wMilliseconds: 0,
    }
}

fn today_days() -> i64 {
    let n = local_now();
    civil::days_from_civil(n.wYear as i32, n.wMonth as u32, n.wDay as u32)
}

// ---------------------------------------------------------------- 边界

fn boundaries(cfg: &Config, base_days: i64, offsets: &[i64]) -> Vec<Switch> {
    let mut out = Vec::new();
    if cfg.mode == Mode::Off {
        return out;
    }

    let coords = cfg.coords();
    let use_sun = cfg.mode == Mode::Sun && coords.is_some();

    for &off in offsets {
        let days = base_days + off;
        let (y, m, d) = civil::civil_from_days(days);
        let wd = civil::weekday_from_days(days);

        if use_sun {
            let (lat, lon) = coords.unwrap();
            let t = sun::sun_times(days, lat, lon);
            if t.polar.is_some() {
                continue; // 极昼/极夜：当日无切换
            }
            for (secs, target, adj) in [
                (t.sunrise, Target::Light, cfg.sunrise_offset),
                (t.sunset, Target::Dark, cfg.sunset_offset),
            ] {
                if let Some(s) = secs {
                    let at = s + adj as i64 * 60;
                    out.push(Switch { at, target, local: unix_to_local(at) });
                }
            }
        } else if let Some((dark_min, light_min)) = cfg.times_for_day(wd) {
            // 日出日落模式若尚无坐标，退回固定时间表
            for (min, target) in [(dark_min, Target::Dark), (light_min, Target::Light)] {
                let st = make_local(y, m, d, min / 60, min % 60);
                if let Some(at) = st_to_unix(&st) {
                    out.push(Switch { at, target, local: unix_to_local(at) });
                }
            }
        }
    }
    out.sort_by_key(|s| s.at);
    out
}

/// 下一次切换
pub fn next_switch(cfg: &Config) -> Option<Switch> {
    let now = now_unix();
    boundaries(cfg, today_days(), &[-1, 0, 1, 2])
        .into_iter()
        .find(|s| s.at > now)
}

/// 按时间表"现在"应处于哪种状态，用于启动/改配置后立即纠正
pub fn current_target(cfg: &Config) -> Option<Target> {
    let now = now_unix();
    boundaries(cfg, today_days(), &[-2, -1, 0, 1])
        .into_iter()
        .rfind(|s| s.at <= now)
        .map(|s| s.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_roundtrip() {
        let st = make_local(2026, 11, 9, 19, 0);
        let unix = st_to_unix(&st).expect("转换失败");
        let back = unix_to_local(unix);
        assert_eq!(
            (back.wYear, back.wMonth, back.wDay, back.wHour, back.wMinute),
            (2026, 11, 9, 19, 0)
        );
    }

    #[test]
    fn now_is_sane() {
        // 2024-01-01 之后、2100 年之前
        let n = now_unix();
        assert!(n > 1_704_067_200 && n < 4_102_444_800);
    }
}
