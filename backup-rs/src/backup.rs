// ─────────────────────────────────────────────────────────────
// BACKUP — saves your files and programs to a safe location
// ─────────────────────────────────────────────────────────────
// This file handles everything about creating a backup:
// - listing installed programs
// - estimating how big your files are
// - copying configs, browsers, VMs, and home folders
// ─────────────────────────────────────────────────────────────

use crate::config;
use crate::config::*;
use crate::util::*;
use std::env;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;

// ── ESTIMATE SIZE ──────────────────────────────────────────
// Asks "gdu" (a disk-usage tool) how big a folder is,
// while ignoring directories the user probably doesn't want
// to back up (like .cache or node_modules).
pub fn gdu_size(path: &str, ignore: &str) -> u64 {
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "gdu -n -s -p --no-prefix --ignore-dirs '{}' '{}' 2>/dev/null | awk '{{print $1}}'",
            ignore, path,
        ))
        .output().ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
        .unwrap_or_default()
        .parse::<u64>().unwrap_or(0)
}

// ── SAVE PACKAGE LISTS ─────────────────────────────────────
// Writes out a list of all installed programs so they can be
// reinstalled later. Supports many Linux families:
//   Arch (pacman, AUR), Debian/Ubuntu (dpkg), Fedora (dnf),
//   openSUSE (zypper), Alpine (apk), Flatpak, and Snap.
pub fn save_package_lists(dest: &str) {
    e("Saving package lists");

    let os_id = std::fs::read_to_string("/etc/os-release")
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("ID=").map(|v| v.trim_matches('"').trim().to_lowercase()))
        .unwrap_or_default();

    let mut commands: Vec<(&str, &str)> = Vec::new();

    match os_id.as_str() {
        "arch" | "archarm" | "manjaro" | "endeavouros" | "cachyos" | "arcolinux" | "garuda" => {
            commands.push(("pacman -Qqen", "packages-pacman-official.txt"));
            commands.push(("pacman -Qqem", "packages-aur.txt"));
        }
        "debian" | "ubuntu" | "linuxmint" | "pop" | "zorin" | "elementary" | "kali" => {
            commands.push(("dpkg --get-selections", "packages-dpkg.txt"));
        }
        "fedora" | "rhel" | "centos" | "rocky" | "almalinux" => {
            commands.push((r"dnf list installed 2>/dev/null | tail -n +2 | awk '{print $1}'", "packages-dnf.txt"));
        }
        "opensuse" | "opensuse-tumbleweed" | "suse" | "opensuse-leap" => {
            commands.push((r"zypper se --installed-only -s 2>/dev/null | tail -n +5 | awk '{print $3}'", "packages-zypper.txt"));
        }
        "alpine" => {
            commands.push(("apk info", "packages-apk.txt"));
        }
        _ => {
            commands.push(("pacman -Qqen", "packages-pacman-official.txt"));
            commands.push(("pacman -Qqem", "packages-aur.txt"));
            commands.push(("dpkg --get-selections", "packages-dpkg.txt"));
            commands.push((r"dnf list installed 2>/dev/null | tail -n +2 | awk '{print $1}'", "packages-dnf.txt"));
            commands.push((r"zypper se --installed-only -s 2>/dev/null | tail -n +5 | awk '{print $3}'", "packages-zypper.txt"));
            commands.push(("apk info", "packages-apk.txt"));
        }
    }

    commands.push(("flatpak list --app --columns=application", "flatpak-list.txt"));
    commands.push(("snap list", "snap-list.txt"));

    let mut handles = Vec::new();
    for (cmd, filename) in commands {
        let dest = dest.to_string();
        let handle = thread::spawn(move || {
            let _ = run(&format!("{} > '{}/{}' 2>/dev/null", cmd, dest, filename));
        });
        handles.push(handle);
    }
    for handle in handles {
        let _ = handle.join();
    }
}

