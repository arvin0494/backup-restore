#!/usr/bin/env python3
"""Backup & restore tool for Linux reinstall."""

import os, sys, subprocess, shutil, argparse, readline
from datetime import datetime
from pathlib import Path

try:
    from tqdm import tqdm
except ImportError:
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

HOME = os.path.expanduser("~")
R = "\033[0;31m"; G = "\033[0;32m"; Y = "\033[0;33m"
M = "\033[0;35m"; C = "\033[0;36m"; W = "\033[1;37m"; N = "\033[0m"

LOG_FILE = None

def e(text, *args, **kwargs):
    s = text.format(*args, **kwargs)
    print(s)
    if LOG_FILE:
        with open(LOG_FILE, "a") as f:
            f.write(s + "\n")

import re, signal

def run(cmd, **kwargs):
    kwargs.setdefault("shell", True)
    return subprocess.run(cmd, **kwargs)

def rsync_progress(cmd, desc="  Syncing"):
    proc = subprocess.Popen(
        f"stdbuf -oL {cmd} --info=progress2 --out-format='%n'",
        shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        text=True, bufsize=1, start_new_session=True
    )
    pat = re.compile(r'\(xfr#\d+,\s*to-chk=(\d+)/(\d+)\)')
    total = None; pbar = None

    try:
        for line in iter(proc.stdout.readline, ''):
            line = line.strip()
            if not line:
                continue
            m = pat.search(line)
            if m:
                rem = int(m.group(1)); t = int(m.group(2))
                if total is None:
                    total = t
                    if total:
                        pbar = tqdm(total=total, unit="file", desc=desc,
                                    bar_format="{desc} {bar} {n_fmt}/{total_fmt} [{elapsed}<{remaining}, {rate_fmt}]")
                if pbar and total:
                    pbar.n = total - rem
                    pbar.refresh()
            elif pbar:
                pbar.set_description(f"{desc} [{line[:55]}]")
    except KeyboardInterrupt:
        e("{}Interrupted, shutting down rsync...{}", Y, N)
        proc.send_signal(signal.SIGINT)
    proc.wait()
    if pbar:
        pbar.n = pbar.total if pbar.total else 0
        pbar.refresh(); pbar.close()
    return proc.returncode

def run_ok(cmd):
    return run(cmd, capture_output=True).returncode == 0

def detect_path():
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
    return f"/mnt/HDD4T/{host}{tag}-{datetime.now():%Y%m}"


