use crate::config::*;
use crate::util::*;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;

pub fn save_package_lists(dest: &str) {
    e(&format!("{}--- Saving package lists ---{}", M, N));
    // Arch
    let _ = run(&format!("pacman -Qqen > '{}/packages-pacman-official.txt' 2>/dev/null", dest));
    let _ = run(&format!("pacman -Qqem > '{}/packages-aur.txt' 2>/dev/null", dest));
    // Debian / Ubuntu
    let _ = run(&format!("dpkg --get-selections > '{}/packages-dpkg.txt' 2>/dev/null", dest));
    // Fedora
    let _ = run(&format!("dnf list installed 2>/dev/null | tail -n +2 | awk '{{print \\$1}}' > '{}/packages-dnf.txt'", dest));
    // openSUSE
    let _ = run(&format!("zypper se --installed-only -s 2>/dev/null | tail -n +5 | awk '{{print \\$3}}' > '{}/packages-zypper.txt'", dest));
    // Alpine
    let _ = run(&format!("apk info > '{}/packages-apk.txt' 2>/dev/null", dest));
    // Cross-platform
    let _ = run(&format!("flatpak list --app --columns=application > '{}/flatpak-list.txt' 2>/dev/null", dest));
    let _ = run(&format!("snap list > '{}/snap-list.txt' 2>/dev/null", dest));
}

pub fn estimate_home_size() -> u64 {
    let total = 0u64;
    if !run_ok("which gdu") {
        return total;
    }
    e(&format!("  {}Estimating size...{}", Y, N));
    let gdu_ignore = GDU_IGNORE_DIRS.join(",");
    let mut total = 0u64;

    for d in GDU_SCAN_DIRS {
        let p = format!("{}/{}", crate::HOME.get().unwrap(), d);
        if !Path::new(&p).is_dir() { continue; }
        e(&format!("  {}  {}...{}", Y, d, N));
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gdu -n -s -p --no-prefix --ignore-dirs '{}' '{}' 2>/dev/null | awk '{{print $1}}'",
                gdu_ignore, p,
            ))
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            })
            .unwrap_or_default();
        total += out.parse::<u64>().unwrap_or(0);
    }
    // Include extra dirs from config
    for d in extra_backup_dirs() {
        if !Path::new(&d).is_dir() { continue; }
        let name = Path::new(&d).file_name().unwrap_or_default().to_string_lossy().to_string();
        e(&format!("  {}  {}...{}", Y, name, N));
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "gdu -n -s -p --no-prefix --ignore-dirs '{}' '{}' 2>/dev/null | awk '{{print $1}}'",
                gdu_ignore, d,
            ))
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            })
            .unwrap_or_default();
        total += out.parse::<u64>().unwrap_or(0);
    }
    e(&format!("  {}Estimated data size:{} {}{}{}", C, N, W, _fmt(total), N));
    total
}

pub fn backup_config(dest: &str, ck: u32) {
    e(&format!("{}--- Backing up configs ---{}", M, N));
    e(&format!("  {}Source:{} ~/.config, ~/.ssh, ~/.gnupg", C, N));
    e(&format!("  {}Target:{} {}/config", C, N, dest));
    let cfg_dest = format!("{}/config", dest);
    let _ = std::fs::create_dir_all(&cfg_dest);

    let cfg_size = run_stdout("du -sh ~/.config 2>/dev/null | cut -f1");
    e(&format!("  {}Config size:{} {}{}{}", C, N, W, cfg_size, N));

    let excludes: Vec<String> = CACHE_EXCLUDES.iter()
        .chain(CONFIG_EXCLUDES.iter())
        .map(|x| format!("--exclude '{}'", x))
        .collect();
    let ex = excludes.join(" ");

    e(&format!("  {}Syncing configs...{}", Y, N));
    let _ = copy_progress(
        &format!("rclone copy ~/.config/ '{}/' {}", cfg_dest, ex),
        ck, true, true, false, Some("Configs"),
    );

    let home = crate::HOME.get().unwrap();
    for item in &[".ssh", ".gnupg", ".local/share/keyrings"] {
        let src = format!("{}/{}", home, item);
        if Path::new(&src).is_dir() {
            let _ = run(&format!("cp -a '{}' '{}/' 2>/dev/null", src, dest));
        }
    }
}

pub fn backup_browsers(dest: &str, ck: u32) {
    e(&format!("{}--- Backing up browser data ---{}", M, N));
    e(&format!("  {}Target:{} {}/browser", C, N, dest));
    let b_dest = format!("{}/browser", dest);
    let _ = std::fs::create_dir_all(&b_dest);

    let bx: Vec<String> = CACHE_EXCLUDES.iter()
        .chain(BROWSER_EXCLUDES.iter())
        .map(|x| format!("--exclude '{}'", x))
        .collect();
    let bx = bx.join(" ");
    let home = crate::HOME.get().unwrap();

    for (src_rel, name) in BROWSERS {
        let src = format!("{}/{}", home, src_rel);
        if Path::new(&src).is_dir() {
            e(&format!("  {}Backing up {}...{}", Y, name, N));
            let sm = format!("{} browser", name);
            let _ = copy_progress(
                &format!("rclone copy '{}/' '{}/{}/' {}", src, b_dest, name, bx),
                ck, true, true, false, Some(&sm),
            );
        }
    }
}

