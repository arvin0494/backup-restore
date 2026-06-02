use std::io::{BufRead, BufReader, Write};
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
    // Use --stats=1s (clean lines, no ANSI escape codes)
    let full = format!(
        "{} --stats=1s --checkers {} --transfers {}{}",
        base_cmd, checkers, checkers, extra,
    );

    let mut child = Command::new("sh")
        .arg("-c").arg(&full)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().unwrap();
    use std::sync::Mutex;
    let progress: std::sync::Arc<Mutex<String>> = Default::default();
    let pclone = progress.clone();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = match line { Ok(l) => l, Err(_) => break };
            // Stats lines from --stats=1s contain "Transferred:" or "Checks:"
            if line.contains("Transferred:") || (line.contains('%') && line.contains("Checks:")) {
                *pclone.lock().unwrap() = line;
            } else if line.contains("Elapsed time:") {
                // skip
            } else {
                // NOTICE/ERROR/other — forward to terminal
                eprintln!("{}", line);
            }
        }
    });

    let exit_code;

    if let Some(msg) = scan_msg {
        let start = Instant::now();
        let bar_width: usize = 28;
        loop {
            thread::sleep(Duration::from_secs(1));
            match child.try_wait()? {
                Some(status) => {
                    exit_code = status.code().unwrap_or(-1);
                    break;
                }
                None => {}
            }
            let _ = start.elapsed(); // keep timing for completion message
            // Parse the latest stats line
            let stats = progress.lock().unwrap().clone();
            let (transferred, pct, speed, eta) = parse_stats_line(&stats);

            // Build a progress bar
            let bar = make_bar(&pct, bar_width);

            // Build the display line like: {msg} {transferred} {speed} {eta} [{bar}] {pct}%
            let pct_display = if pct.is_empty() { "-".to_string() } else { pct.clone() };
            let display = format!(
                "\r  {}{:<20} {:>14}  {:>8}  {:>5}  {} {:>3}%{}",
                Y, msg, transferred, speed, eta, bar, pct_display, N,
            );
            eprint!("{}", display);
            std::io::stderr().flush().ok();
        }
        // Clear the progress line
        eprint!("\r{}\r", " ".repeat(80));
        let d = start.elapsed();
        let m = d.as_secs() / 60;
        let s = d.as_secs() % 60;
        let elapsed = if m > 0 { format!("{}m {}s", m, s) } else { format!("{}s", s) };
        eprintln!("\r{}\r  {}{} complete ({}){}", " ".repeat(80), G, msg, elapsed, N);
    } else {
        stderr_handle.join().ok();
        let status = child.wait()?;
        exit_code = status.code().unwrap_or(-1);
        return Ok(exit_code);
    }

    stderr_handle.join().ok();
    let _ = child.wait(); // already reaped by try_wait
    Ok(exit_code)
}

fn parse_stats_line(line: &str) -> (String, String, String, String) {
    let line = strip_ansi(line);
    // "Transferred:   1.234 GiB / 12.345 GiB, 10%, 5 MiB/s, ETA 5m"
    // or "Checks:         1234 / 412714, 0.3%"
    let (transferred, pct, speed, eta) = if line.is_empty() {
        (String::new(), String::new(), String::new(), String::new())
    } else if line.starts_with("Transferred") {
        // Parse "Transferred:   X / Y, Z%, W MiB/s, ETA T"
        let transferred = if let Some(end) = line.find(',') {
            line[..end].trim().to_string()
        } else { String::new() };
        let pct = if let Some(idx) = line.find('%') {
            let start = line[..idx].rfind(|c: char| !c.is_ascii_digit() && c != '.').map(|i| i+1).unwrap_or(0);
            line[start..idx].to_string()
        } else { String::new() };
        let eta = if let Some(idx) = line.find("ETA") {
            line[idx+3..].trim().to_string()
        } else { String::new() };
        let speed = if let Some(eta_pos) = line.find("ETA") {
            let before = &line[..eta_pos];
            if let Some(comma) = before.rfind(',') {
                before[comma+1..].trim().to_string()
            } else { String::new() }
        } else { String::new() };
        (transferred, pct, speed, eta)
    } else if line.starts_with("Checks") {
        // Parse "Checks:   X / Y, Z%"
        let pct = if let Some(idx) = line.find('%') {
            let start = line[..idx].rfind(|c: char| !c.is_ascii_digit() && c != '.').map(|i| i+1).unwrap_or(0);
            line[start..idx].to_string()
        } else { String::new() };
        (String::new(), pct, String::new(), String::new())
    } else {
        (String::new(), String::new(), String::new(), String::new())
    };
    (transferred, pct, speed, eta)
}

fn make_bar(pct: &str, width: usize) -> String {
    let p = pct.parse::<f64>().unwrap_or(0.0) / 100.0;
    let filled = (p * width as f64).round() as usize;
    let filled = filled.min(width);
    let mut bar = String::with_capacity(width + 2);
    bar.push('[');
    for i in 0..width {
        if i < filled {
            bar.push('=');
        } else if i == filled && filled < width {
            bar.push('>');
        } else {
            bar.push('-');
        }
    }
    bar.push(']');
    bar
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
