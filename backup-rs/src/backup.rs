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
    let commands = [
        ("pacman -Qqen", "packages-pacman-official.txt"),
        ("pacman -Qqem", "packages-aur.txt"),
        ("dpkg --get-selections", "packages-dpkg.txt"),
        (r"dnf list installed 2>/dev/null | tail -n +2 | awk '{print $1}'", "packages-dnf.txt"),
        (r"zypper se --installed-only -s 2>/dev/null | tail -n +5 | awk '{print $3}'", "packages-zypper.txt"),
        ("apk info", "packages-apk.txt"),
        ("flatpak list --app --columns=application", "flatpak-list.txt"),
        ("snap list", "snap-list.txt"),
    ];
    for (cmd, filename) in commands {
        let _ = run(&format!("{} > '{}/{}' 2>/dev/null", cmd, dest, filename));
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
pub fn backup_config(dest: &str, ck: u32) {
    e("Backing up configs");
    let cfg_dest = format!("{}/config", dest);

    let mut extra_args: Vec<&str> = Vec::new();
    for &x in CACHE_EXCLUDES.iter().chain(CONFIG_EXCLUDES.iter()) {
        extra_args.push("--exclude");
        extra_args.push(x);
    }
    let home = crate::HOME.get().unwrap();
    e(&format!("  {}.config{} → ...", W, N));
    let _ = copy_progress("~/.config/", &cfg_dest, ck, false, &extra_args);

    for item in &[".ssh", ".gnupg", ".local/share/keyrings"] {
        let src = format!("{}/{}", home, item);
        if Path::new(&src).is_dir() {
            e(&format!("  {}{}{} → ...", W, item, N));
            let _ = run(&format!("cp -a '{}' '{}/' 2>/dev/null", src, dest));
        }
    }
}

// ── BACKUP BROWSERS ────────────────────────────────────────
// Copies Firefox, Chromium, Chrome, and Brave profiles.
// Only backs up profiles that changed since the last backup
// (saves time by skipping unchanged ones).
pub fn backup_browsers(dest: &str, ck: u32) {
    e("Backing up browser data");
    let b_dest = format!("{}/browser", dest);

    let mut extra_args: Vec<&str> = Vec::new();
    for &x in CACHE_EXCLUDES.iter().chain(BROWSER_EXCLUDES.iter()) {
        extra_args.push("--exclude");
        extra_args.push(x);
    }
    let home = crate::HOME.get().unwrap();

    let manifest_path = config::manifest_path();
    let mut manifest = load_manifest(&manifest_path);
    let mut changed = 0u32;
    let mut skipped = 0u32;

    for (src_rel, name) in BROWSERS {
        let src = format!("{}/{}", home, src_rel);
        if !Path::new(&src).is_dir() { continue; }
        let mtime = dir_mtime(&src).unwrap_or(0);
        if manifest.get(*name) == Some(&mtime) {
            e(&format!("  {}{}{} unchanged", C, name, N));
            skipped += 1;
            continue;
        }
        e(&format!("  {}{}{} → ...", W, name, N));
        let _ = copy_progress(
            &format!("{}/", src),
            &format!("{}/{}/", b_dest, name),
            ck, false, &extra_args,
        );
        manifest.insert(name.to_string(), mtime);
        changed += 1;
    }
    let _ = save_manifest(&manifest_path, &manifest);
    if changed > 0 || skipped > 0 {
        e(&format!("Done: {} backed up, {} skipped", changed, skipped));
    }
}

// ── BACKUP VIRTUAL MACHINES ────────────────────────────────
// Saves libvirt VM configuration files and disk images
// (if you use virt-manager / KVM / QEMU).
pub fn backup_vm(dest: &str, ck: u32) {
    e("Backing up VM data");
    let vm_dest = format!("{}/virt-manager", dest);
    let _ = std::fs::create_dir_all(&vm_dest);

    if Path::new(VM_QEMU_SRC).is_dir() {
        e(&format!("  {}libvirt configs{} → ...", W, N));
        let _ = run(&format!("sudo cp -a '{}' '{}/' 2>/dev/null", VM_QEMU_SRC, vm_dest));
    }
    if Path::new(VM_IMAGES_SRC).is_dir() {
        e(&format!("  {}VM disk images{} → ...", W, N));
        let _ = copy_progress(VM_IMAGES_SRC, &format!("{}/images/", vm_dest), ck, true, &["--inplace"]);
    }
}

// ── BACKUP HOME FOLDER ─────────────────────────────────────
// Copies your entire home folder (~/), but excludes things
// that don't need backing up: caches, trash, node_modules,
// game installs, build artifacts, etc.
pub fn backup_home(dest: &str, ck: u32) {
    e("Backing up home data");
    let home_dest = format!("{}/home", dest);

    let mut extra_args: Vec<&str> = vec!["--links"];
    for &x in HOME_EXCLUDES.iter() {
        extra_args.push("--exclude");
        extra_args.push(x);
    }
    e(&format!("  {}~{}{} → ...", W, N, N));
    let _ = copy_progress("~/", &home_dest, ck, true, &extra_args);
}

// ── BACKUP EXTRA DIRECTORIES ───────────────────────────────
// Copies any extra folders the user specified in the config
// file (BACKUP_EXTRA_DIRS). Skips unchanged ones.
pub fn backup_extra(dest: &str, ck: u32) {
    let dirs = extra_backup_dirs();
    if dirs.is_empty() { return; }
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
        let _ = copy_progress(&src, &target, ck, false, &[]);
        manifest.insert(name, mtime);
        changed += 1;
    }
    let _ = save_manifest(&manifest_path, &manifest);
    if changed > 0 || skipped > 0 {
        e(&format!("Done: {} backed up, {} skipped", changed, skipped));
    }
}

// ── DO BACKUP (main function) ──────────────────────────────
// This is the main backup routine. It:
//   1. Checks the backup drive is plugged in
//   2. Creates the destination folder
//   3. Saves package lists and estimates size (in parallel)
//   4. Backs up configs, browsers, VMs, home, extra dirs
//   5. Writes a ".complete" marker so we know it finished
//   6. Shows the final size and location
pub fn do_backup(dest: &str, auto_yes: bool) -> anyhow::Result<()> {
    let dest = std::path::Path::new(dest);
    let dest_str = dest.to_string_lossy().to_string();
    let base_mount = dest.parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    if !run_ok(&format!("findmnt -n '{}'", base_mount)) {
        e(&format!("{}Error: backup drive not mounted at {}{}", R, base_mount, N));
        e(&format!("{}Mount the drive and try again.{}", Y, N));
        std::process::exit(1);
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
    let kind = if ck <= 3 { "HDD" } else if ck <= 8 { "SSD" } else { "NVMe" };
    e(&format!("Checkers: {} ({})", ck, kind));

    let gdu_handle = thread::spawn(|| estimate_home_size());
    save_package_lists(&dest_str);
    let _ = gdu_handle.join();

    backup_config(&dest_str, ck);
    backup_browsers(&dest_str, ck);
    backup_vm(&dest_str, ck);
    backup_home(&dest_str, ck);
    backup_extra(&dest_str, ck);

    let sz_out = run_stdout(&format!("du -sh '{}' | cut -f1", dest_str));
    e(&format!("{}{}Done!{}", BOLD, G, N));
    e(&format!("Size: {}{}{}", W, sz_out, N));
    e(&format!("Location: {}{}{}", W, dest.display(), N));
    let _ = std::fs::write(&complete_marker, "");
    e(&format!("Log: {}{}{}", Y, dest.join("backup.log").display(), N));

    Ok(())
}