// ── ESTIMATE HOME SIZE ─────────────────────────────────────
// Scans common folders in your home directory (~/Documents,
// ~/Pictures, ~/Projects, etc.) and adds up their sizes so
// you know how much space the backup will need.
pub fn estimate_home_size() -> u64 {
    if !run_ok("which gdu") {
        return 0;
    }
    e("Estimating size...");
    let gdu_ignore = GDU_IGNORE_DIRS.join(",");
    let mut total = 0u64;

    for d in GDU_SCAN_DIRS {
        let p = format!("{}/{}", crate::HOME.get().unwrap(), d);
        if !Path::new(&p).is_dir() { continue; }
        e(&format!("  {}{}{} ...", C, d, N));
        total += gdu_size(&p, &gdu_ignore);
    }
    for d in extra_backup_dirs() {
        if !Path::new(&d).is_dir() { continue; }
        let name = Path::new(&d).file_name().unwrap_or_default().to_string_lossy().to_string();
        e(&format!("  {}{}{} ...", C, name, N));
        total += gdu_size(&d, &gdu_ignore);
    }
    e(&format!("Estimated data size: {}{}{}", W, fmt(total), N));
    total
}

// ── BACKUP CONFIGS ─────────────────────────────────────────
// Copies your settings (~/.config, ~/.ssh, ~/.gnupg, keyrings)
// to the backup folder. Skips caches and trash to save space.
// On Windows, backs up SSH keys and GPG keys to the user profile.
pub fn backup_config(dest: &str, ck: u32) -> anyhow::Result<()> {
    e("Backing up configs");
    let cfg_dest = format!("{}/config", dest);
    
    let platform = detect_platform();
    
    if platform == "linux" {
        let mut extra_args: Vec<&str> = Vec::new();
        for &x in CACHE_EXCLUDES.iter().chain(CONFIG_EXCLUDES.iter()) {
            extra_args.push("--exclude");
            extra_args.push(x);
        }
        let home = crate::HOME.get().unwrap();
        e(&format!("  {}.config{} → ...", W, N));
        copy_progress(&format!("{}/.config/", home), &cfg_dest, ck, false, &extra_args)?;
        
        for item in &[".ssh", ".gnupg", ".local/share/keyrings"] {
            let src = format!("{}/{}", home, item);
            if Path::new(&src).is_dir() {
                e(&format!("  {}{}{} → ...", W, item, N));
                copy_progress(&src, &format!("{}/{}", dest, item), ck, false, &[])?;
            }
        }
    } else {
        // Windows: SSH keys and GPG
        let userprofile = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
        for item in &[".ssh", ".gnupg"] {
            let src = format!("{}{}", userprofile, item);
            if Path::new(&src).is_dir() || Path::new(&format!("{}.gitconfig", src)).exists() {
                e(&format!("  {}{}{} → ...", W, item, N));
                copy_progress(&src, &format!("{}/{}", dest, item), ck, false, &[])?;
            }
        }
        // Git config (Windows git for windows uses it)
        let gitconfig = format!("{}\\gitconfig", userprofile);
        if Path::new(&gitconfig).exists() {
            e(&format!("  {}gitconfig{} → ...", W, N));
            copy_progress(&gitconfig, &format!("{}/gitconfig", dest), ck, false, &[])?;
        }
    }
    Ok(())
}

