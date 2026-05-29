# backup-restore

Backup your Linux system before reinstalling, then restore everything after.

## Features

- **Package lists** — pacman official, AUR (yay), flatpak, snap
- **Configs** — `~/.config` (per-directory rsync with excludes for caches/trash)
- **Browser data** — Firefox, Chromium, Chrome, Brave (profiles, excludes caches)
- **SSH keys, GPG keys, keyrings** — `~/.ssh`, `~/.gnupg`, `~/.local/share/keyrings`
- **Home data** — full `~/` via `sudo rsync` (excludes `.cache`, `node_modules`, etc.)
- **Virt-manager** — libvirt VM configs (`/etc/libvirt/qemu`) and disk images (`/var/lib/libvirt/images`)
- **Auto-detect path** — `/mnt/HDD4T/{hostname}[-{os_id}]-{YYYYMM}`
- **Progress bars** — tqdm with live file name, ETA, speed, file count
- **Logging** — `backup.log` written alongside every backup
- **Restore with fzf** — checkbox-style multi-select (falls back to numbered menu)

## Get the tool

```bash
git clone git@github.com:arvin0494/backup-restore.git
cd backup-restore
```

Or download just the script:

```bash
curl -O https://raw.githubusercontent.com/arvin0494/backup-restore/main/backup-for-reinstall.py
```

## Install

```bash
python3 backup-for-reinstall.py --setup
```

Auto-detects your package manager (pacman, apt, dnf, zypper, apk) and installs `rsync`, `gdu`, `fzf`, `python-tqdm`. Run this on both the old and new system.

## Usage

### Backup

```bash
# Auto-detect destination
python3 backup-for-reinstall.py -b

# Specify destination
python3 backup-for-reinstall.py -b /path/to/backup

# Non-interactive mode
python3 backup-for-reinstall.py -b -y
```

### Restore

```bash
# Interactive restore with fzf (select items to restore)
python3 backup-for-reinstall.py -r /path/to/backup

# Restore everything without prompts
python3 backup-for-reinstall.py -r /path/to/backup -y
```

### Interactive menu

Run without arguments for an interactive menu:

```bash
python3 backup-for-reinstall.py
```

## Notes

- `sudo` is required for home data rsync (permissions, broken symlinks)
- NTFS destination (`/mnt/HDD4T`): uses `--inplace --no-links` to avoid ENOSPC from ntfs-3g temp files. If ENOSPC occurs, run `sudo ntfsfix /dev/sda1`
- The `dirs` list in the gdu estimation step is only for size estimation — the actual rsync copies your entire `~/` (excluding listed patterns)
- Incomplete backups (missing `.complete` marker) are auto-cleaned on next run
