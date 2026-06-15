# Project Template: Rust CLI Tool + Installer

## File Layout
```
project/
├── src/
│   ├── main.rs        # Entry point + CLI parsing (clap)
│   ├── config.rs      # Constants + user config loader
│   ├── util.rs        # Logging, subprocess, helper functions
│   ├── backup.rs      # Primary operation: backup
│   └── restore.rs     # Primary operation: restore
├── install.sh         # Single-binary installer
├── README.md
└── .gitignore
```

## Architecture Layers

### 1. Config Layer (`config.rs`)
- Hardcoded defaults as `pub const`
- User override via `~/.config/<project>/config` (key=value, `#` comments, blank lines skipped)
- `load_user_config() -> HashMap`, fallback to constant with `unwrap_or(DEFAULT)`

### 2. Utility Layer (`util.rs`)
- Colored output + log file (same message → terminal AND log via `e()`)
- `run(cmd)` / `run_ok(cmd)` — `sh -c` subprocess
- `copy_progress()` — rclone wrapper: spawn with `--progress`, inherit stderr, Ctrl+C kills process group
- `detect_path()` — build path from hostname + os-release
- `detect_checkers()` — probe rotational/NVMe for parallelism (HDD=1, SSD=8, NVMe=16)
- `install_deps()` — detect package manager → install rclone, gdu, fzf
- ANSI color constants shared across modules

### 3. Operation Modules (`backup.rs`, `restore.rs`)
- One function per step: `_save_package_lists`, `_estimate_home_size`, `_backup_config`, `_backup_browsers`, `_backup_vm`, `_backup_home`
- Steps run sequentially (order matters)
- `std::thread::spawn` for parallel estimation alongside serial steps
- All call `copy_progress()` with different args (checkers, ntfs flag, skip-links)
- Restore: scan backup dir → build item list → fzf multi-select → execute closures

### 4. Entry Point (`main.rs`)
- `clap::Parser` derive: `--backup/-b`, `--restore/-r`, `--yes/-y`, positional path
- `install_deps()` first, then route: no flags → backup
- `std::panic::catch_unwind` around operations for clean cancellation message

### 5. Installer (`install.sh`)
- `ensure_rust()` — source `~/.cargo/env`, check PATH + `~/.cargo/bin/` directly
- `clone_repo()` — `git clone --depth 1` (HTTPS → SSH fallback for private repos)
- `build_binary()` — `cargo build --release`, cp to `~/.local/bin/`
- `shell_aliases()` — detect shell, append alias to rc file
- `create_config()` — write default `~/.config/<project>/config` if absent

## Key Patterns

| Pattern | Implementation |
|---|---|
| Config sharing | Same file parsed identically across versions |
| Progress display | Inherit stderr, `--progress` on the tool |
| Cancellation | Process group SIGINT → 10s → SIGKILL |
| Parallel estimation | `thread::spawn` fire-and-forget during sequential steps |
| Dual push | `git remote set-url --add --push origin` |
| Hardware tuning | Rotational flag → HDD(1)/SSD(8)/NVMe(16) |
| Error handling | `anyhow::Result` throughout |
