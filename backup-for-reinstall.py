#!/usr/bin/env python3
"""Backup & restore tool for Linux reinstall.

Saves package lists, configs, browser data, VM data, and home directory.
Restore with fzf multi-select or numbered menu, with rclone progress display.
"""

import os, sys, subprocess, shutil, argparse, readline, time
from pathlib import Path

# ── tqdm (progress bar library) with fallback if not installed ──────────────
try:
    from tqdm import tqdm
except ImportError:
    sys.stderr.write("  \033[0;33mInstalling missing dependencies...\033[0m\n")
    # Minimal no-op fallback so the script still runs without tqdm
    class tqdm:
        def __init__(self, iterable=None, desc=None, unit=None, ncols=None, bar_format=None, disable=False):
            self.iterable = iterable or []
            self.disable = disable
        def __iter__(self):
            return iter(self.iterable)
        def __enter__(self):
            return self
        def __exit__(self, *args):
            pass
        def update(self, n=1):
            pass
        def close(self):
            pass
        def set_description(self, desc):
            pass
        @staticmethod
        def write(s):
            print(s)

# ── ANSI colour constants ──────────────────────────────────────────────────
HOME = os.path.expanduser("~")
R = "\033[0;31m"; G = "\033[0;32m"; Y = "\033[0;33m"
M = "\033[0;35m"; C = "\033[0;36m"; W = "\033[1;37m"; N = "\033[0m"

# Global log file path — set at start of each backup
LOG_FILE = None


# ═════════════════════════════════════════════════════════════════════════════
#  UTILITY HELPERS
# ═════════════════════════════════════════════════════════════════════════════

def e(text, *args, **kwargs):
    """Print a colour-formatted message and also append it to the log file."""
    s = text.format(*args, **kwargs)
    print(s)
    if LOG_FILE:
        with open(LOG_FILE, "a") as f:
            f.write(s + "\n")


import re, signal

def run(cmd, **kwargs):
    """Run a shell command via subprocess. shell=True is set by default."""
    kwargs.setdefault("shell", True)
    return subprocess.run(cmd, **kwargs)


def run_ok(cmd):
    """Return True if the shell command exits with code 0."""
    return run(cmd, capture_output=True).returncode == 0


def copy_progress(cmd, checkers=8, desc="  Syncing", ntfs=False):
    """Run rclone (or any command), passing its --progress output through.
    Set *ntfs* = True to add ``--ignore-errors`` (skips modtime/permission
    failures common on NTFS drives).
    """
    import select, os
    extra = " --ignore-errors" if ntfs else ""
    proc = subprocess.Popen(
        f"stdbuf -oL {cmd} --progress --stats 1s --checkers {checkers}{extra}",
        shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
        start_new_session=True
    )
    fd = proc.stderr.fileno(); buf = b""; warn = 0
    try:
        while True:
            r, _, _ = select.select([fd], [], [], 0.2)
            if r:
                chunk = os.read(fd, 65536)
                if not chunk: break
                buf += chunk
            elif proc.poll() is not None:
                break
            while True:
                cr = buf.find(b'\r')
                nl = buf.find(b'\n')
                if cr < 0 and nl < 0: break
                if nl >= 0 and (cr < 0 or nl < cr):
                    idx = nl + 1; raw = buf[:idx]; buf = buf[idx:]
                else:
                    idx = cr + 1; raw = buf[:idx]; buf = buf[idx:]
                s = raw.strip(b'\r\n ')
                if not s: continue
                if s[:2].isdigit() and b':' in s[:20]:
                    # Rclone log line (timestamp prefix) — skip silently
                    pass
                elif s.startswith(b'WARNING') or s.startswith(b'ERROR') or s.startswith(b'Failed'):
                    warn += 1
                    if warn <= 10:
                        sys.stderr.buffer.write(raw)
                else:
                    sys.stderr.buffer.write(raw)
                sys.stderr.flush()
    except KeyboardInterrupt:
        e("  {}Interrupted, shutting down...{}", Y, N)
        proc.send_signal(signal.SIGINT)
    proc.wait()
    if warn > 10:
        e("  {}... ({} more warnings suppressed){}", Y, warn - 10, N)
    return proc.returncode


