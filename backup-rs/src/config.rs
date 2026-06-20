// ─────────────────────────────────────────────────────────────
// CONFIG — settings and defaults that control what gets backed up
// ─────────────────────────────────────────────────────────────
// This file stores:
// - default paths (where to save backups)
// - which folders to include / exclude from backup
// - settings the user can override in their config file
// ─────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::path::PathBuf;

// Default backup location (if the user doesn't set one)
pub const BACKUP_BASE_DEFAULT: &str = "/mnt/HDD4T/BACKUP";
// Default paths for virtual machine files
pub const VM_QEMU_SRC: &str = "/etc/libvirt/qemu";
pub const VM_IMAGES_SRC: &str = "/var/lib/libvirt/images";

// ── LOAD USER CONFIG ───────────────────────────────────────
// Reads the config file at ~/.config/backup-restore/config.
// This file lets the user override the built-in defaults.
// Lines starting with # are skipped (they're just comments).
pub fn load_user_config() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let path = get_config_path();
    if !path.exists() {
        return map;
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

// ── FIND BEST DRIVE ────────────────────────────────────────
// Checks if a drive letter exists and has at least 1GB free space.
fn drive_has_space(drive: &str) -> bool {
    let path = format!("{}\\", drive);
    let ok = std::fs::metadata(&path).is_ok();
    if !ok { return false; }
    // Try to get free space via a simple file write test
    let test_path = format!("{}\\.space_test", drive);
    let written = std::fs::write(&test_path, "x").is_ok();
    let _ = std::fs::remove_file(&test_path);
    written
}

// ── BACKUP BASE ────────────────────────────────────────────
// Where to save the backup. Uses the user's config if set,
// otherwise falls back to platform defaults.
//
// Linux: /mnt/HDD4T/BACKUP
// Windows: tries E:\BACKUP, F:\BACKUP, D:\BACKUP, C:\BACKUP
pub fn backup_base() -> String {
    let cfg = load_user_config();
    if let Some(base) = cfg.get("BACKUP_BASE") {
        return base.clone();
    }
    
    if crate::util::detect_platform() == "windows" {
        for drive in &["E:", "F:", "D:", "C:"] {
            if drive_has_space(drive) {
                return format!("{}\\BACKUP", drive);
            }
        }
        return "C:\\BACKUP".to_string();
    }
    
    BACKUP_BASE_DEFAULT.to_string()
}

// ── EXTRA BACKUP DIRECTORIES ───────────────────────────────
// Returns any extra folders the user wants to back up,
// as a list. These are set in the config with BACKUP_EXTRA_DIRS.
pub fn extra_backup_dirs() -> Vec<String> {
    let cfg = load_user_config();
    cfg.get("BACKUP_EXTRA_DIRS")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
        .unwrap_or_default()
}

// ── ANDROID FTP CONFIG ─────────────────────────────────────
// The tool uses rclone copy via FTP (adb pull is not used for backup).
// The FTP server (e.g. CX File Explorer) must be running on the phone.
// Example:
//   ANDROID_FTP_HOST=192.168.44.13
//   ANDROID_FTP_PORT=5502
//   ANDROID_FTP_USER=ftp
//   ANDROID_FTP_PASS=1111
pub fn android_ftp_host() -> Option<String> {
    let cfg = load_user_config();
    cfg.get("ANDROID_FTP_HOST").cloned()
}
pub fn android_ftp_port() -> String {
    let cfg = load_user_config();
    cfg.get("ANDROID_FTP_PORT").cloned().unwrap_or_else(|| "2121".to_string())
}
pub fn android_ftp_user() -> String {
    let cfg = load_user_config();
    cfg.get("ANDROID_FTP_USER").cloned().unwrap_or_else(|| "ftp".to_string())
}
pub fn android_ftp_pass() -> String {
    let cfg = load_user_config();
    cfg.get("ANDROID_FTP_PASS").cloned().unwrap_or_else(|| "0000".to_string())
}
pub fn mihon_path() -> String {
    let cfg = load_user_config();
    if let Some(base) = cfg.get("MIHON_PATH") {
        return base.clone();
    }
    if crate::util::detect_platform() == "windows" {
        for drive in &["E:", "F:", "D:", "C:"] {
            if drive_has_space(drive) {
                return format!("{}\\Mihon", drive);
            }
        }
        "C:\\Mihon".to_string()
    } else {
        "/mnt/HDD4T/Mihon".to_string()
    }
}

fn get_config_path() -> PathBuf {
    let home = crate::HOME.get().cloned().unwrap_or_else(|| {
        std::env::var("HOME").unwrap_or_else(|_| "/root".into())
    });
    PathBuf::from(home).join(".config").join("backup-restore").join("config")
}

pub fn edit_config() {
    let path = get_config_path();

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let template = r#"# ── CONFIG for backup-restore ──────────────────────
# Uncomment and set any values you want to override.
# Lines starting with # are ignored.

# ── Backup destination ──
# Default: /mnt/HDD4T/BACKUP (Linux) or D:\BACKUP (Windows)
# BACKUP_BASE=/mnt/HDD4T/BACKUP

# ── Extra directories to back up (comma-separated, relative to ~/) ──
# Default: empty (no extra dirs)
# BACKUP_EXTRA_DIRS=

# ── Android FTP backup (IP of phone running CX File Explorer) ──
# Default: skipped (SMS/contacts still back up)
# ANDROID_FTP_HOST=
# Default: 2121
# ANDROID_FTP_PORT=2121
# Default: ftp
# ANDROID_FTP_USER=ftp
# Default: 0000
# ANDROID_FTP_PASS=0000

# ── Mihon manga backup path ──
# Default: /mnt/HDD4T/Mihon (Linux) or D:\Mihon (Windows)
# MIHON_PATH=/mnt/HDD4T/Mihon
"#;

    // Preserve any active (uncommented) lines from existing config
    let mut active = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.contains('=') {
                active.push(trimmed.to_string());
            }
        }
    }

    let mut output = template.to_string();
    if !active.is_empty() {
        output.push_str("\n# ── Active user overrides ──\n");
        for line in active {
            output.push_str(&line);
            output.push('\n');
        }
    }

    let _ = std::fs::write(&path, &output);

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            for cmd in &["nvim", "vim", "nano"] {
                if std::process::Command::new("which")
                    .arg(cmd)
                    .stdout(std::process::Stdio::null())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    return cmd.to_string();
                }
            }
            "nano".to_string()
        });

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .expect("failed to launch editor");

    if !status.success() {
        crate::util::e(&format!("Editor exited with error: {}", status));
    }
}

