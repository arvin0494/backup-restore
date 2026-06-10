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

BAR_W=40

progress() {
    local msg="$1" pct="$2"
    local filled=$((pct * BAR_W / 100))
    local empty=$((BAR_W - filled))
    printf "\r  ${C}(${N}${W}1${N}${C}/${N}${W}1${N}${C})${N} ${C}%s${N}" "$msg"
    local pad=$((60 - ${#msg}))
    [[ pad -lt 1 ]] && pad=1
    printf "%*s" "$pad" ""
    printf "${C}[${N}"
    for ((i=0; i<filled; i++)); do printf "${R}#${N}"; done
    for ((i=0; i<empty; i++)); do printf "${C}-${N}"; done
    printf "${C}]${N} ${W}%3d%%${N}" "$pct"
    if [[ "$pct" -ge 100 ]]; then
        printf "\n"
    fi
}

run_with_spinner() {
    local msg="$1"
    shift
    if [[ $# -eq 0 ]]; then
        printf "  ${C}(1/1)${N} ${C}%s${N}\n" "$msg"
        return
    fi
    local pid
    (
        eval "$*" >/dev/null 2>&1
    ) &
    pid=$!
    local i=0
    while kill -0 "$pid" 2>/dev/null; do
        local filled=$(( (i * 2) % (BAR_W * 2) ))
        [[ filled -gt BAR_W ]] && filled=$((BAR_W * 2 - filled))
        local bar=""
        for ((b=0; b<filled; b++)); do bar+="#"; done
        for ((b=0; b<BAR_W-filled; b++)); do bar+="-"; done
        printf "\r  ${C}(1/1)${N} ${C}%s${N}" "$msg"
        local pad=$((60 - ${#msg}))
        [[ pad -lt 1 ]] && pad=1
        printf "%*s" "$pad" ""
        printf "${C}[${R}%s${C}]${N}" "$bar"
        i=$((i + 1))
        sleep 0.08
    done
    wait "$pid"
    local rc=$?
    printf "\r  ${C}(1/1)${N} ${C}%s${N}" "$msg"
    local pad=$((60 - ${#msg}))
    [[ pad -lt 1 ]] && pad=1
    printf "%*s" "$pad" ""
    printf "${C}[${N}"
    for ((b=0; b<BAR_W; b++)); do printf "${R}#${N}"; done
    printf "${C}]${N} ${W}100%%${N}\n"
    return $rc
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
    progress "Uninstalling in ${i}s — Ctrl+C to cancel" $(( (10 - i) * 10 ))
    sleep 1
done
progress "Uninstalling in 0s — Ctrl+C to cancel" 100

echo ""
printf "  ${C}── REMOVING BINARY ──${N}\n"
if [[ -f "$BIN" ]]; then
    rm -f "$BIN"
    progress "Binary removed" 100
    status "BIN ................................. " "$BIN"
else
    warn "Binary not found at $BIN"
fi

printf "  ${C}── REMOVING CLONE ──${N}\n"
if [[ -d "$DEST" ]]; then
    rm -rf "$DEST"
    progress "Clone removed" 100
    status "CLONE ............................... " "$DEST"
else
    warn "Clone not found at $DEST"
fi

printf "  ${C}── REMOVING CONFIG ──${N}\n"
if [[ -d "$CONFIG_DIR" ]]; then
    rm -rf "$CONFIG_DIR"
    progress "Config removed" 100
    status "CONFIG ............................... " "$CONFIG_DIR"
else
    warn "Config not found at $CONFIG_DIR"
fi

printf "  ${C}── REMOVING SHELL ALIAS ──${N}\n"
for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish" "$HOME/.profile"; do
    [[ -f "$rc" ]] || continue
    if grep -sq "alias bckup=" "$rc" 2>/dev/null || grep -sq "alias bckup " "$rc" 2>/dev/null || grep -sq "alias backup=" "$rc" 2>/dev/null; then
        sed -i '/^alias bckup=/d; /^alias bckup /d; /^alias backup=/d; /^# backup-restore/d' "$rc"
        progress "Alias removed from $rc" 100
        status "SHELL RC ............................ " "$rc"
    fi
done

echo ""
printf "  ${R}████████████████████████████████████████████${N}\n"
printf "  ${R}█${N}  ${W}UNINSTALL COMPLETE${N}                    ${R}█${N}\n"
printf "  ${R}████████████████████████████████████████████${N}\n"
echo ""
