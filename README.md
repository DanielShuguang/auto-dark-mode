# AutoDarkMode

Windows 11 自动深浅色切换：按固定时间或本地日出日落切换，托盘常驻、开机自启、无需管理员权限。

**技术栈：Tauri 2 + Vue 3 + TypeScript + UnoCSS**（Rust 后端负责注册表/调度，Vue 负责设置界面，
样式全部用 UnoCSS 原子类，配色走 CSS 变量）。

---

## 开发与构建

首次需要一个能联网的终端（要下 npm 包和 Rust crate）：

```powershell
cd E:\workspace\desktop-app\auto-dark-mode
pnpm install
pnpm tauri dev      # 开发模式
pnpm tauri build    # 出安装包（NSIS）
```

`pnpm tauri dev` 会自动：生成图标 → 起 Vite → 编译 Rust → 打开桌面窗口。

⚠️ 出独立 exe 时**别直接 `cargo build --release`**。Tauri 用 cargo feature
`custom-protocol` 决定"内嵌 dist/ 资源"还是"去连 devUrl"，
`pnpm tauri build` 会自动带上它；手动构建必须写全：

```powershell
pnpm tauri build --no-bundle
# 或者
cd src-tauri; cargo build --release --features custom-protocol
```

漏掉这个 feature 的话 exe 能启动、能建窗口、托盘也有，但界面只会显示
`127.0.0.1 拒绝连接`——因为它在找 dev server。

单独跑后端测试（注册表、调度、日出日落算法）：

```powershell
cd src-tauri
cargo test
```

---

## 目录

```
src/                      Vue 前端（设置界面）
  App.vue                 装配壳，只负责把各卡片拼起来
  components/             各设置卡片：模式切换 / 时间 / 日出日落 / 自启 / 状态 / 页脚
  composables/useApp.ts   invoke 状态与动作（模块级单例）
  styles/base.css         主题变量 + 最小重置（UnoCSS 不自带 preflight）
  api.ts                  invoke 封装 + 类型
uno.config.ts             UnoCSS：颜色映射 CSS 变量 + 组件级 shortcuts
src-tauri/                Rust 后端
  src/lib.rs              入口：run() + setup
  src/window.rs           主窗口按需创建 / 唤出（懒创建，省内存）
  src/tray.rs             托盘图标与菜单
  src/commands.rs         前端可调用的 IPC 命令
  src/scheduler.rs        调度线程 + 自启同步
  src/autostart_admin.rs  计划任务自启（最高权限，开机不弹 UAC）
  src/state.rs            全局状态
  src/theme.rs            注册表读写 + WM_SETTINGCHANGE 广播
  src/schedule.rs         下一次切换时刻计算
  src/sun.rs              日出日落（sunrise equation）
  src/civil.rs            历法换算
  src/geo.rs              WinHTTP IP 定位
  src/config.rs           配置（JSON）+ 校验
  src/diag.rs             日志 + WebView2 数据目录探测
  src/win/                手写 Win32 FFI（types/kernel/user32/registry/token/proc/single/http）
  tauri.conf.json         窗口 / 打包配置
  capabilities/           权限声明
scripts/gen-icons.mjs     零依赖图标生成（PNG + ICO）
docs/adr/                 技术决策记录
```

---

## 功能

- 固定时间切换，支持周末单独时间表
- 按本地日出日落切换（NOAA 算法，含 ±分钟偏移、极昼/极夜处理）
- IP 自动定位 + 手动经纬度
- 托盘菜单：设置 / 立即深色 / 立即浅色 / 退出
- 关闭窗口 = 隐藏到托盘，不退出
- 开机自启（默认建「最高权限」计划任务，开机**不弹 UAC**；未授权时退回 HKCU Run 普通自启）
- 应用自身界面跟随系统深浅色，不闪白
- 高对比度模式下自动暂停并提示

配置存于 `%APPDATA%\com.autodarkmode.desktop\config.json`。

---

## 实现要点

**主题切换**：Windows 没有公开 API，只能写
`HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize`
下的 `AppsUseLightTheme` / `SystemUsesLightTheme`，再广播
`SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, 0, L"ImmersiveColorSet", ...)`。
顺序必须是「应用键 → 系统键 → 广播」，否则会留下任务栏没跟上的中间态。
两个键都在 HKCU，所以**全程不需要管理员权限**。

**样式**：UnoCSS（`presetWind3`），颜色不写死而是映射到 CSS 变量
（`--bg/--card/--field/--text/--muted/--line/--accent`），所以深色模式只是
`<html data-theme="dark">` 换一组变量，**不需要 `dark:` 前缀逐条写两套**。
重复的卡片/按钮样式收在 `uno.config.ts` 的 shortcuts（`panel`/`row`/`btn-primary`…），
模板里只写语义名。注意 UnoCSS 不带 preflight，`base.css` 里补了
`box-sizing: border-box`，否则「`h-full` + padding」会把页脚挤出窗口。

**调度**：每次重算「下一次切换」不缓存 UTC 偏移，夏令时/改时区/手动改时间都不会错位。
后台线程按边界精确休眠（最多睡 5 分钟就重新评估一次，兼顾系统休眠后的恢复）。

**为什么不用手工 Win32 写整个应用**：上一版用零依赖手写 Win32 实现，
单文件体积和内存确实更小，但托盘、设置窗口、单实例、DPI 这些样板代码
量大且容易出错，改用 Tauri 后这部分交给框架，只在必要处（注册表、广播）
保留少量 FFI。

---

## 故障排查

启动过程会写日志到 `%TEMP%\autodarkmode.log`（临时目录不可写时落到 exe 同级目录）。
窗口起不来、托盘不出来、切换报 `ACCESS_DENIED`，先看它。日志第一行之后会打印
`进程完整性级别`——**如果是 Low，注册表写入必然被拒**，那不是应用的问题，
而是被父进程的受限令牌降权了（沙箱、AppContainer、部分终端工具都会这样）。
把它放到资源管理器里双击，或用普通 Windows Terminal 启动即可。

| 环境变量 | 作用 |
| --- | --- |
| `ADM_NO_TRAY=1` | 跳过托盘，用于区分"托盘问题"和"窗口问题" |

### `Failed to setup app: 拒绝访问。(os error 5)`

Tauri 自动建窗口时，WebView2 数据目录固定为 `%LOCALAPPDATA%\<标识符>`，
它会对这个目录做 `create_dir_all`；一旦返回 `ERROR_ACCESS_DENIED`，Tauri 直接
`panic!`，而且**发生在任何用户代码之前**——没有日志、没有输出，只有一行 panic。

所以本应用在 [tauri.conf.json](<src-tauri/tauri.conf.json>) 里把窗口标为
`"create": false`，改由 `create_main_window()` 自己创建，并按
`%LOCALAPPDATA%\com.autodarkmode.desktop` → `%TEMP%\autodarkmode-webview` →
`<exe 目录>\webview-data` 的顺序探测第一个**真正可写**的目录作为数据目录。
正常桌面环境第一个就成功，行为与默认一致；受限环境（沙箱、受控文件夹访问、
受限令牌）则自动降级，不再直接崩溃——失败原因写进日志。

## 已知风险

- `bundle.targets` 设为 `nsis`，需要 WiX/NSIS 工具链；仅开发可忽略。
- IP 定位会把出口 IP 发给第三方服务（ipwho.is / ip-api.com），
  不接受时改用手动坐标。

## 许可

MIT