// ── BROWSER PROFILES ───────────────────────────────────────
// Linux: stores browser data in ~/.config/ or ~/.mozilla
pub const BROWSERS_LINUX: &[(&str, &str)] = &[
    (".mozilla", "mozilla"),
    (".config/chromium", "chromium"),
    (".config/google-chrome", "google-chrome"),
    (".config/BraveSoftware", "BraveSoftware"),
];

// Windows: stores browser data in %APPDATA% or %LOCALAPPDATA%
pub const BROWSERS_WINDOWS: &[(&str, &str)] = &[
    ("AppData\\Roaming\\Mozilla", "mozilla"),
    ("AppData\\Local\\Chromium\\User Data", "chromium"),
    ("AppData\\Local\\Google\\Chrome\\User Data", "google-chrome"),
    ("AppData\\Local\\BraveSoftware\\Brave-Browser\\User Data", "BraveSoftware"),
];

// Get browser list for current platform
pub fn browsers() -> &'static [(&'static str, &'static str)] {
    if crate::util::detect_platform() == "windows" {
        BROWSERS_WINDOWS
    } else {
        BROWSERS_LINUX
    }
}


// ── CACHE EXCLUDES ─────────────────────────────────────────
// Names of cache folders to skip when copying (they're just
// temporary files that will be recreated automatically).
pub const CACHE_EXCLUDES: &[&str] = &["Cache", "cache", "Caches", "Crash Reports", "crashpad"];

// ── HOME EXCLUDES ──────────────────────────────────────────
// Folders and files inside ~/ that should NOT be backed up.
// These are things like caches, build artifacts, trash,
// virtual machines, package managers, and temporary files.
pub const HOME_EXCLUDES: &[&str] = &[
    ".cache/", ".local/share/Trash/", ".thumbnails/",
    "*__pycache__/", "*.pyc", "node_modules/", "target/", ".next/",
    "snap/", ".local/share/flatpak/", ".npm/", ".cargo/", ".rustup/",
    ".gradle/", ".m2/", "VirtualBox VMs/", ".vagrant.d/",
    "Cache/", "Code Cache/", "GPUCache/", "Caches/",
    "Games/",
    "*~", "*.bak", "*.swp",
];

// ── CONFIG EXCLUDES ────────────────────────────────────────
// Folders inside ~/.config that should be skipped. Browser
// profiles are excluded here because they're handled separately
// (see backup_browsers). Also skips large app data, logs, etc.
pub const CONFIG_EXCLUDES: &[&str] = &[
    "Trash", "trash", "Session", "sessions",
    "tmp", "temp", "thumbnails", "thumbcache", "logs", "Logs",
    "node_modules/", "*.bak", "*~",
    // Browser profiles — handled separately by backup_browsers
    "google-chrome/", "chromium/", "BraveSoftware/", "mozilla/",
    "firefox/", "librewolf/",
    // Large app data
    "Code/", "Code - OSS/", "VSCodium/",
    "discord/", "Slack/", "spotify/",
    "vesktop/", "Vesktop/",
];

// ── BROWSER EXCLUDES ───────────────────────────────────────
// Sub-folders inside browser profiles to skip (rendering
// caches and other temporary data).
pub const BROWSER_EXCLUDES: &[&str] = &[
    "GPUCache", "Code Cache", "Dictionaries", "Safe Browsing",
];

// ── GDU IGNORE DIRS ────────────────────────────────────────
// Directories to ignore when estimating disk usage with gdu.
pub const GDU_IGNORE_DIRS: &[&str] = &[
    ".cache", "node_modules", "target", ".next", "snap",
    ".npm", ".cargo", ".rustup", ".gradle", ".m2",
    "VirtualBox VMs", ".vagrant.d", ".thumbnails",
    "flatpak", "Trash", "Cache", "Code Cache", "GPUCache", "Caches",
];

// ── GDU SCAN DIRS ──────────────────────────────────────────
// Which folders inside ~/ to scan for size estimation.
pub const GDU_SCAN_DIRS: &[&str] = &[
    "Documents", "Pictures", "Music", "Videos", "Downloads", "Desktop",
    "Projects", "Templates", "Public", "Games",
    ".local", ".fonts", ".themes", ".icons",
];

// ── MANIFEST FILE ──────────────────────────────────────────
// A small file that remembers which folders were already
// backed up and when they last changed. This lets the program
// skip unchanged folders on subsequent backups.
pub const MANIFEST_FILE_LINUX: &str = "~/.local/share/backup-restore/manifest";

pub fn manifest_path() -> String {
    if crate::util::detect_platform() == "windows" {
        std::env::var("APPDATA").map(|a| format!("{}\\backup-restore\\manifest", a)).unwrap_or_else(|_| "C:\\Windows\\manifest".to_string())
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        MANIFEST_FILE_LINUX.replacen('~', &home, 1)
    }
}
