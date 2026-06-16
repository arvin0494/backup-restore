// ─────────────────────────────────────────────────────────────
// UTILITIES — helper tools used by the rest of the program
// ─────────────────────────────────────────────────────────────
// This file provides small reusable pieces:
// - log file writing
// - colored text for the terminal
// - running commands
// - copying files with progress
// - detecting backup paths and drive types
// - installing missing dependencies
// ─────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, OnceLock};

// ── LOGGING ─────────────────────────────────────────────────
// Writes messages to a log file (backup.log) so you can review
// what happened after the backup finishes.
pub static LOG_FILE: OnceLock<Mutex<String>> = OnceLock::new();
pub fn init_log(path: String) { let _ = LOG_FILE.set(Mutex::new(path)); }

pub fn log_append(s: &str) {
    if let Some(mtx) = LOG_FILE.get() {
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open(mtx.lock().unwrap().as_str())
        {
            let _ = writeln!(f, "{}", s);
        }
    }
}

// ── TERMINAL COLORS ────────────────────────────────────────
// These variables let the program print text in different
// colors in the terminal (red, green, yellow, cyan, white, bold).
// The "N" variable resets the color back to normal.
// Respects NO_COLOR env var (set to disable colors, e.g., on Windows).
static NO_COLOR: OnceLock<bool> = OnceLock::new();
fn no_color() -> bool {
    *NO_COLOR.get_or_init(|| std::env::var("NO_COLOR").unwrap_or_default().len() > 0)
}
pub static R: &str = if no_color() { "" } else { "\x1b[0;31m" };
pub static G: &str = if no_color() { "" } else { "\x1b[0;32m" };
pub static Y: &str = if no_color() { "" } else { "\x1b[0;33m" };
pub static C: &str = if no_color() { "" } else { "\x1b[0;36m" };
pub static W: &str = if no_color() { "" } else { "\x1b[1;37m" };
pub static BOLD: &str = if no_color() { "" } else { "\x1b[1m" };
pub static N: &str = if no_color() { "" } else { "\x1b[0m" };

// ── PRINT MESSAGE ─────────────────────────────────────────
// Shows a message in the terminal (with a "[*]" prefix) and
// also writes it to the log file (with colors stripped out).
pub fn e(msg: &str) {
    println!("{BOLD}[{G}*{N}{BOLD}]{N} {msg}");
    log_append(&strip_ansi(msg));
}

// ── DIRECTORY MODIFICATION TIME ────────────────────────────
// Gets the last-modified timestamp of a folder. Used to check
// whether a folder has changed since the last backup.
pub fn dir_mtime(path: &str) -> Option<u64> {
    std::fs::metadata(path).ok()?
        .modified().ok()?
        .duration_since(std::time::UNIX_EPOCH).ok()
        .map(|d| d.as_secs())
}

// ── MANIFEST FILE HELPERS ──────────────────────────────────
// The manifest is a simple text file that records when each
// folder was last backed up. On the next run, folders whose
// modification time hasn't changed can be skipped (faster).
pub fn load_manifest(path: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    let content = std::fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if let Ok(ts) = v.trim().parse::<u64>() {
                map.insert(k.trim().to_string(), ts);
            }
        }
    }
    map
}

pub fn save_manifest(path: &str, map: &HashMap<String, u64>) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut s = String::new();
    let mut pairs: Vec<_> = map.iter().collect();
    pairs.sort_by_key(|p| p.0.clone());
    for (k, v) in pairs {
        s.push_str(&format!("{}={}\n", k, v));
    }
    std::fs::write(path, s)?;
    Ok(())
}

