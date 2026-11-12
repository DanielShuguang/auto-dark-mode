# ADR-0001：自动深浅色切换应用的平台与技术栈选型

- **状态**：已接受（Accepted）
- **日期**：2026-11-09（2026-11-09 修订：R4 明确为 <30 MB；修正 .NET NativeAOT 支持范围）
- **决策者**：项目负责人
- **目标平台**：Windows 11（22H2 / 23H2 / 24H2 / 25H2）

---

## 1. 需求与约束

| 编号 | 需求 | 强度 | 备注 |
|---|---|---|---|
| R1 | 按设定时间自动切换 Windows 深浅色 | 必须 | 支持工作日/周末不同时间表 |
| R2 | 开机（登录）自动启动 | 必须 | 最终用户零配置 |
| R3 | 后台静默运行，无主窗口 | 必须 | 仅托盘图标 |
| R4 | **后台常驻内存 < 30 MB** | 必须 | 已确认的硬边界（任务管理器"内存"列） |
| R5 | 按定位的日出/日落切换 | 可选 | 加成项，不能拖累 R4 |
| R6 | 免管理员权限 | 强期望 | 避免每次启动弹 UAC |
| R7 | 单文件分发、无运行时依赖 | 强期望 | 绿色便携 |

**推论**：R4 + R3 的组合直接决定了架构形态。这是一个**每天只干活 2 次、每次约 10 毫秒**的"冬眠型"常驻程序。它 99.99% 的生命周期在等待一个定时器，**不渲染任何界面**。

因此，**任何会在后台常驻一个渲染引擎或托管运行时的技术栈都是错的**，与代码写得好不好无关。

---

## 2. 技术现实：Windows 11 没有切换主题的公开 API

这是选型的第二个硬前提，必须先确认，否则后面的架构讨论没有意义。

截至 Windows 11 25H2，微软仍未提供受支持的系统主题切换 API（`Windows.UI.ViewManagement.UISettings` 只能读，不能写）。业界通用做法是**写注册表 + 广播消息**：

```
HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize
  AppsUseLightTheme     REG_DWORD   0=深色 1=浅色   ← 应用深色
  SystemUsesLightTheme  REG_DWORD   0=深色 1=浅色   ← 任务栏/开始/通知中心
```

随后向所有顶层窗口广播：

```c
SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0,
                    (LPARAM)L"ImmersiveColorSet",
                    SMTO_ABORTIFHUNG, 5000, &dwResult);
```

关键结论：

1. **只需纯 Win32 调用，不需要 UI 框架、不需要 COM、不需要管理员权限**（键在 HKCU，按用户生效）。
   → 任何"重型 GUI 框架"都不是为这个功能服务的，只是为设置界面服务。
2. **应用体积与内存的差异，几乎全部来自"设置界面"这一块**，而不来自核心功能。
   → 这正是架构上要做的取舍：**把设置界面从常驻路径里彻底剥离**。
3. 广播是 `PostMessage` 级别的"尽力而为"。Windows 11 上偶发任务栏不刷新，需要兜底策略（见 §6.2）。

---

## 3. 候选技术栈对比

数值为同类应用的**量级估计**（任务管理器"内存"列 / 单 exe 体积），用于比较相对差异，需在实现后用真实测量替换。

