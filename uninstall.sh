#!/usr/bin/env bash
set -euo pipefail

PROJECT="backup-restore"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/backup-restore"
BIN="${XDG_BIN_HOME:-$HOME/.local/bin}/backup"
CONFIG_DIR="$HOME/.config/backup-restore"

R=$'\033[31m' G=$'\033[32m' Y=$'\033[33m' C=$'\033[36m' B=$'\033[1m' D=$'\033[2m' N=$'\033[0m'

printf "\n${C}   ╭──────────────────────────────────────────╮${N}\n"
printf "${C}   │${B}        backup-restore — uninstall${N}             ${C}│${N}\n"
printf "${C}   ╰──────────────────────────────────────────╯${N}\n\n"

rem()  { printf "  ${R}◆${N} %-28s ${R}removed${N}\n" "$1"; }
skip() { printf "  ${D}◆${N} %-28s ${D}not found${N}\n" "$1"; }

printf "   ${D}──${N} ${B}${C}Removing binary${N}\n"
if [[ -f "$BIN" ]]; then
    rm -f "$BIN"
    rem "Binary"
else
    skip "Binary"
fi

echo
printf "   ${D}──${N} ${B}${C}Removing clone${N}\n"
if [[ -d "$DEST" ]]; then
    rm -rf "$DEST"
    rem "Clone"
else
    skip "Clone"
fi

echo
printf "   ${D}──${N} ${B}${C}Removing config${N}\n"
if [[ -d "$CONFIG_DIR" ]]; then
    rm -rf "$CONFIG_DIR"
    rem "Config"
else
    skip "Config"
fi

echo
printf "   ${D}──${N} ${B}${C}Removing shell alias${N}\n"
aliases_removed=0
for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish" "$HOME/.profile"; do
    [[ -f "$rc" ]] || continue
    if grep -sq "alias bckup=" "$rc" 2>/dev/null || grep -sq "alias bckup " "$rc" 2>/dev/null || grep -sq "alias backup=" "$rc" 2>/dev/null || grep -sq "# backup-restore" "$rc" 2>/dev/null; then
        sed -i '/^alias bckup=/d; /^alias bckup /d; /^alias backup=/d; /^# backup-restore/d' "$rc"
        aliases_removed=$((aliases_removed+1))
        rem "Aliases ($(basename "$rc"))"
    fi
done
[ "$aliases_removed" -eq 0 ] && skip "Aliases"

printf "\n  ${B}${G}◆  Uninstall complete${N}\n"
echo
