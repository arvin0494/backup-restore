#!/usr/bin/env bash
set -euo pipefail

REPO="arvin0494/backup-restore"
BRANCH="main"
URL="https://github.com/$REPO"
RAW="https://raw.githubusercontent.com/$REPO/$BRANCH"
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
    if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
        ok "Rust $(rustc --version) already installed."
        return 0
    fi
    warn "Rust is not installed."
    printf "  Install via rustup? [Y/n] "
    read -r ans
    case "${ans:-y}" in
        y|Y|yes|YES|"")
            info "Downloading rustup…"
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
                | sh -s -- -y --no-modify-path
            # shellcheck disable=SC1091
            . "$HOME/.cargo/env"
            ok "Rust installed."
            ;;
        *)
            err "Rust is required. Install manually: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
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
        git clone --branch "$BRANCH" --depth 1 "$URL.git" "$DEST"
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

# ── Print usage / curl hint ────────────────────────────────
cat <<EOF

  ${W}backup-restore${N}  —  Backup & restore for Linux reinstall

EOF

ensure_rust
clone_repo
build_binary
shell_aliases

info "Python version also available at: $DEST/backup-for-reinstall.py"
cat <<EOF

  ${W}Usage:${N}  backup -b              (backup, auto-detect path)
          backup -b /path    (backup, custom path)
          backup -r /path    (restore interactively)

EOF
