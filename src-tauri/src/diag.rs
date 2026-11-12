//! 启动诊断：把关键步骤写入 `%TEMP%\autodarkmode.log`，便于在窗口起不来时定位。
//!
//! 之所以需要它：Tauri 在 `app.rs` 的 `setup()` 里会自动创建 `tauri.conf.json`
//! 声明的窗口，而窗口创建过程中 WebView2 需要 `create_dir_all(%LOCALAPPDATA%\标识符)`。
//! 这一步一旦失败就会以 `Failed to setup app: 拒绝访问。(os error 5)` 直接 panic，
//! 并且发生在任何用户代码之前 —— 没有任何输出可查。现在我们自己建窗口，
//! 既能把失败原因记下来，也能换一个可写的 WebView2 数据目录。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

fn writable_file(p: &Path) -> bool {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
        .is_ok()
}

/// Tauri 的 `app_local_data_dir` 依赖 `%LOCALAPPDATA%`；日志也优先放临时目录，
/// 但受限环境两者都可能被拒，所以再加一个 exe 同级目录作为兜底。
fn target() -> &'static PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        let t = std::env::temp_dir().join("autodarkmode.log");
        if writable_file(&t) {
            return t;
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let p = dir.join("autodarkmode.log");
                if writable_file(&p) {
                    return p;
                }
            }
        }
        t
    })
}

/// 同时写 stderr 与日志文件；写失败就算了，诊断本身不能拖垮启动。
pub fn log(msg: &str) {
    eprintln!("[adm] {msg}");
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target())
    {
        let _ = writeln!(f, "{secs} {msg}");
    }
}

fn probe(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let f = dir.join(".write-probe");
    std::fs::write(&f, b"ok")?;
    let _ = std::fs::remove_file(&f);
    Ok(())
}

/// 依次尝试 WebView2 数据目录，返回第一个真正可写的。
/// 返回的 `notes` 记录被跳过的候选及原因。
pub fn pick_webview_dir() -> (PathBuf, Vec<String>) {
    let mut notes = Vec::new();
    let mut cands: Vec<PathBuf> = Vec::new();

    match std::env::var("LOCALAPPDATA") {
        Ok(la) => cands.push(PathBuf::from(la).join("com.autodarkmode.desktop")),
        Err(_) => notes.push("LOCALAPPDATA 未定义".to_string()),
    }
    cands.push(std::env::temp_dir().join("autodarkmode-webview"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            cands.push(dir.join("webview-data"));
        }
    }

    for c in cands {
        match probe(&c) {
            Ok(()) => return (c, notes),
            Err(e) => notes.push(format!("{} -> {e}", c.display())),
        }
    }

    // 全都不可写：仍然把第一个候选交给 WebView2，让错误在日志里留痕
    notes.push("所有候选目录均不可写，回退到临时目录".to_string());
    (std::env::temp_dir().join("autodarkmode-webview"), notes)
}