# ─────────────────────────────────────────────
#  BACKUP
# ─────────────────────────────────────────────
def do_backup(dest):
    dest = os.path.abspath(os.path.expanduser(dest))
    os.makedirs(dest, exist_ok=True)

    global LOG_FILE
    LOG_FILE = os.path.join(dest, "backup.log")
    e("{}Log:{} {}{}{}", C, N, Y, LOG_FILE, N)

    complete_marker = os.path.join(dest, ".complete")
    home_dir = os.path.join(dest, "home")
    if os.path.isdir(home_dir) and not os.path.isfile(complete_marker):
        e("  {}Removing previous incomplete backup...{}", Y, N)
        shutil.rmtree(home_dir, ignore_errors=True)

    e("{}Backing up to:{} {}{}{}", C, N, W, dest, N)
    print()

    # ── Package lists ──
    e("{}--- Saving package lists ---{}", M, N)
    run("pacman -Qqen > '{}/pacman-official.txt'".format(dest), stderr=subprocess.DEVNULL)
    run("pacman -Qqem > '{}/pacman-aur.txt'".format(dest), stderr=subprocess.DEVNULL)
    run("flatpak list --app --columns=application > '{}/flatpak-list.txt' 2>/dev/null".format(dest))
    run("snap list > '{}/snap-list.txt' 2>/dev/null".format(dest))

    # ── Configs ──
    e("{}--- Backing up configs ---{}", M, N)
    e("  {}Source:{} ~/.config, ~/.ssh, ~/.gnupg", C, N)
    e("  {}Target:{} {}/config", C, N, dest)
    cfg_dest = os.path.join(dest, "config")
    os.makedirs(cfg_dest, exist_ok=True)
    excludes = " ".join(f"--exclude='{x}'" for x in
        ["Cache","cache","Caches","Trash","trash","Session","sessions",
         "tmp","temp","thumbnails","thumbcache","logs","Logs",
         "Crash Reports","crashpad","*.bak","*~"])
    e("  {}Syncing configs...{}", Y, N)
    run(f"rsync -a {excludes} ~/.config/ '{cfg_dest}/' 2>/dev/null", stderr=subprocess.DEVNULL)
    for item in [".ssh", ".gnupg", ".local/share/keyrings"]:
        src = os.path.join(HOME, item)
        if os.path.isdir(src):
            run(f"cp -a '{src}' '{dest}/' 2>/dev/null")

    # ── Browser data ──
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
    bx = " ".join(f"--exclude='{x}'" for x in
        ["Cache","cache","Caches","GPUCache","Code Cache",
         "Crash Reports","crashpad","Dictionaries","Safe Browsing"])
    for src_rel, name in tqdm(browsers, desc="  Browsers", unit="browser", bar_format="{desc} {bar} {n_fmt}/{total_fmt} {unit}s"):
        src = os.path.join(HOME, src_rel)
        if os.path.isdir(src):
            run(f"rsync -a {bx} '{src}/' '{b_dest}/{name}/' 2>/dev/null", stderr=subprocess.DEVNULL)

    # ── VM data (virt-manager / libvirt) ──
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
        rsync_progress(f"sudo rsync -aAX --inplace --no-inc-recursive /var/lib/libvirt/images/ '{vm_dest}/images/'", desc="  VM images")

    # ── Home data ──
    print()
    e("{}--- Backing up home data ---{}", M, N)
    dirs = ["Documents","Pictures","Music","Videos","Downloads","Desktop",
            "Projects","Templates","Public","Games",
            ".local",".fonts",".themes",".icons"]

    # gdu size estimate
    total = 0
    if shutil.which("gdu"):
        e("  {}Estimating size...{}", Y, N)
        for d in tqdm(dirs, desc="  Scanning", unit="dir", bar_format="{desc} {bar} {n_fmt}/{total_fmt} {unit}s"):
            p = os.path.join(HOME, d)
            if os.path.isdir(p):
                sz = run(f"gdu -n -s -p --no-prefix '{p}' 2>/dev/null | awk '{{print $1}}'",
                         capture_output=True, shell=True, text=True).stdout.strip()
                total += int(sz) if sz and sz.isdigit() else 0
        e("  {}Estimated data size:{} {}{}{}", C, N, W, _fmt(total), N)

    e("  {}Source:{} ~/ (full home, excluded: .cache, node_modules, etc.)", C, N)
    e("  {}Target:{} {}/home", C, N, dest)

    home_dest = os.path.join(dest, "home")
    os.makedirs(home_dest, exist_ok=True)
    print()
    e("  {}Backing up home data (sudo rsync)...{}", Y, N)
    hx = " ".join(f"--exclude='{x}'" for x in
        [".cache/",".local/share/Trash/",".thumbnails/",
         "*__pycache__/","*.pyc","node_modules/","target/",".next/",
         "snap/",".local/share/flatpak/",".npm/",".cargo/",".rustup/",
         ".gradle/",".m2/","VirtualBox VMs/",".vagrant.d/",
         "*~","*.bak","*.swp"])
    rsync_progress(f"sudo rsync -aAX --inplace --no-links --no-inc-recursive {hx} ~/ '{home_dest}'", desc="  Home")

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


def _fmt(size):
    if shutil.which("numfmt"):
        sz = run(f"numfmt --to=iec {size}", capture_output=True, shell=True, text=True).stdout.strip()
        if sz: return sz
    return f"{size // 1024 // 1024} MiB"


