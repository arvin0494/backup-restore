# Changelog

## v1.4.1 — 2026-10-09

- **Android backup works on Windows**: `adb` was looked up with `which`, which doesn't exist on Windows, so `--device android` always reported "no device connected". Now uses `where` on Windows
- **No crash on hostname FTP host**: setting `ANDROID_FTP_HOST` to a name instead of an IP panicked instead of reporting the bad value
- **Failed transfers no longer report success**: if every media directory failed to copy, the run still printed "Android backup complete!". Failures are now listed and the run exits non-zero (metadata is still saved)
- **FTP passwords containing quotes**: a password with `'` broke shell quoting around `rclone obscure`, producing the wrong obscured password and a silent auth failure

## v1.4.0 — 2026-06-22

- **Rclone updated** to v1.74.3 (was v1.71.0)

## v1.3.0 — 2026-06-16

- **Bundled Windows deps**: Chocolatey installer & 7za in `deps/` — offline choco setup
- **Admin elevation fix**: one-liner `iex` install now properly elevates to Administrator
- **Config fix**: Windows config path respects `HOME` global, no longer defaults to `/root`
- **Keyrings restore**: path now matches backup location — keyrings are found and restored
- **Incomplete backup cleanup**: directories missing `.complete` marker are auto-removed on re-run

## v1.2.0 — 2026-06-15

- **HDD backup tuning**: reduced checkers/transfers from 3 to 1 for rotational drives to avoid overwhelming slow disks

## v1.1.0 — 2026-06-12

- **Android backup via FTP**: rclone copy over FTP instead of adb pull — incremental, skips unchanged files, shows real-time progress
- **Android backup requires FTP**: removed ADB pull fallback — FTP is now the only media transfer method
- **Config-based FTP**: `ANDROID_FTP_HOST`/`PORT`/`USER`/`PASS` in config file (required for Android backup)
- **CX File Explorer integration**: auto-starts FTP server via ADB intent, or waits for manual start
- **Android restore improvements**: lists available backups and auto-selects when no path given
- **Removed `--wifi` flag**: device must already be connected via ADB
- **Removed `ANDROID_SKIP_DIRS`**: no longer needed with pure FTP backup

## v1.0.0 — 2026-06-10

- **Pacman-style animated progress bar**: install and uninstall scripts now feature a themed box-drawing banner, braille spinner, diamond status indicators, and section-based output matching backup-games
- **Optimized refactor**: removed `count_files`, DRY package list and gdu logic, cleaned up vars
- **Progress display**: rclone `--progress` with `--stats=1s` for live transfer updates
- **Change tracking**: manifest-based skip for unchanged browser profiles and extra dirs
- **Restore improvements**: fzf multi-select, subdirectory restore, SSH/GPG/keyring support
- **VM backup**: libvirt configs and disk images, sudo rclone for system paths
- **Auto-deps**: installs rclone, gdu, fzf via pacman/apt/dnf/zypper/apk
- **Drive detection**: auto-tunes checkers/transfers based on HDD/SSD/NVMe
