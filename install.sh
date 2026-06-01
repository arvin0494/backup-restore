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

# ── Ensure Rust is installed ───────────────────────────────
ensure_rust() {
    # source rustup env first, in case ~/.cargo/bin isn't on PATH yet
    [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
    if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
        ok "Rust $(rustc --version) already installed."
        return 0
    fi
    # also check ~/.cargo/bin directly
    if [[ -x "$HOME/.cargo/bin/rustc" && -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        ok "Rust $("$HOME/.cargo/bin/rustc" --version) already installed (added ~/.cargo/bin to PATH)."
        return 0
    fi
    warn "Rust is not installed."
    echo ""
    printf "  ${W}1${N}) rustup (recommended)\n"
    printf "  ${W}2${N}) system package manager (pacman / apt / dnf / zypper / apk)\n"
    printf "  ${W}3${N}) skip — I'll install it myself\n"
    echo ""
    printf "  Choose [1]: "
    read -r ans
    case "${ans:-1}" in
        1|"")
            info "Downloading rustup…"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
                | sh -s -- -y --no-modify-path
            # shellcheck disable=SC1091
            . "$HOME/.cargo/env"
            ok "Rust installed via rustup."
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
            info "Installing Rust via system package manager…"
            $run
            if ! command -v rustc &>/dev/null; then
                err "Rust not found after install. Try option 1 (rustup)."
                exit 1
            fi
            ok "Rust installed via system package manager."
            ;;
        *)
            err "Rust is required. Run this script again after installing Rust."
            err "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            exit 1
            ;;
    esac
}

# ── Clone / update repo ────────────────────────────────────
clone_repo() {
    if [[ -d "$DEST" ]]; then
        info "Updating existing clone at $DEST …"
        git -C "$DEST" pull --ff-only origin "$BRANCH"
    else
        info "Cloning $REPO into $DEST …"
        git clone --branch "$BRANCH" --depth 1 "$HTTPS_URL" "$DEST" 2>/dev/null \
            || git clone --branch "$BRANCH" --depth 1 "$SSH_URL" "$DEST"
    fi
}

# ── Build release binary ───────────────────────────────────
build_binary() {
    info "Building release binary …"
    cargo build --release --manifest-path "$DEST/backup-rs/Cargo.toml"
    mkdir -p "$(dirname "$BIN")"
    cp "$DEST/backup-rs/target/release/backup" "$BIN"
    chmod +x "$BIN"
    ok "Binary installed at $BIN"
}

# ── Add shell alias ────────────────────────────────────────
shell_aliases() {
    local rc
    case "${SHELL##*/}" in
        zsh)  rc="$HOME/.zshrc" ;;
        fish) rc="$HOME/.config/fish/config.fish" ;;
        bash) rc="$HOME/.bashrc" ;;
        *)    rc="$HOME/.profile" ;;
    esac

    local line
    if [[ "${SHELL##*/}" == "fish" ]]; then
        line="alias backup='$BIN'"
        if ! grep -sqF "alias backup" "$rc" 2>/dev/null; then
            echo "$line" >> "$rc"
            ok "Alias added to $rc  (run: source $rc)"
        else
            ok "Alias already present in $rc"
        fi
    else
        line="alias backup='$BIN'"
        if ! grep -sqF "alias backup=" "$rc" 2>/dev/null; then
            echo "" >> "$rc"
            echo "# backup-restore" >> "$rc"
            echo "$line" >> "$rc"
            ok "Alias added to $rc  (run: source $rc)"
        else
            ok "Alias already present in $rc"
        fi
    fi
}

# ── Create default config ─────────────────────────────────
create_config() {
    local cfg_dir="$HOME/.config/backup-restore"
    local cfg_file="$cfg_dir/config"
    if [[ -f "$cfg_file" ]]; then
        ok "Config already exists at $cfg_file"
        return
    fi
    mkdir -p "$cfg_dir"
    cat > "$cfg_file" <<EOF
# backup-restore configuration
# Uncomment and edit to override defaults.

BACKUP_BASE=/mnt/HDD4T/BACKUP
# VM_QEMU_SRC=/etc/libvirt/qemu
# VM_IMAGES_SRC=/var/lib/libvirt/images
EOF
    ok "Default config created at $cfg_file"
}

# ── Print usage / curl hint ────────────────────────────────
cat <<EOF

  ${W}backup-restore${N}  —  Backup & restore for Linux reinstall

EOF

ensure_rust
clone_repo
build_binary
shell_aliases
create_config

info "Python version also available at: $DEST/backup-for-reinstall.py"
cat <<EOF

  ${W}Usage:${N}  backup -b              (backup, auto-detect path)
          backup -b /path    (backup, custom path)
          backup -r /path    (restore interactively)

EOF
