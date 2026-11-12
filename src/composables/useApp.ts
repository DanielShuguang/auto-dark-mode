import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { api, type Config, type Status } from "../api";

/**
 * 全局单例状态：整个界面只读写这一份，规模还不需要 Pinia。
 * 模块级 ref 在多个组件间天然共享，改一处处处生效。
 */
const cfg = ref<Config | null>(null);
const status = ref<Status | null>(null);
const busy = ref(false);
const message = ref("");

export { cfg, status, busy, message };

let msgTimer: number | null = null;

/** 当前生效的自启方式，直接读系统的真实状态而不是配置里的意图 */
export const autostartLabel = computed(() => {
  const s = status.value;
  if (!s) return "未知";
  if (s.autostart_task) return "计划任务（最高权限，开机不弹 UAC）";
  if (s.autostart_runkey) return "普通启动项（不提权）";
  return "未启用";
});

export function flash(text: string) {
  message.value = text;
  if (msgTimer !== null) window.clearTimeout(msgTimer);
  msgTimer = window.setTimeout(() => {
    if (message.value === text) message.value = "";
  }, 4000);
}

/**
 * 把界面主题切到指定值。参数取的是「注册表里真实生效的主题」，
 * 而不是系统偏好——应用自己就是主题的切换者，两者必须一致。
 */
export function applyUiTheme(isDark: boolean | null | undefined) {
  document.documentElement.dataset.theme = isDark ? "dark" : "light";
}

export async function refreshStatus() {
  try {
    status.value = await api.status();
    applyUiTheme(status.value.dark);
  } catch {
    /* 状态刷新失败不影响操作 */
  }
}

export async function load() {
  try {
    const c = await api.getConfig();
    // 自启的真实生效方式有两种：HKCU Run 键 或 计划任务（最高权限）。
    // 只用 plugin 的 is_enabled 会漏掉计划任务，导致勾选框显示为未勾选。
    const [runkey, s] = await Promise.all([
      api.getAutostart().catch(() => false),
      api.status().catch(() => null),
    ]);
    c.autostart = runkey || !!s?.autostart_task || !!s?.autostart_runkey;
    cfg.value = c;
    await refreshStatus();
  } catch (e) {
    flash(`读取配置失败：${e}`);
  }
}

export async function save() {
  if (!cfg.value) return;
  busy.value = true;
  try {
    const next = await api.saveConfig({ ...cfg.value });
    await refreshStatus();
    // 后端会把坐标规整到 4 位小数（见 commands.rs），重新读一次才不会显示旧值
    try {
      cfg.value = await api.getConfig();
    } catch {
      /* 读不回来就保留界面上的值，不影响已保存的结果 */
    }
    flash(`已保存，下次切换：${next}`);
  } catch (e) {
    flash(`保存失败：${e}`);
  } finally {
    busy.value = false;
  }
}

export async function apply(dark: boolean) {
  try {
    await api.applyTheme(dark);
    await refreshStatus();
    flash(dark ? "已切换为深色" : "已切换为浅色");
  } catch (e) {
    flash(`切换失败：${e}`);
  }
}

export async function doGeo() {
  if (!cfg.value) return;
  busy.value = true;
  try {
    const g = await api.fetchGeo();
    cfg.value.lat = Number(g.lat.toFixed(4));
    cfg.value.lon = Number(g.lon.toFixed(4));
    cfg.value.coords_known = true;
    flash(`定位成功（${g.source}）：${g.lat.toFixed(4)}, ${g.lon.toFixed(4)}`);
  } catch (e) {
    flash(`定位失败：${e}`);
  } finally {
    busy.value = false;
  }
}

/** 请求一次提权，创建「最高权限」的计划任务作为开机自启。 */
export async function enableAdmin() {
  busy.value = true;
  try {
    flash(await api.enableAdminAutostart());
  } catch (e) {
    flash(`启用失败：${e}`);
  } finally {
    busy.value = false;
  }
}

export function hide() {
  api.hideWindow().catch(() => undefined);
}

/// 关闭窗口只是隐藏到托盘；要真正结束进程只有这里和托盘菜单的「退出」。
export function quit() {
  api.quit().catch(() => undefined);
}

/**
 * 在 App.vue 里调用一次：绑定事件、首帧主题、30 秒轮询。
 */
export function useAppLifecycle() {
  let unlisten: UnlistenFn | null = null;
  let unlistenTheme: UnlistenFn | null = null;
  let mq: MediaQueryList | null = null;
  let timer: number | null = null;

  onMounted(async () => {
    // 首帧先用系统偏好垫一下，随后被真实状态覆盖
    applyUiTheme(window.matchMedia("(prefers-color-scheme: dark)").matches);
    await load();
    applyUiTheme(status.value?.dark);

    unlisten = await listen<string>("theme-applied", (e) => {
      flash(`已按计划切换到 ${e.payload}`);
      refreshStatus();
    });
    // 后端在收到 WM_SETTINGCHANGE 时会推这个事件（用户手动改系统主题也能跟上）
    unlistenTheme = await listen<boolean>("system-theme", (e) => {
      applyUiTheme(e.payload);
      refreshStatus();
    });
    // 兜底：WebView2 自己也会感知系统配色变化，触发一次状态刷新
    mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", refreshStatus);

    timer = window.setInterval(refreshStatus, 30_000);
  });

  onUnmounted(() => {
    unlisten?.();
    unlistenTheme?.();
    mq?.removeEventListener("change", refreshStatus);
    if (timer !== null) window.clearInterval(timer);
  });
}
