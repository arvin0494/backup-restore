# backup-restore

Backup your Linux system before reinstalling, then restore everything after.

## Features

- **Package lists** — pacman official, AUR (yay), flatpak, snap
- **Configs** — `~/.config`, `~/.ssh`, `~/.gnupg`, keyrings (caches/trash excluded)
- **Browser data** — Firefox, Chromium, Chrome, Brave (profiles, caches excluded)
- **Home data** — full `~/` via `sudo rclone` (excludes `.cache`, `node_modules`, etc.)
- **Size estimation** — `gdu` scans home subdirs in parallel with package lists
- **Virt-manager** — libvirt VM configs (`/etc/libvirt/qemu`) and disk images (`/var/lib/libvirt/images`)
- **Auto-detect path** — `/mnt/HDD4T/BACKUP/{hostname}[-{os_id}]`
- **Live progress** — rclone `--progress` with file names, speed, ETA
- **Drive-aware** — `--checkers` / `--transfers` tuned to HDD (3), SSD (8), or NVMe (16)
- **Robust cancellation** — Ctrl+C kills the entire rclone process group, not just Python
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

Or download just the Python script:

```bash
curl -O https://raw.githubusercontent.com/arvin0494/backup-restore/main/backup-for-reinstall.py
```

## Dependencies

**Rust version** — auto‑installed by `install.sh`.  
**Python version** — auto‑installed on first run (pacman, apt, dnf, zypper, apk):

- `rclone` — fast cloud/local sync with progress display
- `gdu` — parallel disk usage estimation
- `fzf` — fuzzy multi-select for restore
- `python-tqdm` — restore progress bar

## Usage

After `install.sh` you can use the `bckup` command directly.  
With the Python script use `python3 backup-for-reinstall.py`.

### Configuration

Create `~/.config/backup-restore/config` to override defaults:

```ini
BACKUP_BASE=/mnt/HDD4T/BACKUP
VM_QEMU_SRC=/etc/libvirt/qemu
VM_IMAGES_SRC=/var/lib/libvirt/images
```

Only the keys you specify need to be included — missing keys fall back to the built-in defaults above.

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

### Interactive menu

Run without arguments:

```bash
bckup
```

## Notes

- Config file at `~/.config/backup-restore/config` overrides built-in paths
- `sudo` is required for home backup (permissions, symlinks) and VM data
- NTFS destination: uses `--inplace` to avoid ENOSPC from ntfs-3g temp files; if ENOSPC occurs, run `sudo ntfsfix /dev/sda1`
- `.steam` and `.var` are intentionally kept in home backup (user data, not cache)
- gdu estimation scans home directories (Documents, Pictures, Projects, etc.) ignoring `.cache`, `node_modules`, etc.
- Incomplete backups (missing `.complete` marker) are auto-cleaned on next run
