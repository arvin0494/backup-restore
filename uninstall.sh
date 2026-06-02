#!/usr/bin/env bash
set -euo pipefail

DEST="${XDG_DATA_HOME:-$HOME/.local/share}/backup-restore"
BIN="${XDG_BIN_HOME:-$HOME/.local/bin}/backup"
CONFIG_DIR="$HOME/.config/backup-restore"

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
    for ((i=0; i<filled; i++)); do printf "${R}█${N}"; done
    for ((i=0; i<empty; i++)); do printf "${C}░${N}"; done
    printf "${C}]${N}  ${W}%3d%%${N} %s\n" "$pct" "$msg"
}

echo ""
printf "  ${R}██╗   ██╗███╗   ██╗██╗███╗   ██╗███████╗████████╗ █████╗ ██╗     ██╗${N}\n"
printf "  ${R}██║   ██║████╗  ██║██║████╗  ██║██╔════╝╚══██╔══╝██╔══██╗██║     ██║${N}\n"
printf "  ${R}██║   ██║██╔██╗ ██║██║██╔██╗ ██║███████╗   ██║   ███████║██║     ██║${N}\n"
printf "  ${R}██║   ██║██║╚██╗██║██║██║╚██╗██║╚════██║   ██║   ██╔══██║██║     ██║${N}\n"
printf "  ${R}╚██████╔╝██║ ╚████║██║██║ ╚████║███████║   ██║   ██║  ██║███████╗███████╗${N}\n"
printf "  ${R} ╚═════╝ ╚═╝  ╚═══╝╚═╝╚═╝  ╚═══╝╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚══════╝${N}\n"
echo ""
printf "  ${W}This will remove backup-restore from your system.${N}\n"
echo ""
printf "  ${W}▸${N} Binary ....... ${C}$BIN${N}\n"
printf "  ${W}▸${N} Clone ........ ${C}$DEST${N}\n"
printf "  ${W}▸${N} Config ....... ${C}$CONFIG_DIR${N}\n"
printf "  ${W}▸${N} Shell alias .. ${C}bckup${N}\n"
echo ""

for i in {10..0}; do
    printf "  ${R}█${N} Uninstalling in ${W}${i}s${N}...  Ctrl+C to cancel\r"
    sleep 1
done
echo ""

# ── Remove binary ─────────────────────────────────────────
echo "  ${C}── REMOVING BINARY ──${N}"
if [[ -f "$BIN" ]]; then
    rm -f "$BIN"
    progress 100 "Binary removed"
    status "BIN ................................. " "$BIN"
else
    warn "Binary not found at $BIN"
fi

# ── Remove clone ──────────────────────────────────────────
echo "  ${C}── REMOVING CLONE ──${N}"
if [[ -d "$DEST" ]]; then
    rm -rf "$DEST"
    progress 100 "Clone removed"
    status "CLONE ............................... " "$DEST"
else
    warn "Clone not found at $DEST"
fi

# ── Remove config ─────────────────────────────────────────
echo "  ${C}── REMOVING CONFIG ──${N}"
if [[ -d "$CONFIG_DIR" ]]; then
    rm -rf "$CONFIG_DIR"
    progress 100 "Config removed"
    status "CONFIG ............................... " "$CONFIG_DIR"
else
    warn "Config not found at $CONFIG_DIR"
fi

# ── Remove alias from shell rc ────────────────────────────
echo "  ${C}── REMOVING SHELL ALIAS ──${N}"
for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish" "$HOME/.profile"; do
    [[ -f "$rc" ]] || continue
    if grep -sq "alias bckup=" "$rc" 2>/dev/null || grep -sq "alias bckup " "$rc" 2>/dev/null || grep -sq "alias backup=" "$rc" 2>/dev/null; then
        sed -i '/^alias bckup=/d; /^alias bckup /d; /^alias backup=/d; /^# backup-restore/d' "$rc"
        progress 100 "Alias removed from $rc"
        status "SHELL RC ............................ " "$rc"
    fi
done

# ── Done ──────────────────────────────────────────────────
echo ""
printf "  ${R}████████████████████████████████████████████${N}\n"
printf "  ${R}█${N}  ${W}UNINSTALL COMPLETE${N}                    ${R}█${N}\n"
printf "  ${R}████████████████████████████████████████████${N}\n"
echo ""