pub fn backup_vm(dest: &str, ck: u32) {
    e(&format!("{}--- Backing up VM data ---{}", M, N));
    let vm_dest = format!("{}/virt-manager", dest);
    let _ = std::fs::create_dir_all(&vm_dest);

    if Path::new(VM_QEMU_SRC).is_dir() {
        e(&format!("  {}Backing up libvirt VM configs...{}", Y, N));
        let _ = run(&format!("sudo cp -a '{}' '{}/' 2>/dev/null", VM_QEMU_SRC, vm_dest));
    }
    if Path::new(VM_IMAGES_SRC).is_dir() {
        let imgsz = run_stdout(&format!("sudo du -sh '{}' | cut -f1", VM_IMAGES_SRC));
        e(&format!("  {}VM disk images:{} {}{}{}", C, N, W, imgsz, N));
        e(&format!("  {}Syncing...{}", Y, N));
        let _ = copy_progress(
            &format!("sudo rclone copy '{}/' '{}/images/' --inplace", VM_IMAGES_SRC, vm_dest),
            ck, true, false, true, Some("VM images"),
        );
    }
}

pub fn backup_home(dest: &str, ck: u32) {
    e(&format!("{}--- Backing up home data ---{}", M, N));
    e(&format!("  {}Source:{} ~/ (full home, excluded: .cache, node_modules, etc.)", C, N));
    e(&format!("  {}Target:{} {}/home", C, N, dest));
    e(&format!("  {}Scanning home directory...{}", Y, N));
    let home_dest = format!("{}/home", dest);
    let _ = std::fs::create_dir_all(&home_dest);

    let hx: Vec<String> = HOME_EXCLUDES.iter()
        .map(|x| format!("--exclude '{}'", x))
        .collect();
    let hx = hx.join(" ");
    let _ = copy_progress(
        &format!("sudo rclone copy ~/ '{}' --links --inplace {}", home_dest, hx),
        ck, false, false, true, Some("Scanning home"),
    );
}

pub fn backup_extra(dest: &str, ck: u32) {
    let dirs = extra_backup_dirs();
    if dirs.is_empty() { return; }
    e(&format!("{}--- Backing up extra dirs ---{}", M, N));
    let extra_dest = format!("{}/extra", dest);
    let _ = std::fs::create_dir_all(&extra_dest);
    for src in dirs {
        let p = Path::new(&src);
        if !p.is_dir() {
            e(&format!("  {}  Skipping {} (not found){}", Y, src, N));
            continue;
        }
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        let target = format!("{}/{}", extra_dest, name);
        let sm = format!("Extra: {}", name);
        e(&format!("  {}Backing up {}...{}", Y, name, N));
        let _ = copy_progress(
            &format!("rclone copy '{}/' '{}/'", src, target),
            ck, false, false, true, Some(&sm),
        );
    }
}

pub fn do_backup(dest: &str, auto_yes: bool) -> anyhow::Result<()> {
    let dest = std::path::Path::new(dest);
    let dest_str = dest.to_string_lossy().to_string();
    let base_mount = dest.parent().and_then(|p| p.parent()).map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Check mount
    if !run_ok(&format!("findmnt -n '{}'", base_mount)) {
        e(&format!("  {}Error: backup drive not mounted at {}{}", R, base_mount, N));
        e(&format!("  {}Mount the drive and try again.{}", Y, N));
        std::process::exit(1);
    }
    let _ = std::fs::create_dir_all(dest);

    let complete_marker = dest.join(".complete");
    init_log(format!("{}/backup.log", dest_str));
    e(&format!("{}Log:{} {}{}{}", C, N, Y, dest.join("backup.log").display(), N));

    e(&format!("{}Backing up to:{} {}{}{}", C, N, W, dest.display(), N));
    if complete_marker.exists() {
        e(&format!("  {}Warning: backup already exists at this location{}", Y, N));
        if !auto_yes {
            print!("  Overwrite existing backup? [y/N] ");
            std::io::stdout().flush().ok();
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).ok();
            if buf.trim().to_lowercase() != "y" {
                e(&format!("  {}Cancelled.{}", Y, N));
                return Ok(());
            }
        }
    }

    let ck = detect_checkers(&dest_str);
    e(&format!("  {}Checkers:{} {}{}{}", C, N, W, ck, N));

    // Run gdu estimation in parallel with package lists
    let gdu_handle = thread::spawn(|| estimate_home_size());
    save_package_lists(&dest_str);
    let _ = gdu_handle.join();

    backup_config(&dest_str, ck);
    backup_browsers(&dest_str, ck);
    backup_vm(&dest_str, ck);
    backup_home(&dest_str, ck);
    backup_extra(&dest_str, ck);

    // Summary
    let sz_out = run_stdout(&format!("du -sh '{}' | cut -f1", dest_str));
    e(&format!("  {}=============================={}", G, N));
    e(&format!("  {}{}Backup complete!{}", W, W, N));
    e(&format!("  {}Size:{} {}{}{}", C, N, W, sz_out, N));
    e(&format!("  {}Location:{} {}{}{}", C, N, W, dest.display(), N));
    e(&format!("  {}=============================={}", G, N));
    let _ = std::fs::write(&complete_marker, "");
    e(&format!("  {}To restore:{} {} --restore {}", Y, N, std::env::args().next().unwrap_or_default(), dest_str));

    Ok(())
}
