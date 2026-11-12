//! 日出日落：sunrise equation（NOAA 简化式），纯数学、零依赖、无需网络。
//!
//! 太阳高度角取 -0.833°（含 34′ 折射 + 16′ 太阳视半径），
//! 中纬度精度约 ±1~2 分钟。

use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;
const RAD: f64 = 180.0 / PI;
const JD_UNIX_EPOCH: f64 = 2_440_587.5;
const JD_J2000: f64 = 2_451_545.0;
const HORIZON: f64 = -0.833;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polar {
    Day,
    Night,
}

#[derive(Debug, Clone, Copy)]
pub struct SunTimes {
    /// UTC Unix 秒
    pub sunrise: Option<i64>,
    pub sunset: Option<i64>,
    pub polar: Option<Polar>,
}

fn sin_d(x: f64) -> f64 {
    (x * DEG).sin()
}
fn cos_d(x: f64) -> f64 {
    (x * DEG).cos()
}

/// 计算某个日期（距 1970-01-01 的天数）的日出日落
pub fn sun_times(days_since_epoch: i64, lat: f64, lon: f64) -> SunTimes {
    let lat = lat.clamp(-89.9, 89.9);

    let jd0 = JD_UNIX_EPOCH + days_since_epoch as f64;
    let n = (jd0 - JD_J2000 + 0.0008).round();

    let j_star = n - lon / 360.0;
    let m = (357.5291 + 0.985_600_28 * j_star).rem_euclid(360.0);
    let c = 1.9148 * sin_d(m) + 0.0200 * sin_d(2.0 * m) + 0.0003 * sin_d(3.0 * m);
    let lambda = (m + c + 180.0 + 102.9372).rem_euclid(360.0);

    let j_transit = JD_J2000 + j_star + 0.0053 * sin_d(m) - 0.0069 * sin_d(2.0 * lambda);

    let sin_dec = sin_d(lambda) * sin_d(23.44);
    let cos_dec = (1.0 - sin_dec * sin_dec).sqrt();
    if cos_dec.abs() < 1e-12 {
        return SunTimes {
            sunrise: None,
            sunset: None,
            polar: Some(if sin_d(lat) * sin_dec > 0.0 {
                Polar::Day
            } else {
                Polar::Night
            }),
        };
    }

    let cos_omega = (sin_d(HORIZON) - sin_d(lat) * sin_dec) / (cos_d(lat) * cos_dec);

    if cos_omega < -1.0 {
        return SunTimes { sunrise: None, sunset: None, polar: Some(Polar::Day) };
    }
    if cos_omega > 1.0 {
        return SunTimes { sunrise: None, sunset: None, polar: Some(Polar::Night) };
    }

    let omega0 = cos_omega.acos() * RAD;
    SunTimes {
        sunrise: Some(jd_to_unix(j_transit - omega0 / 360.0)),
        sunset: Some(jd_to_unix(j_transit + omega0 / 360.0)),
        polar: None,
    }
}

fn jd_to_unix(jd: f64) -> i64 {
    ((jd - JD_UNIX_EPOCH) * 86_400.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::civil::days_from_civil;

    fn local_minutes(unix_utc: i64, tz_hours: i64) -> i64 {
        ((unix_utc + tz_hours * 3600).rem_euclid(86_400)) / 60
    }

    #[test]
    fn shanghai_equinox() {
        let d = days_from_civil(2026, 3, 20);
        let t = sun_times(d, 31.2304, 121.4737);
        let rise = local_minutes(t.sunrise.unwrap(), 8);
        let set = local_minutes(t.sunset.unwrap(), 8);
        assert!((rise - 360).abs() < 20, "sunrise {rise}");
        assert!((set - 1080).abs() < 25, "sunset {set}");
    }

    #[test]
    fn winter_day_length() {
        let d = days_from_civil(2026, 12, 21);
        let t = sun_times(d, 31.2304, 121.4737);
        let hours = (t.sunset.unwrap() - t.sunrise.unwrap()) as f64 / 3600.0;
        assert!(hours > 9.5 && hours < 11.0, "day {hours}h");
    }

    #[test]
    fn polar() {
        let june = sun_times(days_from_civil(2026, 6, 21), 78.2, 15.6);
        assert_eq!(june.polar, Some(Polar::Day));
        let dec = sun_times(days_from_civil(2026, 12, 21), 78.2, 15.6);
        assert_eq!(dec.polar, Some(Polar::Night));
    }

    #[test]
    fn southern_hemisphere() {
        let t = sun_times(days_from_civil(2026, 6, 21), -33.8688, 151.2093);
        let hours = (t.sunset.unwrap() - t.sunrise.unwrap()) as f64 / 3600.0;
        assert!(hours > 9.5 && hours < 10.5, "day {hours}h");
    }
}
