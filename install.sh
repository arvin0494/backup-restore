#!/usr/bin/env bash
set -euo pipefail

REPO="arvin0494/backup-restore"
BRANCH="main"
SSH_URL="git@github.com:$REPO"
HTTPS_URL="https://github.com/$REPO.git"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/backup-restore"
BIN="${XDG_BIN_HOME:-$HOME/.local/bin}/backup"

R=$'\033[31m' G=$'\033[32m' Y=$'\033[33m' C=$'\033[36m' B=$'\033[1m' D=$'\033[2m' N=$'\033[0m'

header() {
    printf "\n${C}   ╭──────────────────────────────────────────╮${N}\n"
    printf "${C}   │${B}         backup-restore installer${N}        ${C}│${N}\n"
    printf "${C}   │${D}       Linux Backup & Restore Tool${N}       ${C}│${N}\n"
    printf "${C}   ╰──────────────────────────────────────────╯${N}\n\n"
}

section() {
    local n="$1" title="$2"
    printf "   ${D}──${N} ${B}${C}%s${N} ${D}%s${N}\n" "$n" "$title"
}

step()    { printf "  ${C}◇${N} %s\n" "$*"; }
ok()      { printf "  ${G}◆${N} %-28s ${G}%s${N}\n" "$1" "$2"; }
warn()    { printf "  ${Y}◇${N} %s\n" "$*"; }
info()    { printf "  ${D}◇${N} %-28s ${D}%s${N}\n" "$1" "$2"; }
success() { printf "\n  ${B}${G}◆  %s${N}\n" "$*"; }
fail()    { printf "\n  ${B}${R}◆  %s${N}\n" "$*"; }

spin() {
    local pid=$1 msg="$2" s
    s=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
    while kill -0 "$pid" 2>/dev/null; do
        for c in "${s[@]}"; do
            printf "\r  ${C}%s${N} %s" "$c" "$msg"
            sleep 0.08
        done
    done
    printf "\r  ${G}◆${N} %s\n" "$msg"
}

ensure_rust() {
    [[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"

    if command -v rustc &>/dev/null && command -v cargo &>/dev/null; then
        ok "Rust" "$(rustc --version)"
        return 0
    fi

    if [[ -x "$HOME/.cargo/bin/rustc" && -x "$HOME/.cargo/bin/cargo" ]]; then
        export PATH="$HOME/.cargo/bin:$PATH"
        ok "Rust" "$("$HOME/.cargo/bin/rustc" --version)"
        return 0
    fi

    printf "\n  ${Y}Rust is not installed. Choose method:${N}\n"
    printf "  ${B}  1${N}) rustup (recommended)\n"
    printf "  ${B}  2${N}) system package manager (pacman / apt / dnf / zypper / apk)\n"
    printf "  ${B}  3${N}) skip — I'll install it myself\n"
    printf "\n  ${B}  Choose [1]:${N} "
    read -r ans < /dev/tty
    case "${ans:-1}" in
        1|"")
            step "Installing rustup…"
            local log=$(mktemp)
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
                | sh -s -- -y --no-modify-path >"$log" 2>&1 &
            spin $! "Installing rustup"
            . "$HOME/.cargo/env"
            ok "Rust" "$(rustc --version)"
            rm -f "$log"
            ;;
        2)
            if command -v pacman &>/dev/null; then
                pkg_cmd="sudo pacman -S --noconfirm rust"
            elif command -v apt-get &>/dev/null; then
                pkg_cmd="sudo apt-get install -y rustc cargo"
            elif command -v dnf &>/dev/null; then
                pkg_cmd="sudo dnf install -y rust cargo"
            elif command -v zypper &>/dev/null; then
                pkg_cmd="sudo zypper install -y rust cargo"
            elif command -v apk &>/dev/null; then
                pkg_cmd="sudo apk add rust cargo"
            else
                fail "No known package manager found."
                exit 1
            fi
            step "Installing via package manager…"
            $pkg_cmd
            if ! command -v rustc &>/dev/null; then
                fail "Rust not found after install."
                exit 1
            fi
            ok "Rust" "$(rustc --version)"
            ;;
        *)
            fail "Rust is required."
            exit 1
            ;;
    esac
}

