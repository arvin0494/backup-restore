// ─────────────────────────────────────────────────────────────
// MAIN ENTRY POINT — this is where the program starts
// ─────────────────────────────────────────────────────────────
// When you run "bckup" in the terminal, this file decides what
// to do: run a backup, run a restore, or show a menu.
// ─────────────────────────────────────────────────────────────

use std::sync::OnceLock;
use std::io::Write;

// These "mod" lines load the other code files (modules) in the
// src/ folder. Each one handles a different job.
mod backup;   // 📦  backup  — saves your files to a safe place
mod config;   // ⚙️  config  — reads your settings from disk
mod restore;  // 🔄  restore — brings your files back
mod util;     // 🛠️  util    — helper tools used everywhere

// A place to store your home folder path (~/) so every part of
// the program can use it.
pub static HOME: OnceLock<String> = OnceLock::new();

// ── MAIN ───────────────────────────────────────────────────
// This runs automatically when you type "bckup" in the terminal.
fn main() -> anyhow::Result<()> {
    // Remember the home folder (e.g., /home/yourname)
    HOME.get_or_init(|| std::env::var("HOME").unwrap_or_else(|_| "/root".into()));

    use clap::Parser;

    // Define the command-line flags the user can pass:
    //   bckup -b       → backup
    //   bckup -r       → restore
    //   bckup -y       → auto-confirm everything
    #[derive(Parser)]
    #[command(name = "backup", version, about = "Backup & restore for Linux reinstall")]
    struct Cli {
        #[arg(short = 'b', long = "backup")]
        backup: Option<Option<String>>,   // "-b" flag for backup

        #[arg(short = 'r', long = "restore")]
        restore: Option<Option<String>>,  // "-r" flag for restore

        #[arg(value_hint = clap::ValueHint::DirPath)]
        dest: Option<String>,             // an optional folder path

        #[arg(short = 'y', long = "yes")]
        yes: bool,                        // "-y" to skip questions
    }

    // Read what the user typed on the command line
    let cli = Cli::parse();

    // Make sure rclone, gdu, and fzf are installed
    util::install_deps();

    // If no flags were given, run the interactive menu (default = backup)
    if cli.backup.is_none() && cli.restore.is_none() {
        let dest = cli.dest.unwrap_or_else(util::detect_path);
        backup::do_backup(&dest, cli.yes)?;
        return Ok(());
    }

    // Restore mode — bring files back from a backup
    if let Some(val) = cli.restore {
        let backup_dir = match val {
            Some(v) => v,
            None => cli.dest.clone().unwrap_or_else(|| {
                print!("  Backup directory: ");
                std::io::stdout().flush().ok();
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf).ok();
                buf.trim().to_string()
            }),
        };
        let dest_dir = cli.dest.unwrap_or_else(|| HOME.get().unwrap().clone());
        restore::do_restore(&backup_dir, &dest_dir, cli.yes)?;
        return Ok(());
    }

    // Backup mode — save files to a safe location
    if let Some(val) = cli.backup {
        let dest = match val {
            Some(v) => v,
            None => cli.dest.unwrap_or_else(util::detect_path),
        };
        backup::do_backup(&dest, cli.yes)?;
    }

    Ok(())
}