def _fmt(size):
    """Format a byte count as human-readable (e.g. ``4.2 GiB``)."""
    if shutil.which("numfmt"):
        sz = run(f"numfmt --to=iec {size}", capture_output=True, shell=True, text=True).stdout.strip()
        if sz: return sz
    return f"{size // 1024 // 1024} MiB"


def detect_path():
    """Auto-detect the backup destination path.

    Pattern: ``/mnt/HDD4T/BACKUP/{hostname}[-{os_id}]``

    *os_id* is read from ``/etc/os-release`` and is omitted when it is a substring of
    the hostname (avoids duplication like ``cachyos-cachyos-…``).
    """
    host = subprocess.run(["hostname", "-s"], capture_output=True, text=True).stdout.strip() or "unknown"
    os_id = ""
    if os.path.isfile("/etc/os-release"):
        for line in open("/etc/os-release"):
            if line.startswith("ID="):
                os_id = line.split("=", 1)[1].strip().strip('"')
                break
    if os_id and os_id in host.lower():
        os_id = ""
    tag = f"-{os_id}" if os_id else ""
    return f"/mnt/HDD4T/BACKUP/{host}{tag}"


def detect_checkers(path):
    """Return optimal rclone ``--checkers`` count for the drive holding *path*.
    HDD → 1 (sequential, minimal seeking), SSD → 8, NVMe → 16.
    """
    try:
        out = subprocess.run(
            ["findmnt", "-n", "-o", "SOURCE", "--target", path],
            capture_output=True, text=True).stdout.strip()
        if not out:
            return 8
        dev = out.rsplit("/", 1)[-1]
        dev = re.sub(r"\d+$", "", dev)
        dev = re.sub(r"p\d+$", "", dev)
        rot = f"/sys/block/{dev}/queue/rotational"
        if os.path.isfile(rot):
            with open(rot) as f:
                if f.read().strip() == "1":
                    return 1
        return 16 if dev.startswith("nvme") else 8
    except Exception:
        return 8


# ═════════════════════════════════════════════════════════════════════════════
#  BACKUP
# ═════════════════════════════════════════════════════════════════════════════

