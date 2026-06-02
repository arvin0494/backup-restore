use crate::config::BROWSERS;
use crate::util::*;
use std::io::Write;
use std::path::Path;

type Item = (String, String, Option<Box<dyn FnOnce()>>);

pub fn do_restore(backup_dir: &str, dest_dir: &str, auto: bool) -> anyhow::Result<()> {
    let backup_dir = std::path::absolute(backup_dir)?;
    let dest_dir = std::path::absolute(dest_dir)?;
    let _ = std::fs::create_dir_all(&dest_dir);

    e(&format!("{}Backup:{} {}{}{}", C, N, W, backup_dir.display(), N));
    e(&format!("{}Restore to:{} {}{}{}", C, N, W, dest_dir.display(), N));
    let ck = detect_checkers(&dest_dir.to_string_lossy());
    e(&format!("  {}Checkers:{} {}{}{}", C, N, W, ck, N));

    if !backup_dir.is_dir() {
        e(&format!("{}Error: backup directory not found{}", R, N));
        std::process::exit(1);
    }

    let bd = backup_dir.to_string_lossy().to_string();
    let dd = dest_dir.to_string_lossy().to_string();
    let mut items: Vec<Item> = Vec::new();

    // Package lists — distro-agnostic
    // Arch
    let pac_off = format!("{}/packages-pacman-official.txt", bd);
    let pac_off_old = format!("{}/pacman-official.txt", bd);
    if (Path::new(&pac_off).exists() || Path::new(&pac_off_old).exists()) && run_ok("which pacman") {
        let d = bd.clone();
        items.push(("official-pkgs".into(), "Install official packages (pacman)".into(), Some(Box::new(move || {
            let f = if Path::new(&format!("{}/packages-pacman-official.txt", d)).exists() { "packages-pacman-official.txt" } else { "pacman-official.txt" };
            let _ = run(&format!("sudo pacman -S --needed - < '{}/{}'", d, f));
        }))));
    }
    let pac_aur = format!("{}/packages-aur.txt", bd);
    let pac_aur_old = format!("{}/pacman-aur.txt", bd);
    if (Path::new(&pac_aur).exists() || Path::new(&pac_aur_old).exists()) && run_ok("which yay") {
        let d = bd.clone();
        items.push(("aur-pkgs".into(), "Install AUR packages (yay)".into(), Some(Box::new(move || {
            let f = if Path::new(&format!("{}/packages-aur.txt", d)).exists() { "packages-aur.txt" } else { "pacman-aur.txt" };
            let _ = run(&format!("yay -S --needed - < '{}/{}'", d, f));
        }))));
    }
    // Debian / Ubuntu
    if Path::new(&format!("{}/packages-dpkg.txt", bd)).exists() && run_ok("which dpkg") {
        let d = bd.clone();
        items.push(("dpkg-pkgs".into(), "Install packages (dpkg/apt)".into(), Some(Box::new(move || {
            let _ = run(&format!("sudo apt-get update && sudo apt-get install -y $(awk '{{print $1}}' '{}/packages-dpkg.txt')", d));
        }))));
    }
    // Fedora
    if Path::new(&format!("{}/packages-dnf.txt", bd)).exists() && run_ok("which dnf") {
        let d = bd.clone();
        items.push(("dnf-pkgs".into(), "Install packages (dnf)".into(), Some(Box::new(move || {
            let _ = run(&format!("sudo dnf install -y $(cat '{}/packages-dnf.txt')", d));
        }))));
    }
    // openSUSE
    if Path::new(&format!("{}/packages-zypper.txt", bd)).exists() && run_ok("which zypper") {
        let d = bd.clone();
        items.push(("zypper-pkgs".into(), "Install packages (zypper)".into(), Some(Box::new(move || {
            let _ = run(&format!("sudo zypper install -y $(cat '{}/packages-zypper.txt')", d));
        }))));
    }
    // Alpine
    if Path::new(&format!("{}/packages-apk.txt", bd)).exists() && run_ok("which apk") {
        let d = bd.clone();
        items.push(("apk-pkgs".into(), "Install packages (apk)".into(), Some(Box::new(move || {
            let _ = run(&format!("sudo apk add $(cat '{}/packages-apk.txt')", d));
        }))));
    }
    // Cross-platform
    if Path::new(&format!("{}/flatpak-list.txt", bd)).exists() && run_ok("which flatpak") {
        let d = bd.clone();
        items.push(("flatpaks".into(), "Install Flatpaks".into(), Some(Box::new(move || { let _ = run(&format!("xargs flatpak install -y < '{}/flatpak-list.txt'", d)); }))));
    }

    // Config
    if Path::new(&format!("{}/config", bd)).is_dir() {
        let (a, b) = (bd.clone(), dd.clone());
        items.push(("config".into(), "Restore ~/.config".into(), Some(Box::new(move || { let _ = run(&format!("rclone copy '{}/config/' '{}/.config/' --checkers {}", a, b, ck)); }))));
    }

    // Browser profiles
    for (src_rel, name) in BROWSERS {
        let p = format!("{}/browser/{}", bd, name);
        if Path::new(&p).is_dir() {
            let (src, dst) = (p.clone(), dd.clone());
            let rd = src_rel.to_string();
            items.push((format!("browser-{}", name), format!("Restore {}", name), Some(Box::new(move || { let _ = run(&format!("rclone copy '{}/' '{}/{}/' --checkers {} 2>/dev/null", src, dst, rd, ck)); }))));
        }
    }

    // SSH keys & GPG
    for name in &[".ssh", ".gnupg"] {
        let p = format!("{}/{}", bd, name);
        if Path::new(&p).is_dir() {
            let (src, dst) = (p.clone(), dd.clone());
            let n = name.to_string();
            items.push((name.trim_start_matches('.').to_string(), format!("Restore ~/{}", name), Some(Box::new(move || { let _ = run(&format!("rclone copy '{}/' '{}/{}/' --checkers {} 2>/dev/null", src, dst, n, ck)); }))));
        }
    }

    // Keyrings
    let keyrings = format!("{}/keyrings", bd);
    if Path::new(&keyrings).is_dir() {
        let (src, dst) = (keyrings.clone(), dd.clone());
        items.push(("keyrings".into(), "Restore keyrings (~/.local/share/keyrings)".into(), Some(Box::new(move || { let _ = run(&format!("rclone copy '{}/' '{}/.local/share/keyrings/' --checkers {} 2>/dev/null", src, dst, ck)); }))));
    }

    // VM configs
    let vm_qemu = format!("{}/virt-manager/qemu", bd);
    if Path::new(&vm_qemu).is_dir() {
        let d = bd.clone();
        items.push(("vm-configs".into(), "Restore libvirt VM configs (/etc/libvirt/qemu)".into(), Some(Box::new(move || { let _ = run(&format!("sudo rclone copy '{}/virt-manager/qemu/' /etc/libvirt/qemu/ --checkers {} 2>/dev/null", d, ck)); }))));
    }
    // VM images
    let vm_images = format!("{}/virt-manager/images", bd);
    if Path::new(&vm_images).is_dir() {
        let src = vm_images.clone();
        items.push(("vm-images".into(), "Restore VM disk images (/var/lib/libvirt/images)".into(), Some(Box::new(move || { let _ = run(&format!("sudo rclone copy '{}/' /var/lib/libvirt/images/ --checkers {} 2>/dev/null", src, ck)); }))));
    }

    // Home subdirectories
    let home_src = format!("{}/home", bd);
    if Path::new(&home_src).is_dir() {
        if let Ok(entries) = std::fs::read_dir(&home_src) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let sub = entry.file_name().to_string_lossy().to_string();
                    let (a, b) = (bd.clone(), dd.clone());
                    let s = sub.clone();
                    items.push((format!("home-{}", sub), format!("Restore ~/{}", sub), Some(Box::new(move || { let _ = run(&format!("rclone copy '{}/home/{}/' '{}/{}/' --checkers {} 2>/dev/null", a, s, b, s, ck)); }))));
                }
            }
        }
    }

    if items.is_empty() {
        e(&format!("{}Nothing found to restore in that directory{}", R, N));
        std::process::exit(1);
    }

    // Selection
    let keys: Vec<String> = items.iter().map(|(k, _, _)| k.clone()).collect();
    let labels: Vec<&str> = items.iter().map(|(_, l, _)| l.as_str()).collect();

    let chosen: Vec<usize> = if auto {
        (0..items.len()).collect()
    } else if run_ok("which fzf") {
        let input: String = keys.iter().zip(labels.iter())
            .map(|(k, l)| format!("{}|{}", k, l))
            .collect::<Vec<_>>()
            .join("\n");
        let result = run_stdin(
            "fzf --multi --prompt='Restore > ' --with-nth=2 -d'|' --height=60% --border",
            &input,
        ).unwrap_or_default();
        result.lines()
            .filter_map(|line| {
                let k = line.split('|').next().unwrap_or("");
                keys.iter().position(|x| x == k)
            })
            .collect()
    } else {
        e(&format!("  {}Select items to restore:{}", Y, N));
        for (i, label) in labels.iter().enumerate() {
            e(&format!("  {}{}){} {}", C, i + 1, N, label));
        }
        print!("  Choose (space-separated numbers, or 'all'): ");
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        let buf = buf.trim().to_lowercase();
        if buf == "all" {
            (0..items.len()).collect()
        } else {
            buf.split_whitespace()
                .filter_map(|s| s.parse::<usize>().ok().map(|i| i - 1))
                .filter(|&i| i < items.len())
                .collect()
        }
    };

    if chosen.is_empty() {
        e(&format!("  {}Nothing selected.{}", Y, N));
        return Ok(());
    }

    e(&format!("  {}Restoring:{} {}{}{}", W, N, Y,
        chosen.iter().map(|&i| labels[i]).collect::<Vec<_>>().join(", "), N));

    if !auto {
        print!("  Proceed? [Y/n] ");
        std::io::stdout().flush().ok();
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).ok();
        if buf.trim().to_lowercase() == "n" {
            e(&format!("  {}Cancelled.{}", Y, N));
            return Ok(());
        }
    }

    // Execute each selected item
    for &i in &chosen {
        if let Some((_, desc, ref mut cb_opt)) = items.get_mut(i) {
            let desc = desc.clone();
            e(&format!("{}--- {} ---{}", M, desc, N));
            if let Some(cb) = cb_opt.take() {
                cb();
            }
        }
    }

    e(&format!("  {}=============================={}", G, N));
    e(&format!("  {}{}Restore complete!{}", W, W, N));
    e(&format!("  {}=============================={}", G, N));

    Ok(())
}