# ─────────────────────────────────────────────
#  RESTORE
# ─────────────────────────────────────────────
def do_restore(backup_dir, dest_dir, auto=False):
    backup_dir = os.path.abspath(os.path.expanduser(backup_dir))
    dest_dir = os.path.abspath(os.path.expanduser(dest_dir))
    os.makedirs(dest_dir, exist_ok=True)

    e("{}Backup:{} {}{}{}", C, N, W, backup_dir, N)
    e("{}Restore to:{} {}{}{}", C, N, W, dest_dir, N)
    print()

    if not os.path.isdir(backup_dir):
        e("{}Error: backup directory not found{}", R, N)
        sys.exit(1)

    # ── Build available items ──
    items = {}
    def add(key, desc, cb):
        items[key] = (desc, cb)

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

    if os.path.isdir(os.path.join(backup_dir, "config")):
        add("config", "Restore ~/.config",
            lambda: run("rsync -a '{}/config/' '{}/.config/'".format(backup_dir, dest_dir),
                        stderr=subprocess.DEVNULL))

    browsers = [("mozilla", ".mozilla"), ("chromium", ".config/chromium"),
                 ("google-chrome", ".config/google-chrome"), ("BraveSoftware", ".config/BraveSoftware")]
    for name, rel_dest in browsers:
        p = os.path.join(backup_dir, "browser", name)
        if os.path.isdir(p):
            add(f"browser-{name}", f"Restore {name}",
                lambda p=p, rd=rel_dest: run(f"rsync -a '{p}/' '{dest_dir}/{rd}/' 2>/dev/null", stderr=subprocess.DEVNULL))

    for name in (".ssh", ".gnupg"):
        p = os.path.join(backup_dir, name)
        if os.path.isdir(p):
            add(name.lstrip("."), f"Restore ~/{name}",
                lambda p=p: run(f"cp -a '{p}' '{dest_dir}/' 2>/dev/null"))

    keyrings = os.path.join(backup_dir, "keyrings")
    if os.path.isdir(keyrings):
        add("keyrings", "Restore keyrings (~/.local/share/keyrings)",
            lambda: run("cp -a '{}' '{}/.local/share/' 2>/dev/null".format(keyrings, dest_dir)))

    vm_qemu = os.path.join(backup_dir, "virt-manager", "qemu")
    if os.path.isdir(vm_qemu):
        add("vm-configs", "Restore libvirt VM configs (/etc/libvirt/qemu)",
            lambda: run("sudo cp -a '{}/qemu' /etc/libvirt/ 2>/dev/null".format(os.path.join(backup_dir, "virt-manager"))))
    vm_images = os.path.join(backup_dir, "virt-manager", "images")
    if os.path.isdir(vm_images):
        add("vm-images", "Restore VM disk images (/var/lib/libvirt/images)",
            lambda: run("sudo rsync -a '{}/' /var/lib/libvirt/images/ 2>/dev/null".format(vm_images)))

    # Home subdirs
    home_src = os.path.join(backup_dir, "home")
    if os.path.isdir(home_src):
        for sub in sorted(os.listdir(home_src)):
            sp = os.path.join(home_src, sub)
            if os.path.isdir(sp):
                add(f"home-{sub}", f"Restore ~/{sub}",
                    lambda sub=sub: run("rsync -a '{}/home/{}/' '{}/{}/' 2>/dev/null".format(backup_dir, sub, dest_dir, sub)))

    if not items:
        e("{}Nothing found to restore in that directory{}", R, N)
        sys.exit(1)

    keys = list(items.keys())
    labels = [items[k][0] for k in keys]

    # ── Selection ──
    if auto:
        chosen = keys
    elif shutil.which("fzf"):
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

    print()
    e("  {}Restoring:{} {}{}{}", W, N, Y, ", ".join(chosen), N)
    print()

    for key in tqdm(chosen, desc="  Progress", unit="item", bar_format="{desc} {bar} {n_fmt}/{total_fmt} {unit}s"):
        if key in items:
            desc, fn = items[key]
            tqdm.write(f"{M}--- {desc} ---{N}")
            fn()

    e("  {}=============================={}", G, N)
    e("  {}{}Restore complete!{}", W, W, N)
    e("  {}=============================={}", G, N)


# ─────────────────────────────────────────────
#  MAIN
# ─────────────────────────────────────────────
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

    # Interactive menu if no flags
    if not args.backup and not args.restore:
        print()
        e("  {}What do you want to do?{}", Y, N)
        e("  {}1){} Backup this system", C, N)
        e("  {}2){} Restore from backup", C, N)
        print()
        try:
            choice = input("  Choose (1/2): ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return

        if choice in ("2", "r", "restore"):
            default_path = ""
            try:
                inp = input("  Backup directory: ").strip()
                if inp:
                    do_restore(inp, args.dest or HOME, auto=args.yes)
                else:
                    e("{}No directory given.{}", R, N)
            except (EOFError, KeyboardInterrupt):
                print()
            return
        else:
            args.backup = args.dest or detect_path()
            do_backup(args.backup)
            return

    if args.restore:
        if args.restore is argparse.SUPPRESS or args.restore is None:
            # --restore without value, check positional
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
        do_backup(dest)
        return


if __name__ == "__main__":
    main()