// ── BACKUP BROWSERS ────────────────────────────────────────
// Copies Firefox, Chromium, Chrome, and Brave profiles.
// Only backs up profiles that changed since the last backup
// (saves time by skipping unchanged ones).
// On Windows, backs up from %APPDATA% and %LOCALAPPDATA%.
pub fn backup_browsers(dest: &str, ck: u32) -> anyhow::Result<()> {
    e("Backing up browser data");
    let b_dest = format!("{}/browser", dest);
    
    let mut extra_args: Vec<&str> = Vec::new();
    for &x in CACHE_EXCLUDES.iter().chain(BROWSER_EXCLUDES.iter()) {
        extra_args.push("--exclude");
        extra_args.push(x);
    }
    
    let platform = detect_platform();
    let manifest_path = config::manifest_path();
    let mut manifest = load_manifest(&manifest_path);
    let mut changed = 0u32;
    let mut skipped = 0u32;
    
    if platform == "linux" {
        let home = crate::HOME.get().unwrap();
        
        for (src_rel, name) in BROWSERS_LINUX {
            let src = format!("{}/{}", home, src_rel);
            if !Path::new(&src).is_dir() { continue; }
            let mtime = dir_mtime(&src).unwrap_or(0);
            if manifest.get(*name) == Some(&mtime) {
                e(&format!("  {}{}{} unchanged", C, name, N));
                skipped += 1;
                continue;
            }
            e(&format!("  {}{}{} → ...", W, name, N));
            copy_progress(
                &format!("{}/", src),
                &format!("{}/{}/", b_dest, name),
                ck, false, &extra_args,
            )?;
            manifest.insert(name.to_string(), mtime);
            changed += 1;
        }
    } else {
        let userprofile = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string());
        
        for (src_rel, name) in BROWSERS_WINDOWS {
            let src = format!("{}\\{}", userprofile, src_rel);
            if !Path::new(&src).is_dir() { continue; }
            let mtime = dir_mtime(&src).unwrap_or(0);
            if manifest.get(*name) == Some(&mtime) {
                e(&format!("  {}{}{} unchanged", C, name, N));
                skipped += 1;
                continue;
            }
            e(&format!("  {}{}{} → ...", W, name, N));
            copy_progress(
                &format!("{}/", src),
                &format!("{}/{}/", b_dest, name),
                ck, false, &extra_args,
            )?;
            manifest.insert(name.to_string(), mtime);
            changed += 1;
        }
    }
    save_manifest(&manifest_path, &manifest)?;
    if changed > 0 || skipped > 0 {
        e(&format!("Done: {} backed up, {} skipped", changed, skipped));
    }
    Ok(())
}

// ── BACKUP VIRTUAL MACHINES ────────────────────────────────
// Saves libvirt VM configuration files and disk images
// (if you use virt-manager / KVM / QEMU). Linux only.
pub fn backup_vm(dest: &str, ck: u32) -> anyhow::Result<()> {
    if detect_platform() != "linux" { return Ok(()); }
    e("Backing up VM data");
    let vm_dest = format!("{}/virt-manager", dest);
    let _ = std::fs::create_dir_all(&vm_dest);
    
    if Path::new(VM_QEMU_SRC).is_dir() {
        e(&format!("  {}libvirt configs{} → ...", W, N));
        run(&format!("sudo cp -a '{}' '{}/' 2>/dev/null", VM_QEMU_SRC, vm_dest))?;
    }
    if Path::new(VM_IMAGES_SRC).is_dir() {
        e(&format!("  {}VM disk images{} → ...", W, N));
        copy_progress(VM_IMAGES_SRC, &format!("{}/images/", vm_dest), ck, true, &["--inplace"])?;
    }
    Ok(())
}

// ── BACKUP HOME FOLDER ─────────────────────────────────────
// Copies your entire home folder (~/), but excludes things
// that don't need backing up: caches, trash, node_modules,
// game installs, build artifacts, etc.
pub fn backup_home(dest: &str, ck: u32) -> anyhow::Result<()> {
    e("Backing up home data");
    let home_dest = format!("{}/home", dest);
    
    let mut extra_args: Vec<&str> = vec!["--links"];
    for &x in HOME_EXCLUDES.iter() {
        extra_args.push("--exclude");
        extra_args.push(x);
    }
    e(&format!("  {}~{}{} → ...", W, N, N));
    copy_progress(&format!("{}/", crate::HOME.get().unwrap()), &home_dest, ck, true, &extra_args)?;
    Ok(())
}