def do_backup(dest, auto_yes=False):
    """Perform a full system backup to *dest*.

    Steps
    -----
    1. Write package lists (pacman, yay, flatpak, snap).
    2. Copy ``~/.config`` (with cache/trash excludes) plus ``.ssh``, ``.gnupg``, keyrings.
    3. Copy browser profiles (Firefox, Chromium, Chrome, Brave — cache excluded).
    4. Copy libvirt VM configs and disk images (sudo).
    5. Copy the full home directory via ``sudo rclone`` with a live progress bar.

    If *auto_yes* is ``True`` the existing-backup warning prompt is skipped.
    """
    dest = os.path.abspath(os.path.expanduser(dest))
    base_mount = os.path.dirname(os.path.dirname(dest))
    if subprocess.run(["findmnt", "-n", base_mount], capture_output=True).returncode != 0:
        print(f"  {R}Error: backup drive not mounted at {base_mount}{N}")
        print(f"  {Y}Mount the drive and try again.{N}")
        sys.exit(1)
    os.makedirs(dest, exist_ok=True)

    global LOG_FILE
    LOG_FILE = os.path.join(dest, "backup.log")
    complete_marker = os.path.join(dest, ".complete")
    e("{}Log:{} {}{}{}", C, N, Y, LOG_FILE, N)

    e("{}Backing up to:{} {}{}{}", C, N, W, dest, N)
    if os.path.isfile(complete_marker):
        e("  {}Warning: backup already exists at this location{}", Y, N)
        if not auto_yes:
            try:
                ok = input("  Overwrite existing backup? [y/N] ").strip().lower()
                if ok != "y":
                    e("  {}Cancelled.{}", Y, N)
                    return
            except (EOFError, KeyboardInterrupt):
                print(); return
    print()

    ck = detect_checkers(dest)
    e("  {}Checkers:{} {}{}{}", C, N, W, ck, N)

    # ── 1. Package lists ─────────────────────────────────────────────────
    e("{}--- Saving package lists ---{}", M, N)
    run("pacman -Qqen > '{}/pacman-official.txt'".format(dest), stderr=subprocess.DEVNULL)
    run("pacman -Qqem > '{}/pacman-aur.txt'".format(dest), stderr=subprocess.DEVNULL)
    run("flatpak list --app --columns=application > '{}/flatpak-list.txt' 2>/dev/null".format(dest))
    run("snap list > '{}/snap-list.txt' 2>/dev/null".format(dest))

    # ── 2. Configs ───────────────────────────────────────────────────────
    e("{}--- Backing up configs ---{}", M, N)
    e("  {}Source:{} ~/.config, ~/.ssh, ~/.gnupg", C, N)
    e("  {}Target:{} {}/config", C, N, dest)
    cfg_dest = os.path.join(dest, "config")
    os.makedirs(cfg_dest, exist_ok=True)
    excludes = " ".join(f"--exclude '{x}'" for x in
        ["Cache","cache","Caches","Trash","trash","Session","sessions",
         "tmp","temp","thumbnails","thumbcache","logs","Logs",
         "Crash Reports","crashpad","*.bak","*~"])
    e("  {}Syncing configs...{}", Y, N)
    copy_progress(f"rclone copy ~/.config/ '{cfg_dest}/' --checkers {ck} {excludes}", desc="  Config", ntfs=True)
    for item in [".ssh", ".gnupg", ".local/share/keyrings"]:
        src = os.path.join(HOME, item)
        if os.path.isdir(src):
            run(f"cp -a '{src}' '{dest}/' 2>/dev/null")

    # ── 3. Browser data ──────────────────────────────────────────────────
    e("{}--- Backing up browser data ---{}", M, N)
    e("  {}Target:{} {}/browser", C, N, dest)
    b_dest = os.path.join(dest, "browser")
    os.makedirs(b_dest, exist_ok=True)
    browsers = [
        (".mozilla", "mozilla"),
        (".config/chromium", "chromium"),
        (".config/google-chrome", "google-chrome"),
        (".config/BraveSoftware", "BraveSoftware"),
    ]
    bx = " ".join(f"--exclude '{x}'" for x in
        ["Cache","cache","Caches","GPUCache","Code Cache",
         "Crash Reports","crashpad","Dictionaries","Safe Browsing"])
    for src_rel, name in browsers:
        src = os.path.join(HOME, src_rel)
        if os.path.isdir(src):
            e("  {}Backing up {}...{}", Y, name, N)
            copy_progress(f"rclone copy '{src}/' '{b_dest}/{name}/' --checkers {ck} {bx}", desc=f"  {name}", ntfs=True)

    # ── 4. VM data (virt-manager / libvirt) ──────────────────────────────
    e("{}--- Backing up VM data ---{}", M, N)
    vm_dest = os.path.join(dest, "virt-manager")
    os.makedirs(vm_dest, exist_ok=True)
    if os.path.isdir("/etc/libvirt/qemu"):
        e("  {}Backing up libvirt VM configs...{}", Y, N)
        run("sudo cp -a /etc/libvirt/qemu '{}/' 2>/dev/null".format(vm_dest))
    if os.path.isdir("/var/lib/libvirt/images"):
        imgsz = run("sudo du -sh /var/lib/libvirt/images | cut -f1", capture_output=True, shell=True, text=True).stdout.strip()
        e("  {}VM disk images:{} {}{}{}", C, N, W, imgsz, N)
        e("  {}Syncing...{}", Y, N)
        try:
            copy_progress(f"sudo rclone copy /var/lib/libvirt/images/ '{vm_dest}/images/' --inplace", checkers=ck, desc="  VM images", ntfs=True)
        except KeyboardInterrupt:
            e("  {}Backup cancelled.{}", R, N)
            return

    # ── 5. Home data ─────────────────────────────────────────────────────
    print()
    e("{}--- Backing up home data ---{}", M, N)

    e("  {}Source:{} ~/ (full home, excluded: .cache, node_modules, etc.)", C, N)
    e("  {}Target:{} {}/home", C, N, dest)

    home_dest = os.path.join(dest, "home")
    os.makedirs(home_dest, exist_ok=True)
    print()
    e("  {}Backing up home data (rclone)...{}", Y, N)

    excludes = [".cache/",".local/share/Trash/",".thumbnails/",
                "*__pycache__/","*.pyc","node_modules/","target/",".next/",
                "snap/",".local/share/flatpak/",".npm/",".cargo/",".rustup/",
                ".gradle/",".m2/","VirtualBox VMs/",".vagrant.d/",
                "Cache/","Code Cache/","GPUCache/","Caches/",
                "*~","*.bak","*.swp"]
    hx = " ".join(f"--exclude '{x}'" for x in excludes)

    # gdu size estimate (fast, parallel scanner)
    total = 0
    if shutil.which("gdu"):
        e("  {}Estimating size...{}", Y, N)
        gdu_ignore = ",".join(
            [".cache","node_modules","target",".next","snap",
             ".npm",".cargo",".rustup",".gradle",".m2",
             "VirtualBox VMs",".vagrant.d",".thumbnails",
             "flatpak","Trash","Cache","Code Cache","GPUCache","Caches"])
        dirs = ["Documents","Pictures","Music","Videos","Downloads","Desktop",
                "Projects","Templates","Public","Games",
                ".local",".fonts",".themes",".icons"]
        for d in dirs:
            p = os.path.join(HOME, d)
            if os.path.isdir(p):
                e("  {}  {}...{}", Y, d, N)
                try:
                    sz = run(f"gdu -n -s -p --no-prefix --ignore-dirs '{gdu_ignore}' '{p}' 2>/dev/null | awk '{{print $1}}'",
                             capture_output=True, shell=True, text=True, timeout=30).stdout.strip()
                except subprocess.TimeoutExpired:
                    sz = ""
                total += int(sz) if sz and sz.isdigit() else 0
        e("  {}Estimated data size:{} {}{}{}", C, N, W, _fmt(total), N)

    try:
        copy_progress(f"sudo rclone copy ~/ '{home_dest}' --links --inplace {hx}", checkers=ck, desc="  Home", ntfs=True)
    except KeyboardInterrupt:
        e("  {}Backup cancelled.{}", R, N)
        return

    # ── Summary ──────────────────────────────────────────────────────────
    print()
    sz_out = run(f"du -sh '{dest}' | cut -f1", capture_output=True, shell=True, text=True).stdout.strip()
    e("  {}=============================={}", G, N)
    e("  {}{}Backup complete!{}", W, W, N)
    e("  {}Size:{} {}{}{}", C, N, W, sz_out, N)
    e("  {}Location:{} {}{}{}", C, N, W, dest, N)
    e("  {}=============================={}", G, N)
    Path(complete_marker).touch()
    print()
    e("  {}To restore:{} python3 {} --restore {}", Y, N, sys.argv[0], dest)