| 方案 | 常驻内存 | 分发体积 | 冷启动 | 开发速度 | UI 表现力 | 主要风险 |
|---|---|---|---|---|---|---|
| **纯计划任务，无常驻进程** | **0 MB** | ~0.3 MB | 0 | 快 | 无 | 无托盘、无即时操作、易被精简系统禁用 |
| **Rust + windows-rs（原生 Win32）** | **3–6 MB** | 0.4–1.5 MB | <50 ms | 中慢 | 一般（原生控件） | 需要 Win32 经验 |
| C++ + Win32/MFC | 2–5 MB | 0.2–1 MB | <50 ms | 慢 | 一般 | 内存安全、迭代成本高 |
| C# + 裸 Win32 P/Invoke + NativeAOT | 8–15 MB | 8–14 MB | ~150 ms | 中 | 需手写 | 官方 AOT 仅覆盖控制台/Win32，无 GUI 框架 |
| C# + WinForms / WPF（JIT 或裁剪） | 35–60 MB | 0.3 MB + 运行时 | ~500 ms | **最快** | 优秀 | **WinForms 官方不支持 NativeAOT**（.NET 11 P3 已回归损坏）；JIT 版必然破 30 MB |
| **Tauri v2（懒加载窗口）** | 10–20 MB | 3–6 MB | ~200 ms | 快 | 优秀（Web） | WebView2 进程驻留，需严格懒加载 |
| Tauri v2（常驻 WebView） | 100–180 MB | 3–6 MB | ~400 ms | 快 | 优秀 | 直接违反 R4 |
| Go + systray（cgo） | 12–20 MB | 8–12 MB | ~100 ms | 快 | 一般 | cgo、托盘库质量参差 |
| WinUI 3 / Windows App SDK | 80–150 MB | 需运行时 | ~800 ms | 中 | 优秀 | 严重违反 R4，且打包复杂 |
| Electron | 150–300 MB | 80–120 MB | ~1 s | 最快 | 优秀 | 直接违反 R4 |
| AutoHotkey / PowerShell 脚本 | 8–15 MB | 极小 | 慢 | 快 | 无 | 不适合做正式产品 |

**排序（内存优先）**：纯任务 < C++ ≈ Rust < C# 裸 P/Invoke + AOT ≈ Tauri 懒加载 < Go < WPF/WinForms（JIT）< WinUI3 < Electron

**排序（开发速度优先）**：Electron > C# WPF > Tauri ≈ Go > C++ > Rust

---

## 4. 决策

### 4.1 主方案 A（推荐，默认）

> **Rust（stable）+ `windows` crate，单进程原生 Win32 托盘程序，无任何 GUI 框架。**

| 层 | 选型 | 理由 |
|---|---|---|
| 语言/运行时 | Rust stable，`panic=abort` | 无运行时、无 GC、无托管堆 |
| 系统调用 | `windows` crate（按需开 feature） | 官方绑定，只编译用到的部分 |
| 托盘 | `Shell_NotifyIconW` 直调（或 `tray-icon` 薄封装） | 不引入 GUI 框架 |
| 设置窗口 | **按需创建的 Win32 对话框**（`.rc` + `DialogBoxParamW`），或 `native-windows-gui` | 关闭即销毁，不占常驻内存 |
| 调度 | `CreateWaitableTimerExW` + `WaitForMultipleObjects` | 零轮询、零 CPU、可跨越睡眠 |
| 配置 | 单个 `config.json`（serde_json） | 无数据库依赖 |
| 日出日落 | 自实现 NOAA / sunrise equation（~120 行） | 零第三方依赖，精度 ±1–2 分钟 |
| 开机自启 | `HKCU\...\CurrentVersion\Run` | 免管理员、免 COM |
| 打包 | 单 exe（`+crt-static`） | 绿色便携 |

**预期指标**：exe ≈ 0.6–1.5 MB，常驻内存 ≈ 3–6 MB，空闲 CPU = 0.0%，每日唤醒 2 次。

### 4.2 变体 A′（推荐，若设置界面要求精致）

> **Rust 托盘核心（单 exe，3–6 MB）+ Tauri 设置器（独立 exe，按需启动、关闭即退）**

托盘进程永不加载 WebView；用户点"设置"时 `CreateProcess` 拉起设置器，设置器写完配置后退出（`msedgewebview2.exe` 随之退出）。常驻内存仍是 3–6 MB，但 UI 可以用现代 Web 技术做。

代价：两个二进制、需要进程间配置同步（用配置文件 + 文件变更通知即可，无需 IPC 协议）。

### 4.3 备选 B（团队是 .NET 背景时的务实选择）

> **C# + 裸 Win32 P/Invoke + NativeAOT（`PublishAot=true`，`OutputType=WinExe`，不引用任何 GUI 框架）**

8–15 MB，能进 30 MB 预算。但必须注意两个已核实的事实：