// ── STRIP ANSI COLORS ──────────────────────────────────────
// Removes color codes from text before writing to the log file.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.next() == Some('[') {
                for c in &mut chars {
                    if c == 'm' {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ── RUN COMMAND ────────────────────────────────────────────
// Runs any shell command and returns the result (output + status).
pub fn run(cmd: &str) -> anyhow::Result<Output> {
    Ok(Command::new("sh").arg("-c").arg(cmd).output()?)
}

// ── RUN AND CHECK SUCCESS ──────────────────────────────────
// Runs a command and returns true/false depending on whether
// it succeeded (e.g., "which rclone" → does rclone exist?).
pub fn run_ok(cmd: &str) -> bool {
    Command::new("sh").arg("-c").arg(cmd).output().is_ok_and(|o| o.status.success())
}

// ── RUN WITH INPUT ─────────────────────────────────────────
// Runs a command, sends it some text as input, and returns
// whatever the command prints out (used for fzf selection).
pub fn run_stdin(cmd: &str, input: &str) -> anyhow::Result<String> {
    let mut child = Command::new("sh")
        .arg("-c").arg(cmd)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// ── RUN AND GET OUTPUT ─────────────────────────────────────
// Runs a command and returns whatever it printed to the screen
// (or an empty string if it failed).
pub fn run_stdout(cmd: &str) -> String {
    Command::new("sh")
        .arg("-c").arg(cmd)
        .output().ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_default()
}

// ── COPY WITH PROGRESS ─────────────────────────────────────
// Uses "rclone copy" to copy files from one place to another.
// Shows progress on screen (speed, ETA, file names).
// If sudo is needed (for system folders), it runs rclone via sudo.
pub fn copy_progress(
    src: &str,
    dst: &str,
    checkers: u32,
    sudo: bool,
    extra_args: &[&str],
) -> anyhow::Result<i32> {
    let _ = std::fs::create_dir_all(dst);

    let program = if sudo { "sudo" } else { "rclone" };
    let mut args: Vec<String> = Vec::with_capacity(10 + extra_args.len());
    if sudo {
        args.push("rclone".into());
    }
    args.push("copy".into());
    args.push(src.into());
    args.push(dst.into());
    args.push("--progress".into());
    args.push("--stats=1s".into());
    args.push(format!("--checkers={}", checkers));
    args.push(format!("--transfers={}", checkers));
    args.extend(extra_args.iter().map(|a| a.to_string()));

    let status = Command::new(program)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    Ok(status.code().unwrap_or(-1))
}

// ── FORMAT SIZE ────────────────────────────────────────────
// Converts a raw byte count into a human-readable format
// (e.g., "1.5 GiB" instead of "1610612736").
pub fn fmt(size: u64) -> String {
    let out = run_stdout(&format!("numfmt --to=iec {}", size));
    if !out.is_empty() { return out; }
    format!("{} MiB", size / 1024 / 1024)
}

// ── DETECT BACKUP PATH ─────────────────────────────────────
// Automatically figures out the backup destination:
//   Linux:  /mnt/HDD4T/BACKUP/{computer-name}[-{os-name}]
//   Windows: E:\BACKUP\{computer-name}
// It uses the computer's hostname.
pub fn detect_path() -> String {
    let host = run_stdout("hostname -s");
    let host = if host.is_empty() { "unknown".into() } else { host };
    let base = crate::config::backup_base();
    
    if detect_platform() == "windows" {
        format!("{}\\{}", base, host)
    } else {
        let os_id = run_stdout(r#"grep -oP '(?<=^ID=).*' /etc/os-release | tr -d '"'"#);
        let tag = if !os_id.is_empty() && !host.to_lowercase().contains(&os_id.to_lowercase()) {
            format!("-{}", os_id)
        } else {
            String::new()
        };
        format!("{}/{}{}", base, host, tag)
    }
}

// ── DETECT DRIVE TYPE ──────────────────────────────────────
// Figures out whether the backup drive is:
//   - HDD (slow, 1 parallel transfer)
//   - SSD (fast, 8 parallel transfers)
//   - NVMe (very fast, 16 parallel transfers)
// This tunes the copy speed to avoid overwhelming the drive.
pub fn detect_checkers(path: &str) -> u32 {
    let out = run_stdout(&format!(
        "lsblk -ndo rota $(findmnt -T '{}' -o SOURCE | tail -1) 2>/dev/null",
        path
    ));
    match out.as_str() {
        "0" => {
            let name = run_stdout(&format!(
                "lsblk -ndo pkname $(findmnt -T '{}' -o SOURCE | tail -1) 2>/dev/null",
                path
            ));
            if name.starts_with("nvme") { 16 } else { 8 }
        }
        _ => 1,
    }
}

// ── DETECT PLATFORM ────────────────────────────────────────
// Checks if we're running on Linux or Windows (WSL/MSYS).
// Returns "linux" or "windows".
pub fn detect_platform() -> &'static str {
    if std::env::var("APPDATA").is_ok()
        || std::env::var("SYSTEMDRIVE").is_ok()
        || std::env::var("COMPUTERNAME").is_ok()
    {
        "windows"
    } else {
        "linux"
    }
}

// ── INSTALL DEPENDENCIES ───────────────────────────────────
// Checks whether rclone, gdu, and fzf are installed. If not,
// installs them using the system's package manager (pacman,
// apt-get, dnf, zypper, or apk). Runs automatically at startup.
pub fn install_deps() -> bool {
    if detect_platform() == "windows" {
        if run_ok("where rclone") {
            return true;
        }
        e(&format!("{}rclone not found.{}", R, N));
        let mut tried = false;
        // Winget (built into Win10 1809+, Win11)
        if run_ok("where winget") {
            tried = true;
            e(&format!("{}Installing rclone via winget...{}", Y, N));
            let _ = run("winget install --id Rclone.Rclone --silent --accept-package-agreements --accept-source-agreements");
            if run_ok("where rclone") {
                e(&format!("{}{}Installed via winget{}", G, W, N));
                return true;
            }
        }
        // Chocolatey
        if run_ok("where choco") {
            tried = true;
            e(&format!("{}Installing rclone via choco...{}", Y, N));
            let _ = run("choco install rclone -y");
            if run_ok("where rclone") {
                e(&format!("{}{}Installed via choco{}", G, W, N));
                return true;
            }
        }
        if !tried {
            e("  Winget and choco not found.");
            e("  Install winget: https://apps.microsoft.com/detail/97980606q267 (Win11) or winget source");
            e("  Install choco: https://chocolatey.org/install");
            e("  Or download rclone: https://rclone.org/downloads/");
        } else {
            e("  Failed to install. Try manually:");
            e("    winget install Rclone.Rclone   or   choco install rclone");
            e("    Or download: https://rclone.org/downloads/");
        }
        return false;
    }

    let (pm, pkgs): (&str, Vec<&str>) = if run_ok("which pacman") {
        ("sudo pacman -S --noconfirm", vec!["rclone", "gdu", "fzf"])
    } else if run_ok("which apt-get") {
        ("sudo apt-get install -y", vec!["rclone", "gdu", "fzf"])
    } else if run_ok("which dnf") {
        ("sudo dnf install -y", vec!["rclone", "gdu", "fzf"])
    } else if run_ok("which zypper") {
        ("sudo zypper install -y", vec!["rclone", "gdu", "fzf"])
    } else if run_ok("which apk") {
        ("sudo apk add", vec!["rclone", "gdu", "fzf"])
    } else {
        e(&format!("{}No known package manager found.{}", R, N));
        return false;
    };

    let need: Vec<&str> = pkgs.iter().filter(|p| !run_ok(&format!("which {}", p))).copied().collect();
    if need.is_empty() {
        return true;
    }

    e(&format!("{}Installing:{} {}{}{}", Y, N, W, need.join(" "), N));
    for pkg in &need {
        let _ = run(&format!("{} {}", pm, pkg));
    }
    true
}
