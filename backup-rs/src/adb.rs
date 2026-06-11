use crate::util::*;
use std::path::Path;
use std::process::Command;

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

pub fn pull(src: &str, dest: &str) -> anyhow::Result<()> {
    let _ = std::fs::create_dir_all(dest);
    adb(&["pull", src, dest])?;
    Ok(())
}

pub fn push(src: &str, dest: &str) -> anyhow::Result<()> {
    adb(&["push", src, dest])?;
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
    e(&format!("Connecting to {}:5555...", ip));
    adb(&["tcpip", "5555"])?;
    adb(&["connect", &format!("{}:5555", ip)])?;
    Ok(())
}

pub fn backup_android(dest: &str) -> anyhow::Result<()> {
    let dest = std::path::Path::new(dest);
    let dest_str = dest.to_string_lossy().to_string();

    e("Backing up Android device");

    let device_list = devices();
    if device_list.is_empty() {
        return Err(anyhow::anyhow!("No Android device connected"));
    }
    e(&format!("Device: {}{}{}", C, device_list[0], N));

    let phone_dir = format!("{}/phone", dest_str);
    let _ = std::fs::create_dir_all(&phone_dir);

    e("Pulling media...");
    for dir in &["DCIM", "Download", "Pictures", "Movies", "Music"] {
        let src = format!("/sdcard/{}", dir);
        let dst = format!("{}/{}", phone_dir, dir);
        e(&format!("  {}{}{} → ...", W, dir, N));
        let _ = pull(&src, &dst);
    }

    e("Pulling MIUI data...");
    if let Ok(out) = shell("ls /sdcard/MIUI/") {
        if !out.is_empty() {
            let dst = format!("{}/MIUI", phone_dir);
            let _ = pull("/sdcard/MIUI/", &dst);
        }
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

    Ok(())
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