# ═════════════════════════════════════════════════════════════════════════════
#  RESTORE
# ═════════════════════════════════════════════════════════════════════════════

def do_restore(backup_dir, dest_dir, auto=False):
    """Interactively restore a backup.

    Scans *backup_dir* for backup artifacts and builds a list of *items*
    (official packages, AUR packages, Flatpaks, config, browsers, SSH keys,
    GPG keys, keyrings, VM data, home subdirectories).  The user selects
    which items to restore via **fzf** (checkbox-style, multi-select) or a
    fallback numbered menu.  Runs each selected item's callback in sequence.

    If *auto* is ``True`` (``--yes`` flag) every available item is restored
    without prompting.
    """
    backup_dir = os.path.abspath(os.path.expanduser(backup_dir))
    dest_dir = os.path.abspath(os.path.expanduser(dest_dir))
    os.makedirs(dest_dir, exist_ok=True)

    e("{}Backup:{} {}{}{}", C, N, W, backup_dir, N)
    e("{}Restore to:{} {}{}{}", C, N, W, dest_dir, N)
    ck = detect_checkers(dest_dir)
    e("  {}Checkers:{} {}{}{}", C, N, W, ck, N)
    print()

    if not os.path.isdir(backup_dir):
        e("{}Error: backup directory not found{}", R, N)
        sys.exit(1)

    # ── Build the list of restore-able items ──
    items = {}
    def add(key, desc, cb):
        items[key] = (desc, cb)

    # Package lists
    if os.path.isfile(os.path.join(backup_dir, "pacman-official.txt")):
        add("official-pkgs", "Install official packages (pacman)",
            lambda: run("sudo pacman -S --needed - < '{}'".format(os.path.join(backup_dir, "pacman-official.txt")),
                        stderr=subprocess.DEVNULL))

    if os.path.isfile(os.path.join(backup_dir, "pacman-aur.txt")) and shutil.which("yay"):
        add("aur-pkgs", "Install AUR packages (yay)",
            lambda: run("yay -S --needed - < '{}'".format(os.path.join(backup_dir, "pacman-aur.txt")),
                        stderr=subprocess.DEVNULL))

    if os.path.isfile(os.path.join(backup_dir, "flatpak-list.txt")) and shutil.which("flatpak"):
        add("flatpaks", "Install Flatpaks",
            lambda: run("xargs flatpak install -y < '{}'".format(os.path.join(backup_dir, "flatpak-list.txt")),
                        stderr=subprocess.DEVNULL))

    # Config (restore ~/.config)
    if os.path.isdir(os.path.join(backup_dir, "config")):
        add("config", "Restore ~/.config",
            lambda: run("rclone copy '{}/config/' '{}/.config/' --checkers {}".format(backup_dir, dest_dir, ck),
                        stderr=subprocess.DEVNULL))

    # Browser profiles
    browsers = [("mozilla", ".mozilla"), ("chromium", ".config/chromium"),
                 ("google-chrome", ".config/google-chrome"), ("BraveSoftware", ".config/BraveSoftware")]
    for name, rel_dest in browsers:
        p = os.path.join(backup_dir, "browser", name)
        if os.path.isdir(p):
            add(f"browser-{name}", f"Restore {name}",
                lambda p=p, rd=rel_dest: run(f"rclone copy '{p}/' '{dest_dir}/{rd}/' --checkers {ck} 2>/dev/null", stderr=subprocess.DEVNULL))

    # SSH keys & GPG keys
    for name in (".ssh", ".gnupg"):
        p = os.path.join(backup_dir, name)
        if os.path.isdir(p):
            add(name.lstrip("."), f"Restore ~/{name}",
                lambda p=p: run(f"rclone copy '{p}/' '{dest_dir}/{name}/' --checkers {ck} 2>/dev/null"))

    # Login keyrings
    keyrings = os.path.join(backup_dir, "keyrings")
    if os.path.isdir(keyrings):
        add("keyrings", "Restore keyrings (~/.local/share/keyrings)",
            lambda: run("rclone copy '{}/' '{}/.local/share/keyrings/' --checkers {} 2>/dev/null".format(keyrings, dest_dir, ck)))

    # VM configs & disk images (system paths, need sudo)
    vm_qemu = os.path.join(backup_dir, "virt-manager", "qemu")
    if os.path.isdir(vm_qemu):
        add("vm-configs", "Restore libvirt VM configs (/etc/libvirt/qemu)",
            lambda: run("sudo rclone copy '{}/qemu/' /etc/libvirt/qemu/ --checkers {} 2>/dev/null".format(os.path.join(backup_dir, "virt-manager"), ck)))
    vm_images = os.path.join(backup_dir, "virt-manager", "images")
    if os.path.isdir(vm_images):
        add("vm-images", "Restore VM disk images (/var/lib/libvirt/images)",
            lambda: run("sudo rclone copy '{}/' /var/lib/libvirt/images/ --checkers {} 2>/dev/null".format(vm_images, ck)))

    # Per-subdirectory home data
    home_src = os.path.join(backup_dir, "home")
    if os.path.isdir(home_src):
        for sub in sorted(os.listdir(home_src)):
            sp = os.path.join(home_src, sub)
            if os.path.isdir(sp):
                add(f"home-{sub}", f"Restore ~/{sub}",
                    lambda sub=sub: run("rclone copy '{}/home/{}/' '{}/{}/' --checkers {} 2>/dev/null".format(backup_dir, sub, dest_dir, sub, ck)))

    if not items:
        e("{}Nothing found to restore in that directory{}", R, N)
        sys.exit(1)

    keys = list(items.keys())
    labels = [items[k][0] for k in keys]

    # ── Selection ──
    if auto:
        chosen = keys
    elif shutil.which("fzf"):
        # fzf multi-select (Tab to toggle, Enter to confirm)
        print()
        e("  {}Select items to restore (Tab to toggle, Enter to confirm):{}", Y, N)
        inp = "\n".join(f"{k}|{items[k][0]}" for k in keys)
        result = run(f"fzf --multi --prompt='Restore > ' --with-nth=2 -d'|' --height=60% --border",
                     input=inp, capture_output=True, text=True, shell=True)
        chosen = []
        for line in result.stdout.strip().split("\n"):
            if line:
                chosen.append(line.split("|")[0])
    else:
        # Fallback numbered menu
        print()
        e("  {}Select items to restore:{}", Y, N)
        for i, label in enumerate(labels, 1):
            e("  {}{}){} {}", C, i, N, label)
        print()
        inp = input("  Choose (space-separated numbers, or 'all'): ").strip()
        if inp.lower() == "all":
            chosen = keys
        else:
            chosen = []
            for s in inp.split():
                try:
                    idx = int(s) - 1
                    if 0 <= idx < len(keys):
                        chosen.append(keys[idx])
                except ValueError:
                    pass

    if not chosen:
        e("  {}Nothing selected.{}", Y, N)
        return

    # Confirmation prompt
    print()
    e("  {}Restoring:{} {}{}{}", W, N, Y, ", ".join(chosen), N)
    if not auto:
        try:
            ok = input("  Proceed? [Y/n] ").strip().lower()
            if ok in ("n", "no"):
                e("  {}Cancelled.{}", Y, N)
                return
        except (EOFError, KeyboardInterrupt):
            print(); return
    print()

    # Execute each selected restore callback
    for key in tqdm(chosen, desc="  Progress", unit="item", bar_format="{desc} {bar} {n_fmt}/{total_fmt} {unit}s"):
        if key in items:
            desc, fn = items[key]
            tqdm.write(f"{M}--- {desc} ---{N}")
            fn()

    e("  {}=============================={}", G, N)
    e("  {}{}Restore complete!{}", W, W, N)
    e("  {}=============================={}", G, N)


