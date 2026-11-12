//! 配置：serde JSON，存在 `%APPDATA%\AutoDarkMode\config.json`。

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Off,
    #[default]
    Time,
    Sun,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum LocMode {
    #[default]
    Auto,
    Manual,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub mode: Mode,
    /// "19:00" 形式的本地墙钟
    pub dark_at: String,
    pub light_at: String,
    pub weekend_enabled: bool,
    pub weekend_dark_at: String,
    pub weekend_light_at: String,
    /// 是否同时切换任务栏/开始菜单
    pub system_ui: bool,
    pub sunrise_offset: i32,
    pub sunset_offset: i32,
    pub location: LocMode,
    pub lat: f64,
    pub lon: f64,
    pub coords_known: bool,
    pub geo_updated: i64,
    pub autostart: bool,
    /// 开机自启是否以管理员权限静默运行（计划任务 + 最高权限，不弹 UAC）。
    /// 关闭时退回普通的 HKCU\Run 自启（不提权）。
    #[serde(default = "default_true")]
    pub autostart_admin: bool,
}

pub fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: Mode::Time,
            dark_at: "19:00".into(),
            light_at: "07:00".into(),
            weekend_enabled: false,
            weekend_dark_at: "20:00".into(),
            weekend_light_at: "08:00".into(),
            system_ui: true,
            sunrise_offset: 0,
            sunset_offset: 0,
            location: LocMode::Auto,
            lat: 0.0,
            lon: 0.0,
            coords_known: false,
            geo_updated: 0,
            autostart: true,
            autostart_admin: true,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Config {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("配置解析失败（{e}），使用默认值");
                Config::default()
            }),
            Err(_) => {
                let cfg = Config::default();
                let _ = cfg.save(path);
                cfg
            }
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("创建配置目录失败: {e}"))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| format!("序列化失败: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| format!("写入配置失败: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("替换配置失败: {e}"))?;
        Ok(())
    }

    /// 指定星期的两个切换时刻（自 00:00 的分钟数）。
    /// 仅在固定时间模式下有效；日出日落模式由 schedule 单独处理。
    pub fn times_for_day(&self, weekday: u32) -> Option<(u32, u32)> {
        if self.mode != Mode::Time {
            return None;
        }
        let weekend = self.weekend_enabled && (weekday == 0 || weekday == 6);
        let (d, l) = if weekend {
            (&self.weekend_dark_at, &self.weekend_light_at)
        } else {
            (&self.dark_at, &self.light_at)
        };
        Some((parse_hhmm(d)?, parse_hhmm(l)?))
    }

    pub fn coords(&self) -> Option<(f64, f64)> {
        if self.coords_known {
            Some((self.lat, self.lon))
        } else {
            None
        }
    }
}

/// "19:05" → 1145（分钟）
pub fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hhmm() {
        assert_eq!(parse_hhmm("19:05"), Some(1145));
        assert_eq!(parse_hhmm("7:0"), Some(420));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("19:60"), None);
        assert_eq!(parse_hhmm("abc"), None);
    }
}