// ── BACKUP EXTRA DIRECTORIES ───────────────────────────────
// Copies any extra folders the user specified in the config
// file (BACKUP_EXTRA_DIRS). Skips unchanged ones.
pub fn backup_extra(dest: &str, ck: u32) -> anyhow::Result<()> {
    let dirs = extra_backup_dirs();
    if dirs.is_empty() { return Ok(()); }
    e("Backing up extra dirs");
    let extra_dest = format!("{}/extra", dest);
    let _ = std::fs::create_dir_all(&extra_dest);
    
    let manifest_path = config::manifest_path();
    let mut manifest = load_manifest(&manifest_path);
    let mut changed = 0u32;
    let mut skipped = 0u32;
    
    for src in dirs {
        let p = Path::new(&src);
        if !p.is_dir() {
            e(&format!("  {}{}{} not found", Y, src, N));
            continue;
        }
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let mtime = dir_mtime(&src).unwrap_or(0);
        if manifest.get(&name) == Some(&mtime) {
            e(&format!("  {}{}{} unchanged", C, name, N));
            skipped += 1;
            continue;
        }
        let target = format!("{}/{}", extra_dest, name);
        e(&format!("  {}{}{} → ...", W, name, N));
        copy_progress(&src, &target, ck, false, &[])?;
        manifest.insert(name, mtime);
        changed += 1;
    }
    save_manifest(&manifest_path, &manifest)?;
    if changed > 0 || skipped > 0 {
        e(&format!("Done: {} backed up, {} skipped", changed, skipped));
    }
    Ok(())
}

// ── DO BACKUP (main function) ──────────────────────────────
// This is the main backup routine. It:
//   1. Checks the backup drive is available
//   2. Creates the destination folder
//   3. Saves package lists and estimates size (in parallel)
//   4. Backs up configs, browsers, VMs, home, extra dirs
//   5. Writes a ".complete" marker so we know it finished
//   6. Shows the final size and location
pub fn do_backup(dest: &str, auto_yes: bool) -> anyhow::Result<()> {
    let dest = std::path::Path::new(dest);
    let dest_str = dest.to_string_lossy().to_string();
    let platform = detect_platform();

    // Check the backup drive is available
    if platform == "linux" {
        let base_mount = dest.parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if !run_ok(&format!("findmnt -n '{}'", base_mount)) {
            e(&format!("{}Error: backup drive not mounted at {}{}", R, base_mount, N));
            e(&format!("{}Mount the drive and try again.{}", Y, N));
            return Err(anyhow::anyhow!("Backup drive not mounted"));
        }
    } else {
        // Windows: just verify the parent drive exists
        if let Some(parent) = dest.parent() {
            let parent_str = parent.to_string_lossy();
            if !Path::new(&parent_str.to_string()).exists() {
                e(&format!("{}Error: backup path not found: {}{}", R, parent_str, N));
                return Err(anyhow::anyhow!("Backup path not found"));
            }
        }
    }
    let _ = std::fs::create_dir_all(dest);

    let complete_marker = dest.join(".complete");
    init_log(format!("{}/backup.log", dest_str));

    e(&format!("Target: {}{}{}", W, dest.display(), N));
    if complete_marker.exists() {
        e(&format!("{}Warning: backup already exists at this location{}", Y, N));
        if !auto_yes {
            print!("  Overwrite existing backup? [y/N] ");
            std::io::stdout().flush().ok();
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).ok();
            if buf.trim().to_lowercase() != "y" {
                e(&format!("{}Cancelled.{}", Y, N));
                return Ok(());
            }
        }
    }

    let ck = detect_checkers(&dest_str);
    let kind = if ck <= 1 { "HDD" } else if ck <= 8 { "SSD" } else { "NVMe" };
    e(&format!("Checkers: {} ({})", ck, kind));

    let gdu_handle = thread::spawn(|| estimate_home_size());
    save_package_lists(&dest_str);
    let _ = gdu_handle.join();

    backup_config(&dest_str, ck)?;
    backup_browsers(&dest_str, ck)?;
    backup_vm(&dest_str, ck)?;
    backup_home(&dest_str, ck)?;
    backup_extra(&dest_str, ck)?;

    let sz_out = run_stdout(&format!("du -sh '{}' | cut -f1", dest_str));
    e(&format!("{}{}Done!{}", BOLD, G, N));
    e(&format!("Size: {}{}{}", W, sz_out, N));
    e(&format!("Location: {}{}{}", W, dest.display(), N));
    let _ = std::fs::write(&complete_marker, "");
    e(&format!("Log: {}{}{}", Y, dest.join("backup.log").display(), N));

    Ok(())
}
