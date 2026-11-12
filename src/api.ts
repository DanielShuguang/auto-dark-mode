import { invoke } from "@tauri-apps/api/core";

export type Mode = "off" | "time" | "sun";
export type LocMode = "auto" | "manual";

export interface Config {
  mode: Mode;
  dark_at: string;
  light_at: string;
  weekend_enabled: boolean;
  weekend_dark_at: string;
  weekend_light_at: string;
  system_ui: boolean;
  sunrise_offset: number;
  sunset_offset: number;
  location: LocMode;
  lat: number;
  lon: number;
  coords_known: boolean;
  geo_updated: number;
  autostart: boolean;
  /** 开机自启是否以管理员权限静默运行（计划任务 + 最高权限） */
  autostart_admin: boolean;
}

export interface Status {
  dark: boolean | null;
  system_dark: boolean | null;
  high_contrast: boolean;
  next: string | null;
  now_state: string | null;
  /** 进程完整性级别；4096 (0x1000) = Low，此时无法写入主题注册表 */
  integrity: number | null;
  /** 已注册「最高权限」计划任务 */
  autostart_task: boolean;
  /** 仍有 HKCU\Run 自启项 */
  autostart_runkey: boolean;
}

export interface Geo {
  lat: number;
  lon: number;
  source: string;
}

export const api = {
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (cfg: Config) => invoke<string>("save_config", { cfg }),
  applyTheme: (dark: boolean) => invoke<void>("apply_theme", { dark }),
  status: () => invoke<Status>("get_status"),
  fetchGeo: () => invoke<Geo>("fetch_geo"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  enableAdminAutostart: () => invoke<string>("enable_admin_autostart"),
  hideWindow: () => invoke<void>("hide_window"),
  quit: () => invoke<void>("quit_app"),
};
