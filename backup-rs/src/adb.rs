use crate::util::*;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

fn adb(args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("adb")
        .args(args)
        .output()?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow::anyhow!("adb {} failed: {}", args.join(" "), stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub struct PullStats {
    pub files_pulled: u64,
    pub files_skipped: u64,
    pub bytes: u64,
}

fn parse_pull_stats(line: &str) -> PullStats {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut pulled = 0u64;
    let mut skipped = 0u64;
    let mut bytes = 0u64;

    for (i, part) in parts.iter().enumerate() {
        if *part == "files" && i + 1 < parts.len() && parts[i + 1] == "pulled," {
            pulled = parts[i - 1].parse().unwrap_or(0);
        }
        if *part == "skipped." && i > 0 {
            skipped = parts[i - 1].trim_end_matches(',').parse().unwrap_or(0);
        }
        if *part == "bytes" && i > 0 {
            bytes = parts[i - 1].trim_start_matches('(').parse().unwrap_or(0);
        }
    }

    PullStats { files_pulled: pulled, files_skipped: skipped, bytes }
}

pub fn available() -> bool {
    run_ok("which adb") && adb(&["get-state"]).is_ok()
}

pub fn devices() -> Vec<String> {
    let out = adb(&["devices"]).unwrap_or_default();
    out.lines()
        .skip(1)
        .filter(|l| l.contains("device") && !l.contains("unauthorized") && !l.contains("offline"))
        .filter_map(|l| l.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

fn device_model() -> String {
    let model = shell("getprop ro.product.model").unwrap_or_default();
    let model = model.trim();
    if model.is_empty() || model == "sdk_gphone64_arm64" {
        shell("getprop ro.product.name").unwrap_or_default().trim().to_string()
    } else {
        model.to_string()
    }
}

pub fn pull(src: &str, dest: &str) -> anyhow::Result<PullStats> {
    let _ = std::fs::create_dir_all(dest);
    let mut child = Command::new("adb")
        .args(&["pull", src, dest])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let mut last_line = String::new();
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
                if !line.trim().is_empty() {
                    last_line = line;
                }
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(anyhow::anyhow!("adb pull {} {} failed", src, dest));
    }

    Ok(parse_pull_stats(&last_line))
}

// ── FTP SERVER CONTROL ────────────────────────────────────
// Tries to start CX File Explorer's FTP server via ADB.
// Falls back gracefully — user can start it manually.
fn ftp_start() {
    for intent in &[
        "com.cxinventor.file.explorer.action.FTP_START",
        "com.cxinventor.file.explorer.action.FTPSERVER",
        "com.alphainventor.filemanager.action.FTP_START",
        "com.alphainventor.filemanager.action.FTPSERVER",
    ] {
        let _ = adb(&["shell", "am", "broadcast", "-a", intent]);
    }
    // Also try launching the app (FTP may auto-start if it was running before)
    let _ = adb(&["shell", "am", "start", "-n",
        "com.cxinventor.file.explorer/com.alphainventor.filemanager.activity.MainActivity"]);
}

fn ftp_stop() {
    for intent in &[
        "com.cxinventor.file.explorer.action.FTP_STOP",
        "com.cxinventor.file.explorer.action.FTPSERVER_STOP",
        "com.alphainventor.filemanager.action.FTP_STOP",
    ] {
        let _ = adb(&["shell", "am", "broadcast", "-a", intent]);
    }
}

fn wait_for_ftp(host: &str, port: &str, timeout_secs: u64) -> bool {
    let addr = format!("{}:{}", host, port);
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    while std::time::Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(2)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

// ── RCLONE COPY VIA FTP ──────────────────────────────────
// Uses rclone's FTP backend to copy files incrementally.
// Only transfers new/changed files — identical files are skipped.
fn ftp_copy(src: &str, dst: &str, host: &str, port: &str, user: &str, pass: &str) -> anyhow::Result<PullStats> {
    let _ = std::fs::create_dir_all(dst);

    let obs_pass = run_stdout(&format!("rclone obscure '{}'", pass));
    let obs_pass = obs_pass.trim().to_string();

    let status = Command::new("rclone")
        .args(&[
            "copy",
            &format!(":ftp:{}", src),
            dst,
            "--progress",
            "--stats=1s",
            "--ftp-host", host,
            "--ftp-port", port,
            "--ftp-user", user,
            "--ftp-pass", &obs_pass,
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("rclone copy via FTP failed: {} -> {}", src, dst));
    }

    // Count local files as a rough stats estimate
    let file_count = std::fs::read_dir(dst).ok()
        .map(|e| e.flatten().filter(|e| e.path().is_file()).count() as u64)
        .unwrap_or(0);

    Ok(PullStats { files_pulled: file_count, files_skipped: 0, bytes: 0 })
}

pub fn push(src: &str, dest: &str) -> anyhow::Result<()> {
    let status = Command::new("adb")
        .args(&["push", src, dest])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("adb push {} {} failed", src, dest));
    }
    Ok(())
}

pub fn shell(cmd: &str) -> anyhow::Result<String> {
    adb(&["shell", cmd])
}

pub fn content_query(uri: &str) -> anyhow::Result<String> {
    shell(&format!("content query --uri {}", uri))
}

#[allow(dead_code)]
pub fn connect_wifi(ip: &str) -> anyhow::Result<()> {
    let usb_connected = run_ok("adb shell getprop ro.serialno 2>/dev/null");

    if usb_connected {
        e("Switching to TCP/IP mode...");
        adb(&["tcpip", "5555"])?;
        e(&format!("Unplug USB, then connecting to {}:5555...", ip));
    } else {
        e(&format!("Connecting to {}:5555...", ip));
    }

    let out = adb(&["connect", &format!("{}:5555", ip)]);
    match out {
        Ok(s) if s.contains("connected") => {
            e(&format!("{}{}{}", G, s.trim(), N));
            Ok(())
        }
        _ => Err(anyhow::anyhow!(
            "Could not connect.\n  Connect phone via USB first, or use: ~/easy-scrcpy.sh --backup"
        )),
    }
}

pub fn backup_android(_dest: &str) -> anyhow::Result<()> {
    let device_list = devices();
    if device_list.is_empty() {
        return Err(anyhow::anyhow!("No Android device connected"));
    }
    let serial = &device_list[0];

    let model = device_model();
    let tag = model.to_lowercase().replace(' ', "-");
    let tag: String = tag.chars().filter(|c| c.is_alphanumeric() || *c == '-').collect();
    let tag = if tag.is_empty() { serial.clone() } else { tag };

    let base = crate::config::backup_base();
    let dest = format!("{}/{}", base, tag);
    let phone_dir = format!("{}/phone", dest);
    let _ = std::fs::create_dir_all(&phone_dir);

    e("Backing up Android device");
    e(&format!("Device: {}{}{}", C, serial, N));
    e(&format!("Dest:   {}{}{}", W, phone_dir, N));

    let mut total_pulled = 0u64;
    let mut total_skipped = 0u64;
    let mut total_bytes = 0u64;
    let skip_dirs = crate::config::android_skip_dirs();

    fn local_file_count(dir: &str) -> u64 {
        std::fs::read_dir(dir).ok()
            .map(|e| e.flatten().filter(|e| e.path().is_file()).count() as u64)
            .unwrap_or(0)
    }

    // Check if FTP mode is configured
    let use_ftp = crate::config::android_ftp_host().is_some();
    let ftp_host = crate::config::android_ftp_host();
    let ftp_port = crate::config::android_ftp_port();
    let ftp_user = crate::config::android_ftp_user();
    let ftp_pass = crate::config::android_ftp_pass();

    let (ftp_host, ftp_port, ftp_user, ftp_pass) = if use_ftp {
        let h = ftp_host.as_deref().unwrap_or("localhost");
        let p = &ftp_port;
        let u = &ftp_user;
        let pw = &ftp_pass;
        e("Starting FTP server on phone...");
        ftp_start();
        if wait_for_ftp(h, p, 10) {
            e(&format!("  {}FTP connected {}:{} {}", G, h, p, N));
        } else {
            e(&format!("  {}Could not reach FTP server at {}:{}{}", Y, h, p, N));
            e("  Start the FTP server manually in CX File Explorer (Network → FTP)");
            e("  Waiting longer...");
            if !wait_for_ftp(h, p, 60) {
                return Err(anyhow::anyhow!("FTP server not reachable"));
            }
        }
        (h.to_string(), p.clone(), u.clone(), pw.clone())
    } else {
        (String::new(), String::new(), String::new(), String::new())
    };

    let media_dirs: &[&str] = &["DCIM", "Download", "Pictures", "Movies", "Music"];

    if use_ftp {
        e("Copying media via FTP (rclone)...");
    } else {
        e("Pulling media via ADB...");
    }

    for dir in media_dirs {
        if skip_dirs.iter().any(|d| d.eq_ignore_ascii_case(dir)) {
            e(&format!("  {}{}{} → {} skipped (config)", W, dir, N, Y));
            continue;
        }
        let dst = format!("{}/{}", phone_dir, dir);

        if use_ftp {
            let src = format!("/device/{}", dir);
            let label = format!("  {}{}{} → ...", W, dir, N);
            e(&label);
            match ftp_copy(&src, &dst, &ftp_host, &ftp_port, &ftp_user, &ftp_pass) {
                Ok(stats) => {
                    total_pulled += stats.files_pulled;
                    total_skipped += stats.files_skipped;
                    total_bytes += stats.bytes;
                }
                Err(err) => {
                    e(&format!("  {} FTP copy failed for {}: {}{}", R, dir, err, N));
                }
            }
        } else {
            let src = format!("/sdcard/{}", dir);
            let existing = local_file_count(&dst);
            if existing > 0 {
                total_skipped += existing;
                e(&format!("  {}{}{} → {} (already backed up, {} files skipped)", W, dir, N, C, existing));
                continue;
            }
            e(&format!("  {}{}{} → ...", W, dir, N));
            if let Ok(stats) = pull(&src, &dst) {
                total_pulled += stats.files_pulled;
                total_skipped += stats.files_skipped;
                total_bytes += stats.bytes;
            }
        }
    }

    if !skip_dirs.iter().any(|d| d.eq_ignore_ascii_case("MIUI")) {
        if use_ftp {
            let dst = format!("{}/MIUI", phone_dir);
            e(&format!("  {}{}{} → ...", W, "MIUI", N));
            if let Ok(stats) = ftp_copy("/device/MIUI/", &dst, &ftp_host, &ftp_port, &ftp_user, &ftp_pass) {
                total_pulled += stats.files_pulled;
                total_skipped += stats.files_skipped;
                total_bytes += stats.bytes;
            }
        } else {
            e("Pulling MIUI data...");
            if let Ok(out) = shell("ls /sdcard/MIUI/") {
                if !out.is_empty() {
                    let dst = format!("{}/MIUI", phone_dir);
                    let existing = local_file_count(&dst);
                    if existing > 0 {
                        total_skipped += existing;
                        e(&format!("  {}{}{} → {} (already backed up, {} files skipped)", W, "MIUI", N, C, existing));
                    } else if let Ok(stats) = pull("/sdcard/MIUI/", &dst) {
                        total_pulled += stats.files_pulled;
                        total_skipped += stats.files_skipped;
                        total_bytes += stats.bytes;
                    }
                }
            }
        }
    } else {
        e(&format!("  {}{}{} → {} skipped (config)", W, "MIUI", N, Y));
    }

    if use_ftp {
        ftp_stop();
    }

    e("Saving SMS...");
    let sms_path = format!("{}/sms.json", phone_dir);
    if let Ok(out) = content_query("content://sms/") {
        let _ = std::fs::write(&sms_path, &out);
        e(&format!("  {} {} SMS messages", C, out.lines().count()));
    }

    e("Saving contacts...");
    let contacts_path = format!("{}/contacts.json", phone_dir);
    if let Ok(out) = content_query("content://contacts/phones/") {
        let _ = std::fs::write(&contacts_path, &out);
        e(&format!("  {} {} contacts", C, out.lines().count()));
    }

    e("Saving call logs...");
    let calls_path = format!("{}/call_logs.json", phone_dir);
    if let Ok(out) = content_query("content://call_log/calls/") {
        let _ = std::fs::write(&calls_path, &out);
    }

    e("Saving installed apps...");
    let apps_path = format!("{}/packages.txt", phone_dir);
    if let Ok(out) = shell("pm list packages -f") {
        let _ = std::fs::write(&apps_path, &out);
        let count = out.lines().filter(|l| l.starts_with("package:")).count();
        e(&format!("  {} {} packages listed", C, count));
    }

    e("Saving device info...");
    if let Ok(out) = shell("getprop") {
        let _ = std::fs::write(&format!("{}/device.prop", phone_dir), &out);
    }

    e(&format!("{}{}Android backup complete!{}", BOLD, G, N));
    e(&format!("Location: {}{}{}", W, phone_dir, N));
    e(&format!(
        "  {} {} pulled, {} skipped, {} total",
        C, total_pulled, total_skipped, fmt(total_bytes)
    ));

    Ok(())
}

pub fn list_android_dirs() -> Vec<String> {
    let base = crate::config::backup_base();
    let dir = std::path::Path::new(&base);
    if !dir.is_dir() {
        return vec![];
    }
    let mut out = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("phone").is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    out.push(name.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

pub fn restore_android(backup_dir: &str) -> anyhow::Result<()> {
    let phone_dir = format!("{}/phone", backup_dir);

    if !Path::new(&phone_dir).is_dir() {
        return Err(anyhow::anyhow!("No Android backup found in {}", backup_dir));
    }

    e("Restoring Android device");

    let device_list = devices();
    if device_list.is_empty() {
        return Err(anyhow::anyhow!("No Android device connected"));
    }
    e(&format!("Device: {}{}{}", C, device_list[0], N));

    for dir in &["DCIM", "Download", "Pictures", "Movies", "Music", "MIUI"] {
        let src = format!("{}/{}", phone_dir, dir);
        if Path::new(&src).is_dir() {
            e(&format!("  Pushing {}{}{} → phone...", W, dir, N));
            let dst = format!("/sdcard/{}", dir);
            let _ = push(&src, &dst);
        }
    }

    let apps_path = format!("{}/packages.txt", phone_dir);
    if Path::new(&apps_path).exists() {
        e("Install packages listed, re-import SMS/contacts via Android app");
    }

    e(&format!("{}{}Android restore complete!{}", BOLD, G, N));
    Ok(())
}
