use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Global log file path, set at backup start.
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

pub static R: &str = "\x1b[0;31m";
pub static G: &str = "\x1b[0;32m";
pub static Y: &str = "\x1b[0;33m";
pub static M: &str = "\x1b[0;35m";
pub static C: &str = "\x1b[0;36m";
pub static W: &str = "\x1b[1;37m";
pub static N: &str = "\x1b[0m";

pub fn e(s: &str) {
    println!("{}", s);
    log_append(&strip_ansi(s));
}

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

pub fn run(cmd: &str) -> anyhow::Result<Output> {
    Ok(Command::new("sh").arg("-c").arg(cmd).output()?)
}

pub fn run_ok(cmd: &str) -> bool {
    Command::new("sh").arg("-c").arg(cmd).output().is_ok_and(|o| o.status.success())
}

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

pub fn count_files(path: &str) -> u64 {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("find '{}' -xdev -type f 2>/dev/null | wc -l", path))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_default();
    out.parse().unwrap_or(0)
}

pub fn copy_progress(
    base_cmd: &str,
    checkers: u32,
    ntfs: bool,
    skip_links: bool,
    no_traverse: bool,
    _total_files: u64,
    scan_msg: Option<&str>,
) -> anyhow::Result<i32> {
    let mut extra = String::new();
    if ntfs { extra.push_str(" --ignore-errors"); }
    if skip_links { extra.push_str(" --skip-links"); }
    if no_traverse { extra.push_str(" --no-traverse"); }
    extra.push_str(" --fast-list --buffer-size=64M");
    // Use --progress for the live status bar with fast updates
    let full = format!(
        "{} --progress --stats=200ms --checkers {} --transfers {}{}",
        base_cmd, checkers, checkers, extra,
    );

    let mut child = Command::new("sh")
        .arg("-c").arg(&full)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()?;

    if let Some(msg) = scan_msg {
        let start = Instant::now();
        let mut first = true;
        loop {
            thread::sleep(Duration::from_secs(1));
            if let Some(_) = child.try_wait()? {
                break;
            }
            let d = start.elapsed();
            let secs = d.as_secs();
            let m = secs / 60;
            let s = secs % 60;
            if first {
                first = false;
                let line = if m > 0 {
                    format!("  {}{}... {}m {}s{}", Y, msg, m, s, N)
                } else {
                    format!("  {}{}... {}s{}", Y, msg, s, N)
                };
                e(&line);
            }
        }
        let d = start.elapsed();
        let m = d.as_secs() / 60;
        let s = d.as_secs() % 60;
        if m > 0 {
            e(&format!("  {}{} complete ({}m {}s){}", G, msg, m, s, N));
        } else {
            e(&format!("  {}{} complete ({}s){}", G, msg, s, N));
        }
    }

    let status = child.wait()?;
    Ok(status.code().unwrap_or(-1))
}

pub fn _fmt(size: u64) -> String {
    // Try numfmt first
    let out = run_stdout(&format!("numfmt --to=iec {}", size));
    if !out.is_empty() { return out; }
    format!("{} MiB", size / 1024 / 1024)
}

pub fn detect_path() -> String {
    let host = run_stdout("hostname -s");
    let host = if host.is_empty() { "unknown".into() } else { host };
    let os_id = run_stdout(r#"grep -oP '(?<=^ID=).*' /etc/os-release | tr -d '"'"#);
    let tag = if !os_id.is_empty() && !host.to_lowercase().contains(&os_id.to_lowercase()) {
        format!("-{}", os_id)
    } else {
        String::new()
    };
    format!("{}/{}{}", crate::config::backup_base(), host, tag)
}

pub fn detect_checkers(path: &str) -> u32 {
    let out = run_stdout(&format!(
        "lsblk -ndo rota $(findmnt -T '{}' -o SOURCE | tail -1) 2>/dev/null",
        path
    ));
    match out.as_str() {
        "0" => {
            // Check if NVMe
            let name = run_stdout(&format!(
                "lsblk -ndo pkname $(findmnt -T '{}' -o SOURCE | tail -1) 2>/dev/null",
                path
            ));
            if name.starts_with("nvme") { 16 } else { 8 }
        }
        _ => 3,
    }
}

pub fn install_deps() -> bool {
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
