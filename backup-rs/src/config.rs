use std::collections::HashMap;
use std::path::PathBuf;

pub const BACKUP_BASE_DEFAULT: &str = "/mnt/HDD4T/BACKUP";
pub const VM_QEMU_SRC: &str = "/etc/libvirt/qemu";
pub const VM_IMAGES_SRC: &str = "/var/lib/libvirt/images";

/// Load user config from `~/.config/backup-restore/config`, return overrides.
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

/// Resolved backup base directory — user config overrides compiled-in default.
pub fn backup_base() -> String {
    let cfg = load_user_config();
    cfg.get("BACKUP_BASE").cloned().unwrap_or_else(|| BACKUP_BASE_DEFAULT.to_string())
}

pub fn extra_backup_dirs() -> Vec<String> {
    let cfg = load_user_config();
    cfg.get("BACKUP_EXTRA_DIRS")
        .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
        .unwrap_or_default()
}

fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(home).join(".config").join("backup-restore").join("config")
}

pub const BROWSERS: &[(&str, &str)] = &[
    (".mozilla", "mozilla"),
    (".config/chromium", "chromium"),
    (".config/google-chrome", "google-chrome"),
    (".config/BraveSoftware", "BraveSoftware"),
];

pub const CACHE_EXCLUDES: &[&str] = &["Cache", "cache", "Caches", "Crash Reports", "crashpad"];

pub const HOME_EXCLUDES: &[&str] = &[
    ".cache/", ".local/share/Trash/", ".thumbnails/",
    "*__pycache__/", "*.pyc", "node_modules/", "target/", ".next/",
    "snap/", ".local/share/flatpak/", ".npm/", ".cargo/", ".rustup/",
    ".gradle/", ".m2/", "VirtualBox VMs/", ".vagrant.d/",
    "Cache/", "Code Cache/", "GPUCache/", "Caches/",
    "Games/",
    "*~", "*.bak", "*.swp",
];

pub const CONFIG_EXCLUDES: &[&str] = &[
    "Trash", "trash", "Session", "sessions",
    "tmp", "temp", "thumbnails", "thumbcache", "logs", "Logs",
    "node_modules", "*.bak", "*~",
    // Browser profiles — handled separately by backup_browsers
    "google-chrome/", "chromium/", "BraveSoftware/", "mozilla/",
    "firefox/", "librewolf/",
    // Large app data
    "Code/", "Code - OSS/", "VSCodium/",
    "discord/", "Slack/", "spotify/",
];

pub const BROWSER_EXCLUDES: &[&str] = &[
    "GPUCache", "Code Cache", "Dictionaries", "Safe Browsing",
];

pub const GDU_IGNORE_DIRS: &[&str] = &[
    ".cache", "node_modules", "target", ".next", "snap",
    ".npm", ".cargo", ".rustup", ".gradle", ".m2",
    "VirtualBox VMs", ".vagrant.d", ".thumbnails",
    "flatpak", "Trash", "Cache", "Code Cache", "GPUCache", "Caches",
];

pub const GDU_SCAN_DIRS: &[&str] = &[
    "Documents", "Pictures", "Music", "Videos", "Downloads", "Desktop",
    "Projects", "Templates", "Public", "Games",
    ".local", ".fonts", ".themes", ".icons",
];