clone_repo() {
    if [[ -d "$DEST" ]]; then
        (git -C "$DEST" fetch origin "$BRANCH" 2>/dev/null; \
         git -C "$DEST" reset --hard "origin/$BRANCH" 2>/dev/null) &
        spin $! "Updating existing repository"
        ok "Clone" "$DEST"
    else
        (git clone --branch "$BRANCH" --depth 1 "$HTTPS_URL" "$DEST" 2>/dev/null || \
         git clone --branch "$BRANCH" --depth 1 "$SSH_URL" "$DEST") &
        spin $! "Cloning repository"
        ok "Clone" "$DEST"
    fi
}

build_binary() {
    step "Compiling..."
    cargo build --release --manifest-path "$DEST/backup-rs/Cargo.toml" 2>&1 | while IFS= read -r line; do
        [[ "$line" == "   Compiling "* ]] && printf "\r  ${C}⠙${N} %s" "${line#   }"
        [[ "$line" == "    Finished"* ]] && printf "\r  ${G}◆${N} Build complete\n"
    done

    if [[ ! -f "$DEST/backup-rs/target/release/backup" ]]; then
        fail "Build failed."
        exit 1
    fi

    step "Installing..."
    mkdir -p "$(dirname "$BIN")"
    cp "$DEST/backup-rs/target/release/backup" "$BIN"
    chmod +x "$BIN"
    ok "Binary" "$BIN"
}

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
        line="alias bckup='$BIN'"
        if ! grep -sqE "^alias bckup[= ']" "$rc" 2>/dev/null; then
            echo "$line" >> "$rc"
            ok "Alias" "injected"
        else
            ok "Alias" "already set"
        fi
    else
        line="alias bckup='$BIN'"
        if ! grep -sqF "alias bckup=" "$rc" 2>/dev/null; then
            echo "" >> "$rc"
            echo "# backup-restore" >> "$rc"
            echo "$line" >> "$rc"
            ok "Alias" "injected"
        else
            ok "Alias" "already set"
        fi
    fi
}

show_changelog() {
    local changelog="$1/CHANGELOG.md"
    local self="$(dirname "$(readlink -f "$0")")/CHANGELOG.md"
    [[ -f "$self" ]] && changelog="$self"
    if [ ! -f "$changelog" ]; then return; fi
    printf "\n   ${D}──${N} ${B}${C}What's new${N}\n"
    while IFS= read -r line; do
        case "$line" in
            "## v"*) printf "  ${B}${C}%s${N}\n" "${line### }" ;;
            "- "*)   printf "  ${D}%s${N}\n" "${line}" ;;
        esac
    done < "$changelog"
    echo
}

create_config() {
    local cfg_dir="$HOME/.config/backup-restore"
    local cfg_file="$cfg_dir/config"
    if [[ -f "$cfg_file" ]]; then
        ok "Config" "$cfg_file"
        return
    fi
    mkdir -p "$cfg_dir"
    cat > "$cfg_file" <<'EOF'
# backup-restore configuration
# Uncomment and edit to override defaults.

BACKUP_BASE=/mnt/HDD4T/BACKUP
# VM_QEMU_SRC=/etc/libvirt/qemu
# VM_IMAGES_SRC=/var/lib/libvirt/images
# BACKUP_EXTRA_DIRS=/path/to/something,/another/path

# ── Android FTP backup ───────────────────────────────────
# Uses rclone copy via FTP. Incremental — only new/changed files.
# Required for Android backup.
#
# Setup (CX File Explorer):
#   1. Open CX File Explorer → Network tab → FTP
#   2. Tap "Start" (set port/password if needed)
#   3. Add the settings below to this file
#
# Setup (any FTP server app):
#   Install "WiFi FTP Server" or similar from Play Store
#   Start the server and note host/port/user/pass
#
# ANDROID_FTP_HOST=192.168.44.13
# ANDROID_FTP_PORT=2121
# ANDROID_FTP_USER=ftp
# ANDROID_FTP_PASS=0000
EOF
    ok "Config" "$cfg_file"
}

main() {
    header

    info "User" "$(whoami)"
    info "Target" "$BIN"

    echo
    section "1" "Fetching source"
    ensure_rust
    clone_repo

    echo
    section "2" "Building binary"
    build_binary

    show_changelog "$DEST"

    echo
    section "3" "Setting up"
    shell_aliases
    create_config

    echo
    success "Install complete!"
    step "Run ${B}${C}bckup -b${N} or ${B}${C}bckup --help${N}"
    echo
}

main "$@"