- **WinForms 官方不支持 NativeAOT**（[dotnet/winforms#10208](https://github.com/dotnet/winforms/issues/10208)、[dotnet/runtime#79166](https://github.com/dotnet/runtime/issues/79166)），且 .NET 11 Preview 3 出现回归：AOT 后的 exe 体积异常小且无法启动（[dotnet/runtime#127049](https://github.com/dotnet/runtime/issues/127049)）。**不要把它当生产路径押注。**
- **NativeAOT 在 Windows 上没有内置 COM 支持**。这直接影响 WinRT `Geolocator`（COM 激活）——所以定位方案无论选哪个技术栈，都应以 IP 兜底/手动坐标为主。

因此 C# 选项的实际形态与方案 A 相同：手写 Win32 托盘 + 对话框，只是换成 C# 语法。**开发速度并没有比 Rust 快多少，却多了一份"官方不支持"的风险**，故降为备选。

### 4.4 方案 0（R4 绝对优先时的极端解）

> **无常驻进程：注册两个计划任务，分别在日出/日落时刻运行一个极小 exe（切完即退）；再加一个每日 00:05 的任务重算次日时刻并改写前两个任务。**

常驻内存 **0 MB**，是理论最优解。但牺牲：托盘图标、一键临时切换、"5 分钟后切换"这类交互、以及任务计划被系统策略禁用时的容错。**建议作为方案 A 的降级形态保留，而不是主路径。**

---

## 5. 架构设计（方案 A）

### 5.1 进程与线程模型

```
主线程（消息循环，唯一常驻）
├─ Shell_NotifyIconW 托盘图标 + 右键菜单
├─ WaitForMultipleObjects 三位一体等待：
│    ├─ hTimer     可等待定时器（下一次切换时刻，绝对 UTC）
│    ├─ hRegNotify RegNotifyChangeKeyValue（用户手动改了主题 → 让位/同步状态）
│    └─ hWakeEvt   WM_POWERBROADCAST 唤醒事件（睡眠/休眠恢复）
└─ 按需创建设置对话框 → 关闭即 DestroyWindow，内存归还
```

**只有一个线程。** 不引入 async runtime（tokio 会带来约 2 MB 基线和额外线程），日出日落计算是纯 CPU 微秒级运算，不值得异步。

### 5.2 状态机

```
        ┌──────────── 计算 next_transition_at ────────────┐
        ▼                                                 │
   [Armed] ──timer 触发──► [ApplyTheme] ──成功──► 重新 Armed
        │
        ├── 注册表变更（用户手动）──► [Yield]  同步内部状态，等待下一个边界
        ├── 系统唤醒/时钟变更 ──────► [Recompute] 重新计算并 Armed
        └── 配置变更 ──────────────► [Recompute]
```

要点：**每次唤醒都从头重算"下一个切换时刻"，绝不缓存 UTC 时刻。** 这样夏令时切换、时区变更、系统时间被手动修改都不会导致错位。

### 5.3 数据流

```
config.json ──启动时读一次──► 内存中的 Config（<1 KB）
                                  │
定位: 手动坐标 > WinRT Geolocator > IP 兜底（缓存到 config）
                                  │
NOAA 算法（当日）──► sunrise/sunset（本地时间）
                                  │
调度器 ──► 下一次切换的 UTC 时刻 ──► SetWaitableTimer
                                  │
应用主题 ──► 注册表两键 + WM_SETTINGCHANGE 广播
```

### 5.4 建议目录结构

```
auto-dark-mode/
├─ Cargo.toml
├─ src/
│  ├─ main.rs            // windows_subsystem, 单实例 mutex, 消息循环
│  ├─ tray.rs            // Shell_NotifyIcon + 菜单
│  ├─ theme.rs           // 注册表读写 + 广播 + 高对比度保护
│  ├─ schedule.rs        // 时间表解析、next_transition 计算、waitable timer
│  ├─ sun.rs             // NOAA 日出日落
│  ├─ geo.rs             // 手动坐标 / WinRT / IP 兜底
│  ├─ autostart.rs       // Run 键增删 + 状态检测
│  ├─ config.rs          // serde 结构 + 原子写入
│  └─ settings_ui.rs     // 按需创建的 Win32 对话框
├─ res/app.rc, app.ico
└─ docs/adr/
```

---

## 6. 关键技术要点与坑

### 6.1 主题切换的正确顺序

1. 先写 `AppsUseLightTheme`，再写 `SystemUsesLightTheme`（顺序反了会出现"应用已变、任务栏没变"的中间态）。
2. 再广播 `WM_SETTINGCHANGE`，`lParam = L"ImmersiveColorSet"`，**必须用 `SendMessageTimeout` 而非 `SendMessage`**（某个 hung 窗口会卡死你的进程）。
3. **高对比度模式下必须直接放弃操作**：`SystemParametersInfoW(SPI_GETHIGHCONTRAST)` 返回开启时，切深浅色会破坏无障碍设置。这是必须处理的伦理与稳定性问题。
4. 不要尝试用 `DwmSetWindowAttribute` 去"修"别的进程的窗口——那只对自身窗口有效。

### 6.2 任务栏不刷新的兜底

Windows 11 上偶发：注册表已改、应用已变、任务栏没变。按代价从低到高兜底：

1. 重复广播一次（间隔 ~100 ms）；
2. 追加广播 `WM_THEMECHANGED`；
3. 最坏情况重启 `explorer.exe`（会闪屏、丢失资源管理器窗口）——**默认关闭，仅作为用户可在设置里开启的"强制模式"**。

### 6.3 定时器不要轮询

```rust
// 绝对 UTC 时间；正数=绝对，负数=相对。单位 100ns（FILETIME）
SetWaitableTimer(h, &due_100ns, 0, None, None, false)?;
WaitForSingleObject(h, INFINITE);
```

- **禁止 `Sleep(1000)` 轮询**：会让 CPU 无法进入深度 C-state，笔记本续航变差，且违背 R4 的精神（内存数字好看但功耗难看）。
- 必须监听 `WM_POWERBROADCAST` / `PBT_APMRESUMEAUTOMATIC`：睡眠跨过切换时刻时，绝对定时器行为不可靠，唤醒后要立刻重算。
- 必须监听 `WM_TIMECHANGE`：用户改时区/时间后重算。
- 每次触发后都要重新 `SetWaitableTimer`（一次性定时器），顺便重算下一次。

### 6.4 用户手动覆盖的尊重

用 `RegNotifyChangeKeyValue(hKey, FALSE, REG_NOTIFY_CHANGE_LAST_SET, hEvent, TRUE)` 监视。

区分"自己写的"和"用户改的"：写入时设置一个 `suppress_until` 时间戳（比如 2 秒），在此期间收到的变更通知判定为自己触发的，忽略。否则用户手切一次，程序立刻切回去，体验极差。

### 6.5 日出日落

- 算法：NOAA 太阳位置 + 日出方程，太阳高度角取 **-0.833°**（含 34′ 折射 + 16′ 太阳半径）。给出日出/日落/正午/昼长，纯数学、无网络。
- 时区：**用本地墙钟时间存配置，每次重新计算时再做 本地→UTC 转换**，不要缓存 UTC 偏移。
- **极昼/极夜**：`cos(ω) = (sin(-0.833°) - sin(φ)sin(δ)) / (cos(φ)cos(δ))` 落在 [-1,1] 之外时无解，需退化为"保持当前主题"或用户设定的固定时间兜底。高纬度用户一定会遇到。
- 缓冲：真实产品需要 ±N 分钟偏移和"提前/延后"设置，日出时刻精确到秒没有意义（云层、室内光照影响远大于 1 分钟）。

### 6.6 定位

优先级：**手动坐标（最可靠）> Windows 定位 API > IP 兜底**。

- WinRT `Geolocator`：需 `windows` crate 的 `Devices_Geolocation` feature；未打包的 Win32 应用受"设置 → 隐私 → 定位 → 允许桌面应用访问位置"开关控制，可能返回 `AccessDenied`。**不能把它当作唯一来源。**
- IP 定位（如 ip-api.com，免费无 key）：城市级误差（±20–50 km），对日出日落而言误差 <2 分钟，**完全够用**，且无权限问题。建议作为默认，手动坐标作为精确修正。
- 离线场景：缓存上次成功的位置；首次启动无网络就用默认时区中心点并标注"估算"。

### 6.7 开机自启

- `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，值 = `"C:\path\app.exe" --tray`。
- 优点：免管理员、免 COM、User 注销即清理。缺点：会被"任务管理器 → 启动应用"里被用户禁用（这是合理行为，不要强行"修复"）。
- 更稳但更重的替代：计划任务 `LogonTrigger` + `PT30S` 延迟（等网络就绪）。**建议作为 v2 选项**，写入需要任务计划 COM/XML，代码量翻倍。
- 首次运行就应询问"是否开机启动"，并写入；不要静默写开机项。

### 6.8 单实例

`CreateMutexW(Local\AutoDarkMode)`；已存在时用 `FindWindowW` 找到托盘窗口类名，`PostMessage(WM_APP_SHOW_SETTINGS)` 唤起设置窗口后退出。**用 `Local\` 而非 `Global\`**（按登录会话隔离，多用户 RDP 场景才对）。

### 6.9 配置写入要原子

写 `config.json.tmp` → `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)`。避免断电/崩溃留下半个 JSON 导致下次启动崩在解析上。

---

## 7. 内存与体积优化清单（Rust）

```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

