#!/usr/bin/env bash
set -euo pipefail

REPO="arvin0494/backup-restore"
BRANCH="main"
SSH_URL="git@github.com:$REPO"
HTTPS_URL="https://github.com/$REPO.git"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/backup-restore"
BIN="${XDG_BIN_HOME:-$HOME/.local/bin}/backup"

# ── Colours ────────────────────────────────────────────────
R="\033[0;31m"; G="\033[0;32m"; Y="\033[0;33m"; C="\033[0;36m"; W="\033[1;37m"; N="\033[0m"

info()  { printf "  ${C}%s${N}\n" "$*"; }
ok()    { printf "  ${G}%s${N}\n" "$*"; }
warn()  { printf "  ${Y}%s${N}\n" "$*"; }
err()   { printf "  ${R}%s${N}\n" "$*"; }

status() {
    local label="$1" val="$2"
    printf "  ${W}▸${N} ${label} ${G}${val}${N}\n"
}

progress() {
    local pct="$1" msg="$2"
    local filled=$((pct / 5))
    local empty=$((20 - filled))
    printf "  ${C}[${N}"
    for ((i=0; i<filled; i++)); do printf "${G}█${N}"; done
    for ((i=0; i<empty; i++)); do printf "${C}░${N}"; done
    printf "${C}]${N}  ${W}%3d%%${N} %s\n" "$pct" "$msg"
}

# ── Header ─────────────────────────────────────────────────
echo ""
printf "  ${C}██████╗  █████╗  ██████╗██╗  ██╗██╗   ██╗██████╗${N}\n"
printf "  ${C}██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██║   ██║██╔══██╗${N}\n"
printf "  ${C}██████╔╝███████║██║     █████╔╝ ██║   ██║██████╔╝${N}\n"
printf "  ${C}██╔══██╗██╔══██║██║     ██╔═██╗ ██║   ██║██╔═══╝${N}\n"
printf "  ${C}██████╔╝██║  ██║╚██████╗██║  ██╗╚██████╔╝██║${N}\n"
printf "  ${C}╚═════╝ ╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝${N}\n"
echo ""

# ── Pre-checks ─────────────────────────────────────────────
printf "  ${W}▸${N} ROOT ACCESS ........................ ${G}CONFIRMED${N}\n"
printf "  ${W}▸${N} USER .............................. ${C}${USER}${N}\n"
printf "  ${W}▸${N} TARGET ............................ ${C}${BIN}${N}\n"
printf "  ${W}▸${N} SOURCE ............................ ${C}${HTTPS_URL}${N}\n"
echo ""

# ── Ensure Rust is installed ───────────────────────────────
ensure_rust() {
    [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"

    if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
        progress 100 "Rust check"
        status "RUST ............................... " "$(rustc --version)"
        return 0
    fi

    if [[ -x "$HOME/.cargo/bin/rustc" && -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        progress 100 "Rust check"
        status "RUST ............................... " "$("$HOME/.cargo/bin/rustc" --version)"
        return 0
    fi

    printf "  ${C}── INSTALLING RUST ──${N}\n"
    echo ""
    printf "  ${Y}Rust is not installed. Choose method:${N}\n"
    printf "  ${W}  1${N}) rustup (recommended)\n"
    printf "  ${W}  2${N}) system package manager (pacman / apt / dnf / zypper / apk)\n"
    printf "  ${W}  3${N}) skip — I'll install it myself\n"
    echo ""
    printf "  ${W}  Choose [1]:${N} "
    read -r ans
    case "${ans:-1}" in
        1|"")
            progress 14 "Fetching rustup…"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
                | sh -s -- -y --no-modify-path >/dev/null 2>&1
            progress 50 "Installing rustup…"
            . "$HOME/.cargo/env"
            progress 100 "Rust installed"
            status "RUST ............................... " "$(rustc --version)"
            ;;
        2)
            if command -v pacman &>/dev/null; then
                run="sudo pacman -S --noconfirm rust"
            elif command -v apt-get &>/dev/null; then
                run="sudo apt-get install -y rustc cargo"
            elif command -v dnf &>/dev/null; then
                run="sudo dnf install -y rust cargo"
            elif command -v zypper &>/dev/null; then
                run="sudo zypper install -y rust cargo"
            elif command -v apk &>/dev/null; then
                run="sudo apk add rust cargo"
            else
                err "No known package manager found."
                err "Install Rust manually: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
                exit 1
            fi
            progress 30 "Installing via package manager…"
            $run >/dev/null 2>&1
            if ! command -v rustc &>/dev/null; then
                err "Rust not found after install."
                exit 1
            fi
            progress 100 "Rust installed"
            status "RUST ............................... " "$(rustc --version)"
            ;;
        *)
            err "Rust is required."
            err "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            exit 1
            ;;
    esac
}

