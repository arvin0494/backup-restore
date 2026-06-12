# backup-restore

Backup your Linux system before reinstalling, then restore everything after.

## Features

- **Package lists** — pacman official, AUR (yay), flatpak, snap
- **Configs** — `~/.config`, `~/.ssh`, `~/.gnupg`, keyrings (caches/trash excluded)
- **Browser data** — Firefox, Chromium, Chrome, Brave (profiles, caches excluded)
- **Home data** — full `~/` via `sudo rclone` (excludes `.cache`, `node_modules`, `Games/`, etc.)
- **Size estimation** — `gdu` scans home subdirs in parallel with package lists
- **Virt-manager** — libvirt VM configs (`/etc/libvirt/qemu`) and disk images (`/var/lib/libvirt/images`)
- **Android backup** — SMS, contacts, call logs, installed apps, device properties
- **Android media via ADB or FTP** — DCIM, Download, Pictures, Movies, Music, MIUI
- **FTP mode** — rclone copy over FTP for incremental, skip-unchanged Android backups
- **Smart ADB re-run** — already-downloaded directories are skipped, near-instant re-runs
- **`ANDROID_SKIP_DIRS`** — exclude unwanted media directories from Android backup
- **Auto-detect path** — `/mnt/HDD4T/BACKUP/{hostname}[-{os_id}]`
- **Live progress** — rclone `--progress` with file names, speed, ETA
- **Drive-aware** — `--checkers` / `--transfers` tuned to HDD (3), SSD (8), or NVMe (16)
- **Robust cancellation** — Ctrl+C kills the entire rclone process group, not just the script
- **Logging** — `backup.log` written alongside every backup
- **Restore with fzf** — checkbox-style multi-select (falls back to numbered menu)

## Get the tool

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

## Dependencies

Auto‑installed by `install.sh` (pacman, apt, dnf, zypper, apk):

- `rclone` — fast cloud/local sync with progress display
- `gdu` — parallel disk usage estimation
- `fzf` — fuzzy multi-select for restore

## Usage

After `install.sh` you can use the `bckup` command directly.

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
ANDROID_SKIP_DIRS=Music,Download
```

Only the keys you specify need to be included — missing keys fall back to the built-in defaults above.

`BACKUP_EXTRA_DIRS` takes a comma-separated list of directories. Each is backed up to `dest/extra/<basename>/` and shown as a separate item in the restore menu.

`ANDROID_FTP_*` enables FTP-based Android backup (rclone copy, incremental). `ANDROID_SKIP_DIRS` excludes directories from Android backup.

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

### Android

```bash
# Backup Android device (ADB must be connected)
bckup --device android -b

# Restore Android device
bckup --device android -r
```

#### FTP mode (faster incremental backups)

Add to `~/.config/backup-restore/config`:

```ini
ANDROID_FTP_HOST=192.168.44.13
ANDROID_FTP_PORT=2121
ANDROID_FTP_USER=ftp
ANDROID_FTP_PASS=0000
```

Start an FTP server on your phone (CX File Explorer → Network → FTP, or any FTP server app). The tool uses `rclone copy` over FTP — only new/changed files are transferred on re-runs.

#### Skip directories

```ini
ANDROID_SKIP_DIRS=Music,Download,MIUI
```

### Interactive menu

Run without arguments:

```bash
bckup
```

## Uninstall

```bash
bash ~/.local/share/backup-restore/uninstall.sh
```

Or re-run the curl one-liner to reinstall.

## Notes

- Config file at `~/.config/backup-restore/config` overrides built-in paths
- `sudo` is required for home backup (permissions, symlinks) and VM data
- NTFS destination: uses `--inplace` to avoid ENOSPC from ntfs-3g temp files; if ENOSPC occurs, run `sudo ntfsfix /dev/sda1`
- `.steam` and `.var` are intentionally kept in home backup (user data, not cache)
- `Games/` is excluded from home backup — use [`backup-games`](https://github.com/arvin0494/backup-games) separately for game data
- gdu estimation scans home directories (Documents, Pictures, Projects, etc.) ignoring `.cache`, `node_modules`, etc.
- Incomplete backups (missing `.complete` marker) are auto-cleaned on next run