1. `#![windows_subsystem = "windows"]` —— 否则会挂一个控制台窗口。
2. `+crt-static`（`.cargo/config.toml`）—— 去掉 VC++ 运行库依赖，真正单文件。
3. `windows` crate 只开必需 feature（`Win32_UI_Shell`、`Win32_UI_WindowsAndMessaging`、`Win32_System_Registry`、`Win32_System_Threading` …），未开的不参与编译。
4. 不引入 tokio / anyhow / clap / log+env_logger 全套。日志用 ~30 行自写滚动写入即可。
5. 设置窗口关闭时 `DestroyWindow`，不要 `ShowWindow(SW_HIDE)` —— 隐藏的窗口仍持有 GDI 资源。
6. 图标走 `.rc` 嵌入资源，不读外部文件。
7. 避免 `String` 到处 clone；配置本身 <1 KB，无所谓，但注意别把大头像/日志读进内存。

**验收口径**：安装后静置 24 小时，任务管理器"内存"列 ≤ 8 MB，CPU 累计时间 ≤ 1 秒，无周期性唤醒（可用 `powercfg /sleepstudy` 或 ETW 验证）。

---

## 8. 里程碑

| 阶段 | 内容 | 验收 |
|---|---|---|
| M1 | 主题切换 PoC（注册表 + 广播，CLI 参数） | 命令行切换成功，无需重启 explorer |
| M2 | 托盘 + 单实例 + 配置文件 + 固定时间表调度 | 静置内存 < 8 MB，切换准时 |
| M3 | 设置对话框 + 开机自启 + 手动覆盖检测 | 全流程可用，无 UAC |
| M4 | 日出日落 + 定位（IP 兜底 + 手动坐标） | 极昼/极夜不崩 |
| M5 | 便携包 + 签名 + 真实内存/功耗测量 | 达到 §7 验收口径 |

