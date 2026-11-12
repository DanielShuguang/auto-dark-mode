//! 开机自启的两种模式。
//!
//! - **普通**：`HKCU\...\Run`，由 `tauri-plugin-autostart` 维护。不涉及提权，
//!   但未签名的程序在一些环境里会被安全软件/UAC 询问。
//! - **管理员**：计划任务 + `RunLevel Highest`。Windows 在执行"最高权限"的计划任务时
//!   **不会弹 UAC**，这是让提权程序和"开机不弹窗"同时成立的唯一办法。
//!
//! 注意 exe 本身始终是 `asInvoker`：双击不会提权，也就不会弹 UAC。
//! 提权只发生在计划任务这一条路径上。

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::win;

pub const TASK_NAME: &str = "AutoDarkMode";
/// 与 `tauri-plugin-autostart` 写入的值名保持一致，切换模式时才能清干净。
pub const RUN_VALUE: &str = "AutoDarkMode";

/// CREATE_NO_WINDOW：不加这个，每次起 schtasks 都会在屏幕上闪一个黑色控制台窗口。
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn schtasks() -> Command {
    let mut c = Command::new("schtasks");
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

// ---------------------------------------------------------------- 任务状态缓存

/// 状态面板每 30 秒刷新一次，如果每次都真去起一个 schtasks 进程，
/// 屏幕上就是不停闪黑框（某些环境下还会直接弹 0xc0000142 错误框）。
/// 任务只会在本进程里建/删，所以缓存到下次建删或超时即可。
fn cache() -> &'static Mutex<Option<(Instant, bool)>> {
    static CACHE: OnceLock<Mutex<Option<(Instant, bool)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn query_task() -> bool {
    schtasks()
        .args(["/query", "/tn", TASK_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn store(v: bool) {
    if let Ok(mut g) = cache().lock() {
        *g = Some((Instant::now(), v));
    }
}

/// 带缓存的查询，供状态面板这类高频调用使用。
///
/// 缓存**不过期**：任务只会由本进程创建/删除（都会 `invalidate_task_cache()`），
/// 所以一个进程生命周期里起 schtasks 的次数屈指可数 ——
/// 这正是「闪黑框 + 0xc0000142 弹窗」的根源。
pub fn task_exists() -> bool {
    if let Ok(g) = cache().lock() {
        if let Some((_, v)) = *g {
            return v;
        }
    }
    let v = query_task();
    store(v);
    v
}

/// 绕过缓存立即查询，用于建/删任务之后确认结果。
pub fn task_exists_fresh() -> bool {
    let v = query_task();
    store(v);
    v
}

pub fn invalidate_task_cache() {
    if let Ok(mut g) = cache().lock() {
        *g = None;
    }
}

fn output_text(out: &std::process::Output) -> String {
    let stderr = win::ansi_to_string(&out.stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = win::ansi_to_string(&out.stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }
    format!("退出码 {:?}", out.status.code())
}

fn exe_path() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| format!("拿不到自身路径: {e}"))
}

/// 当前用户账户名，形如 `DOMAIN\User`。计划任务需要一个明确的 UserId。
fn account_name() -> String {
    let user = std::env::var("USERNAME").unwrap_or_default();
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
        _ => user,
    }
}

/// 生成计划任务 XML：登录触发 + 最高权限 + 无执行时限。
fn task_xml(exe: &str) -> String {
    let account = account_name();
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>AutoDarkMode: 按时间或日出日落自动切换 Windows 11 深浅色</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{account}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{account}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>--background</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

fn write_xml(exe: &str) -> Result<PathBuf, String> {
    let utf16: Vec<u8> = std::iter::once(0xFFu8)
        .chain(std::iter::once(0xFE))
        .chain(task_xml(exe).encode_utf16().flat_map(|u| u.to_le_bytes()))
        .collect();

    // 临时目录在受限进程里可能不可写，退回到 exe 所在目录
    let mut candidates = vec![std::env::temp_dir().join("autodarkmode-task.xml")];
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            candidates.push(dir.join("autodarkmode-task.xml"));
        }
    }

    let mut last_err = String::new();
    for path in candidates {
        match std::fs::write(&path, &utf16) {
            Ok(()) => return Ok(path),
            Err(e) => last_err = format!("{}: {e}", path.display()),
        }
    }
    Err(last_err)
}

/// 创建/覆盖计划任务：登录时以最高权限静默运行 `exe --background`。
/// 需要管理员权限，否则 schtasks 会返回拒绝访问。
pub fn create_task() -> Result<String, String> {
    let exe = exe_path()?;
    let exe_str = exe.to_string_lossy().to_string();

    // 方式一：/tr 直接给命令行
    let mut tr = exe_str.clone();
    if tr.contains(' ') {
        tr = format!("\"{tr}\"");
    }
    tr.push_str(" --background");

    let out = schtasks()
        .args([
            "/create", "/f", "/tn", TASK_NAME, "/sc", "onlogon", "/rl", "HIGHEST", "/tr", &tr,
        ])
        .output()
        .map_err(|e| format!("调用 schtasks 失败: {e}"))?;

    if out.status.success() {
        invalidate_task_cache();
        return Ok(format!(
            "已创建计划任务 {TASK_NAME}：登录时以最高权限静默启动（不弹 UAC）"
        ));
    }
    let first_err = output_text(&out);

    // 方式二：/tr 的引号规则在部分环境下会被解析坏，改用任务 XML
    let xml = match write_xml(&exe_str) {
        Ok(p) => p,
        Err(e) => return Err(format!("/tr 方式: {first_err}；XML 方式不可用: {e}")),
    };
    let out2 = schtasks()
        .args(["/create", "/f", "/tn", TASK_NAME, "/xml"])
        .arg(&xml)
        .output()
        .map_err(|e| format!("调用 schtasks 失败: {e}"))?;
    let _ = std::fs::remove_file(&xml);

    if out2.status.success() {
        invalidate_task_cache();
        return Ok(format!(
            "已创建计划任务 {TASK_NAME}（XML 方式）：登录时以最高权限静默启动（不弹 UAC）"
        ));
    }

    Err(format!(
        "/tr 方式: {first_err}；XML 方式: {}",
        output_text(&out2)
    ))
}

pub fn delete_task() -> Result<(), String> {
    if !task_exists_fresh() {
        return Ok(());
    }
    let out = schtasks()
        .args(["/delete", "/f", "/tn", TASK_NAME])
        .output()
        .map_err(|e| format!("调用 schtasks 失败: {e}"))?;
    invalidate_task_cache();
    if out.status.success() {
        Ok(())
    } else {
        Err(output_text(&out))
    }
}
