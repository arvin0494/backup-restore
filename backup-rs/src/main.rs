use std::sync::OnceLock;
use std::io::Write;

mod backup;
mod config;
mod restore;
mod util;

pub static HOME: OnceLock<String> = OnceLock::new();

fn main() -> anyhow::Result<()> {
    HOME.get_or_init(|| std::env::var("HOME").unwrap_or_else(|_| "/root".into()));

    use clap::Parser;

    #[derive(Parser)]
    #[command(name = "backup", version, about = "Backup & restore for Linux reinstall")]
    struct Cli {
        /// Backup to DIR (auto-detect if no value)
        #[arg(short = 'b', long = "backup")]
        backup: Option<Option<String>>,

        /// Restore from backup DIR
        #[arg(short = 'r', long = "restore")]
        restore: Option<Option<String>>,

        /// Backup target or restore destination
        #[arg(value_hint = clap::ValueHint::DirPath)]
        dest: Option<String>,

        /// Skip prompts, select all
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    }

    let cli = Cli::parse();

    // Auto-install deps
    util::install_deps();

    // No flags → default to backup
    if cli.backup.is_none() && cli.restore.is_none() {
        let dest = cli.dest.unwrap_or_else(util::detect_path);
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = backup::do_backup(&dest, cli.yes);
        })) {
            Ok(_) => {}
            Err(_) => {
                eprintln!();
                util::e(&format!("{}Backup cancelled.{}", util::R, util::N));
            }
        }
        return Ok(());
    }

    // Restore
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
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = restore::do_restore(&backup_dir, &dest_dir, cli.yes);
        })) {
            Ok(_) => {}
            Err(_) => {
                eprintln!();
                util::e(&format!("{}Restore cancelled.{}", util::R, util::N));
            }
        }
        return Ok(());
    }

    // Backup
    if let Some(val) = cli.backup {
        let dest = match val {
            Some(v) => v,
            None => cli.dest.unwrap_or_else(util::detect_path),
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = backup::do_backup(&dest, cli.yes);
        })) {
            Ok(_) => {}
            Err(_) => {
                eprintln!();
                util::e(&format!("{}Backup cancelled.{}", util::R, util::N));
            }
        }
    }

    Ok(())
}