# ── Clone / update repo ────────────────────────────────────
clone_repo() {
    printf "  ${C}── DOWNLOADING SOURCE ──${N}\n"
    echo ""
    if [[ -d "$DEST" ]]; then
        progress 40 "Updating existing clone…"
        git -C "$DEST" fetch origin "$BRANCH" 2>/dev/null
        git -C "$DEST" reset --hard "origin/$BRANCH" 2>/dev/null || true
        progress 100 "Repository updated"
        status "CLONE .............................. " "$DEST"
    else
        progress 10 "Cloning repository…"
        if git clone --branch "$BRANCH" --depth 1 "$HTTPS_URL" "$DEST" 2>/dev/null; then
            :
        else
            git clone --branch "$BRANCH" --depth 1 "$SSH_URL" "$DEST"
        fi
        progress 100 "Repository cloned"
        status "CLONE .............................. " "$DEST"
    fi
}

# ── Build release binary ───────────────────────────────────
build_binary() {
    printf "  ${C}── BUILDING BINARY ──${N}\n"
    echo ""
    progress 10 "Compiling (this may take a while)…"
    cargo build --release --manifest-path "$DEST/backup-rs/Cargo.toml" >/dev/null 2>&1
    progress 70 "Installing binary…"
    mkdir -p "$(dirname "$BIN")"
    cp "$DEST/backup-rs/target/release/backup" "$BIN"
    chmod +x "$BIN"
    progress 100 "Build complete"
    status "BINARY .............................. " "$BIN"
}

# ── Add shell alias ────────────────────────────────────────
shell_aliases() {
    printf "  ${C}── INJECTING SHELL ALIAS ──${N}\n"
    echo ""
    local rc
    case "${SHELL##*/}" in
        zsh)  rc="$HOME/.zshrc" ;;
        fish) rc="$HOME/.config/fish/config.fish" ;;
        bash) rc="$HOME/.bashrc" ;;
        *)    rc="$HOME/.profile" ;;
    esac

    local line
    if [[ "${SHELL##*/}" == "fish" ]]; then
        line="alias bckup='$BIN'"
        if ! grep -sqE "^alias bckup[= ']" "$rc" 2>/dev/null; then
            echo "$line" >> "$rc"
            progress 100 "Alias injected"
            status "SHELL RC ............................ " "$rc"
        else
            progress 100 "Alias verified"
            status "SHELL RC ............................ " "$rc (already set)"
        fi
    else
        line="alias bckup='$BIN'"
        if ! grep -sqF "alias bckup=" "$rc" 2>/dev/null; then
            echo "" >> "$rc"
            echo "# backup-restore" >> "$rc"
            echo "$line" >> "$rc"
            progress 100 "Alias injected"
            status "SHELL RC ............................ " "$rc"
        else
            progress 100 "Alias verified"
            status "SHELL RC ............................ " "$rc (already set)"
        fi
    fi
}

# ── Create default config ─────────────────────────────────
create_config() {
    printf "  ${C}── INITIALIZING CONFIG ──${N}\n"
    echo ""
    local cfg_dir="$HOME/.config/backup-restore"
    local cfg_file="$cfg_dir/config"
    if [[ -f "$cfg_file" ]]; then
        progress 100 "Config exists"
        status "CONFIG .............................. " "$cfg_file"
        return
    fi
    mkdir -p "$cfg_dir"
    cat > "$cfg_file" <<EOF
# backup-restore configuration
# Uncomment and edit to override defaults.

BACKUP_BASE=/mnt/HDD4T/BACKUP
# VM_QEMU_SRC=/etc/libvirt/qemu
# VM_IMAGES_SRC=/var/lib/libvirt/images
# BACKUP_EXTRA_DIRS=/path/to/something,/another/path
EOF
    progress 100 "Config created"
    status "CONFIG .............................. " "$cfg_file"
}

# ── Execute ────────────────────────────────────────────────
ensure_rust
clone_repo
build_binary
shell_aliases
create_config

# ── Complete ───────────────────────────────────────────────
echo ""
printf "  ${C}╔══════════════════════════════════════════════════════╗${N}\n"
printf "  ${C}║${N}  ${G}████████████████████████████████████████████${N}\n"
printf "  ${C}║${N}  ${G}█${N}  ${W}DEPLOYMENT COMPLETE${N}                    ${G}█${N}\n"
printf "  ${C}║${N}  ${G}████████████████████████████████████████████${N}\n"
printf "  ${C}╚══════════════════════════════════════════════════════╝${N}\n"
echo ""
printf "  ${W}▸${N} RUN ................................ ${C}bckup -b${N}\n"
printf "  ${W}▸${N} HELP ............................... ${C}bckup --help${N}\n"
printf "  ${W}▸${N} UNINSTALL .......................... ${C}bash $DEST/uninstall.sh${N}\n"

VERSION=$(grep -oP '(?<=^version = ").*(?=")' "$DEST/backup-rs/Cargo.toml" 2>/dev/null || echo "dev")
printf "  ${W}▸${N} VERSION ............................ ${C}v${VERSION}${N}\n"
echo ""
