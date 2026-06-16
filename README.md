# backup-restore

Backup your Linux system before reinstalling, then restore everything after — including on Windows.

## Features

- **Package lists** — pacman official, AUR (yay), flatpak, snap, dpkg, dnf, zypper, apk
- **Configs** — `~/.config`, `~/.ssh`, `~/.gnupg`, keyrings (caches/trash excluded)
- **Browser data** — Firefox, Chromium, Chrome, Brave (profiles, caches excluded)
- **Home data** — full `~/` via `rclone` with progress (excludes `.cache`, `node_modules`, `Games/`, etc.)
- **Size estimation** — `gdu` scans home subdirs in parallel with package lists
- **Virt-manager** — libvirt VM configs and disk images
- **Android backup** — SMS, contacts, call logs, installed apps, device properties, media via FTP
- **Incremental** — only new/changed files are transferred on re-runs
- **Auto-detect path** — Linux: `/mnt/HDD4T/BACKUP/{hostname}[-{os_id}]`, Windows: `D:\BACKUP\{hostname}`
- **Live progress** — rclone `--progress` with file names, speed, ETA
- **Drive-aware** — checkers/transfers tuned to HDD (1), SSD (8), or NVMe (16)
- **Robust cancellation** — Ctrl+C kills the entire rclone process group, not just the script
- **Logging** — `backup.log` written alongside every backup
- **Restore with fzf** — checkbox-style multi-select (falls back to numbered menu)
- **Cross-platform** — backup on Linux, restore on Windows (skips Linux-specific dirs like `.config`, `.local`, `.var`)
- **NO_COLOR support** — respects `NO_COLOR` env var (auto-set on Windows)

## Get the tool

### Linux

Quick one-liner:

```bash
curl -fsSL https://raw.githubusercontent.com/arvin0494/backup-restore/main/install.sh | bash
```

Clone the repo, then run the installer:

```bash
git clone git@github.com:arvin0494/backup-restore.git
cd backup-restore
bash install.sh
```

The installer checks for Rust, offers to install it (rustup or system package manager), builds the binary, and adds the `bckup` alias to your shell.

### Windows

Quick one-liner (PowerShell):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/arvin0494/backup-restore/main/install.ps1 | iex"
```

This downloads `rclone` and `fzf`, installs Rust via `rustup`, builds or downloads the binary, and adds the `bckup` alias to your PowerShell profile with `NO_COLOR` set.

If you prefer cloning first:

```powershell
git clone git@github.com:arvin0494/backup-restore.git
cd backup-restore
.\install.ps1
```

Requires administrator access (for PATH modification). Chocolatey and MinGW are auto-installed if needed for local compilation.

## Dependencies

### Linux

Auto-installed by `install.sh` (pacman, apt, dnf, zypper, apk):

- `rclone` — fast cloud/local sync with progress display
- `gdu` — parallel disk usage estimation
- `fzf` — fuzzy multi-select for restore

### Windows

Installed by `install.ps1` from bundled `deps/` folder or downloaded on demand:

- `rclone` — file sync
- `fzf` — fuzzy selection
- `Rust` (via rustup) — compilation

## Usage

After install, use the `bckup` command directly on both Linux and Windows.

### Configuration

Create `~/.config/backup-restore/config` to override defaults:

```ini
BACKUP_BASE=/mnt/HDD4T/BACKUP
VM_QEMU_SRC=/etc/libvirt/qemu
VM_IMAGES_SRC=/var/lib/libvirt/images
BACKUP_EXTRA_DIRS=/path/to/something,/another/path
ANDROID_FTP_HOST=192.168.44.13
ANDROID_FTP_PORT=2121
ANDROID_FTP_USER=ftp
ANDROID_FTP_PASS=0000
```

Only the keys you specify need to be included — missing keys fall back to the built-in defaults above.

`BACKUP_EXTRA_DIRS` takes a comma-separated list of directories. Each is backed up to `dest/extra/<basename>/` and shown as a separate item in the restore menu.

`ANDROID_FTP_*` configures FTP-based Android backup (required).

### Backup

```bash
# Auto-detect destination
bckup -b

# Specify destination
bckup -b /path/to/backup

# Non-interactive mode
bckup -b -y
```

### Restore

```bash
# Interactive restore with fzf (select items to restore)
bckup -r /path/to/backup

# Restore everything without prompts
bckup -r /path/to/backup -y
```

Cross-platform: restore a Linux backup on Windows — hidden Linux-only directories (`.config`, `.local`, `.var`) are automatically skipped.

### Android

```bash
# Backup Android device (ADB + FTP server required)
bckup --device android -b

# Restore Android device
bckup --device android -r
```

Set `ANDROID_FTP_HOST` (and optionally PORT/USER/PASS) in config.
Start an FTP server on your phone (CX File Explorer → Network → FTP, or any FTP server app).
The tool uses `rclone copy` over FTP — only new/changed files are transferred on re-runs.

### Interactive menu

Run without arguments:

```bash
bckup
```

## Uninstall

### Linux

```bash
bash ~/.local/share/backup-restore/uninstall.sh
```

### Windows

Remove the alias from your PowerShell profile (`$PROFILE`) and delete `C:\Users\<you>\bin\`.

## Build from source

### Linux

```bash
cd backup-rs
cargo build --release
```

### Windows

```bash
cd backup-rs
cargo build --release
```

Requires MSVC Build Tools or MinGW. The installer handles this automatically.

## Notes

- Config file at `~/.config/backup-restore/config` overrides built-in paths
- On Linux, `sudo` is required for home backup (permissions, symlinks) and VM data
- On Windows, `sudo` is not used — home backup runs as the current user
- NTFS destination: uses `--inplace` to avoid ENOSPC from ntfs-3g temp files; if ENOSPC occurs, run `sudo ntfsfix /dev/sda1`
- `.steam` and `.var` are kept in home backup (user data, not cache)
- `Games/` is excluded from home backup — use [`backup-games`](https://github.com/arvin0494/backup-games) separately for game data
- gdu estimation scans home directories (Documents, Pictures, Projects, etc.) ignoring `.cache`, `node_modules`, etc.
- Incomplete backups (missing `.complete` marker) are auto-cleaned on next run
- On Windows, `NO_COLOR=1` is automatically set in the PowerShell profile to disable ANSI codes
- `--links` flag is only used on Linux (Windows symlinks require admin privileges)