# ═════════════════════════════════════════════════════════════════════════════
#  DEPENDENCIES
# ═════════════════════════════════════════════════════════════════════════════

def install_deps():
    """Auto-detect the system package manager and install required packages.

    Supports pacman (Arch), apt (Debian/Ubuntu), dnf (Fedora), zypper (openSUSE),
    and apk (Alpine).  Installs ``rclone``, ``gdu``, ``fzf``, and the Python
    ``tqdm`` package.
    """
    pm = None; pkgs = {}
    if shutil.which("pacman"):
        pm = "sudo pacman -S --noconfirm"
        pkgs = {"rclone":"rclone","gdu":"gdu","fzf":"fzf","tqdm":"python-tqdm"}
    elif shutil.which("apt-get"):
        pm = "sudo apt-get install -y"
        pkgs = {"rclone":"rclone","gdu":"gdu","fzf":"fzf","tqdm":"python3-tqdm"}
    elif shutil.which("dnf"):
        pm = "sudo dnf install -y"
        pkgs = {"rclone":"rclone","gdu":"gdu","fzf":"fzf","tqdm":"python3-tqdm"}
    elif shutil.which("zypper"):
        pm = "sudo zypper install -y"
        pkgs = {"rclone":"rclone","gdu":"gdu","fzf":"fzf","tqdm":"python3-tqdm"}
    elif shutil.which("apk"):
        pm = "sudo apk add"
        pkgs = {"rclone":"rclone","gdu":"gdu","fzf":"fzf","tqdm":"py3-tqdm"}
    else:
        e("{}No known package manager found. Try: pip install tqdm{}", R, N)

    need = []
    for name, pkg in pkgs.items():
        if name == "tqdm":
            try:
                __import__("tqdm"); continue
            except ImportError:
                pass
        elif shutil.which(name):
            continue
        need.append(pkg if pm else name)

    if not need:
        return True
    if not pm:
        e("  {}Install manually: pip install --user {}rclone gdu fzf{}", Y, "" if sys.platform == "linux" else "", N)
        return False

    e("  {}Installing:{} {}{}{}", Y, N, W, " ".join(need), N)
    for pkg in need:
        run(f"{pm} {pkg}")

    # Verify installation
    try:
        __import__("tqdm")
        e("  {}Dependencies installed.{}", G, N)
    except ImportError:
        e("  {}tqdm still missing. Try: pip install --user tqdm{}", R, N)
    return True


