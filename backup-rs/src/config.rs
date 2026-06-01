pub const BACKUP_BASE: &str = "/mnt/HDD4T/BACKUP";
pub const VM_QEMU_SRC: &str = "/etc/libvirt/qemu";
pub const VM_IMAGES_SRC: &str = "/var/lib/libvirt/images";

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
