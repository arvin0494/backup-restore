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

// ── BACKUP BASE ────────────────────────────────────────────
// Where to save the backup. Uses the user's config if set,
// otherwise falls back to the default (/mnt/HDD4T/BACKUP).
pub fn backup_base() -> String {
    let cfg = load_user_config();
    cfg.get("BACKUP_BASE").cloned().unwrap_or_else(|| BACKUP_BASE_DEFAULT.to_string())
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

// ── ANDROID SKIP DIRECTORIES ───────────────────────────────
// Which media directories to skip when backing up Android.
// Set in the config file with ANDROID_SKIP_DIRS.
// Example: ANDROID_SKIP_DIRS=Music,Download
pub fn android_skip_dirs() -> Vec<String> {
    let cfg = load_user_config();
    cfg.get("ANDROID_SKIP_DIRS")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
        .unwrap_or_default()
}

// ── ANDROID FTP CONFIG ─────────────────────────────────────
// If set, the tool uses rclone copy via FTP instead of adb pull.
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

fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(home).join(".config").join("backup-restore").join("config")
}

// ── BROWSER PROFILES ───────────────────────────────────────
// Which browsers to back up. Each entry is (folder, display name).
pub const BROWSERS: &[(&str, &str)] = &[
    (".mozilla", "mozilla"),
    (".config/chromium", "chromium"),
    (".config/google-chrome", "google-chrome"),
    (".config/BraveSoftware", "BraveSoftware"),
];

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
pub const MANIFEST_FILE: &str = "~/.local/share/backup-restore/manifest";

pub fn manifest_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    MANIFEST_FILE.replacen('~', &home, 1)
}
