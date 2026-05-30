# Summary

## Goal
Python backup/restore tool for Linux reinstall, pushed to private GitHub repo `arvin0494/backup-restore` (SSH `git@github.com`).

## File
- `/home/akiiiii/Projects/test/backup-for-reinstall.py`

## Backup Path
- Auto-detect: `/mnt/HDD4T/BACKUP/{hostname}[-{os_id}]`
- OS ID omitted when it's a substring of hostname (avoids `cachyos-cachyos`)

## Features
| Feature | Detail |
|---|---|
| Package lists | pacman (official + AUR), flatpak, snap |
| Configs | `~/.config` (excludes cache/trash), `.ssh`, `.gnupg`, keyrings |
| Browser data | Firefox, Chromium, Chrome, Brave (excludes cache) |
| VMs | libvirt configs (`/etc/libvirt/qemu`) + disk images (`/var/lib/libvirt/images`) via sudo |
| Home data | Full `~/` via `sudo rsync` (excludes `.cache`, node_modules, etc.) with live progress bar |
| Progress | tqdm bar with transfer speed, file count, ETA, live filename |
| Restore | fzf multi-select (fallback: numbered menu) with confirmation |
| Auto-install deps | On startup, installs rsync, gdu, fzf, tqdm via pacman/apt/dnf/zypper/apk |

## Key Technical Details
- NTFS target (`/dev/sda1` → `/mnt/HDD4T`): `rsync --inplace --no-links` avoids ntfs-3g temp-file ENOSPC
- `sudo ntfsfix /dev/sda1` if NTFS errors
- `rsync_progress()`: parses `--info=progress2` output via regex, drives tqdm bar
- Rsync isolated via `start_new_session=True`; Ctrl+C sends SIGINT to rsync
- Auto-clears incomplete backups (home dir exists without `.complete` marker)
- Stale mount detection via `findmnt` (avoids hanging on dead mounts)
- All restore copy operations use `rsync -a` (skips identical files)

## Recent Changes in This Session
- Removed date suffix from backup folder path
- Added `BACKUP/` subdirectory in path
- Script defaults to backup immediately (no interactive menu)
- Auto-install deps on run (removed `--setup` flag)
- Transfer speed displayed in progress bar (parsed from rsync output)
- `rsync_progress` now handles both `ir-chk=` and `to-chk=` progress formats
- Fixed lowercase `k` in speed regex (`kB/s`)
- Speed persists alongside filename in bar description
- tqdm `ncols=80` to prevent garbage on terminal zoom
- Mount check uses `findmnt` instead of `os.path.isdir` (avoids hanging on stale mounts)
- `--setup` flag removed from help

## Next Steps
- Run full backup test
- Potential: per-file speed, dry-run option, ext4/btrfs for better speed