---

## 9. 明确不采用的方案

| 方案 | 否决理由 |
|---|---|
| Electron | 150 MB+ 常驻，与 R4 差 30 倍。 |
| WinUI 3 | 需 Windows App SDK 运行时，常驻 80 MB+，打包复杂，为一个托盘图标付出这个代价不合理。 |
| WPF / WinForms | JIT 版常驻 35–60 MB，直接突破 30 MB 预算；而 NativeAOT 官方不支持 WinForms，无法用 AOT 换内存。 |
| Tauri（WebView 常驻） | WebView2 子进程（browser/renderer/gpu/utility）合计 80–150 MB。仅接受"懒加载 + 关闭即销毁"的变体 A′。 |
| Python + pystray | 25–50 MB，且需打包运行时。 |
| AutoHotkey / PowerShell | 无法做可靠的产品级托盘与调度，功耗与稳定性差。 |

---

## 10. 已确认 / 待确认

**已确认（2026-11-09）**

- **R4 = 后台常驻 < 30 MB。** 该预算下 Rust（3–6 MB）与 Tauri 懒加载（10–20 MB）都合格；Rust 保留 5 倍以上余量，且留出后续加功能的空间。
- **本机工具链**：Rust 1.92 可用；`dotnet` 不可用。C# 路线在当前环境无落地条件。

**待确认**

1. **设置界面是否需要"精致"？** —— 若是，走 A′（Rust 核心 + Tauri 设置器）而非方案 A 的原生对话框；若否，方案 A 的 Win32 对话框足够。
2. 是否需要"完整主题"切换（连带壁纸/鼠标指针/声音）？—— 这会引入 `.theme` 文件操作与 explorer 闪烁，建议 v1 不做。