# ═════════════════════════════════════════════════════════════════════════════
#  CLI ENTRY POINT
# ═════════════════════════════════════════════════════════════════════════════

def main():
    parser = argparse.ArgumentParser(description="Backup & restore for Linux reinstall",
                                     formatter_class=argparse.RawTextHelpFormatter)
    parser.add_argument("--backup", "-b", nargs="?", const=None, metavar="DIR",
                        help="Backup to DIR (default: auto-detect)")
    parser.add_argument("--restore", "-r", nargs="?", const=None, metavar="DIR",
                        help="Restore from backup DIR")
    parser.add_argument("dest", nargs="?", help="Backup target or restore destination")
    parser.add_argument("--yes", "-y", action="store_true", help="Skip prompts, select all")

    args = parser.parse_args()

    install_deps()

    # Default to backup when no arguments given
    if not args.backup and not args.restore:
        do_backup(args.dest or detect_path(), auto_yes=args.yes)
        return

    if args.restore:
        if args.restore is argparse.SUPPRESS or args.restore is None:
            # --restore without a value: try positional arg, or prompt
            if args.dest and os.path.isdir(args.dest):
                do_restore(args.dest, HOME, auto=args.yes)
            else:
                try:
                    inp = input("  Backup directory: ").strip()
                    if inp:
                        do_restore(inp, args.dest or HOME, auto=args.yes)
                except (EOFError, KeyboardInterrupt):
                    print()
        else:
            do_restore(args.restore, args.dest or HOME, auto=args.yes)
        return

    if args.backup:
        dest = args.backup if args.backup is not None else (args.dest or detect_path())
        do_backup(dest, auto_yes=args.yes)
        return


if __name__ == "__main__":
    main()
