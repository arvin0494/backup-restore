# backup-restore installer for Windows
# Uses bundled deps/ folder. Downloads only if deps are missing.

# Auto-elevate to Administrator if needed (required for PATH modification)
$isAdmin = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $isAdmin.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $scriptPath = $MyInvocation.MyCommand.Path
    Start-Process powershell -ArgumentList "-NoProfile", "-ExecutionPolicy Bypass", "-File", "`"$scriptPath`"" -Verb RunAs
    exit
}

$ErrorActionPreference = "Continue"

$REPO       = "arvin0494/backup-restore"
$BRANCH     = "feat/cross-platform"
$HTTPS_URL  = "https://github.com/$REPO.git"
$DEST       = "$env:USERPROFILE\Projects\backup-restore"
$BIN_NAME   = "backup.exe"
$BIN_DIR    = "$env:USERPROFILE\bin"
$BIN_PATH   = "$BIN_DIR\$BIN_NAME"
$SCRIPT_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path
$DEPS_DIR   = Join-Path $SCRIPT_DIR "deps"
$RUSTUP_EXE = Join-Path $DEPS_DIR "rustup-init.exe"
$RCLONE_ZIP = Join-Path $DEPS_DIR "rclone-v1.71.0-windows-amd64.zip"
$FZF_ZIP    = Join-Path $DEPS_DIR "fzf-0.73.1-windows_amd64.zip"

$R  = "`e[31m"; $G  = "`e[32m"; $Y  = "`e[33m"; $C  = "`e[36m"
$W  = "`e[37m"; $B = "`e[1m";  $D = "`e[2m"; $N = "`e[0m"

function Show-Header {
    Write-Host ""
    Write-Host "   +----------------------------------------+" -ForegroundColor Cyan
    Write-Host "   |  backup-restore installer              |" -ForegroundColor Cyan
    Write-Host "   |  Windows Backup & Restore Tool         |" -ForegroundColor Cyan
    Write-Host "   +----------------------------------------+" -ForegroundColor Cyan
    Write-Host ""
}

function Show-Section {
    param($n, $title)
    Write-Host "   --- $title [$n]" -ForegroundColor DarkGray
}

function Show-Step   { Write-Host "  [>] $args" -ForegroundColor Cyan }
function Show-Ok     { Write-Host "  [OK] $args" -ForegroundColor Green }
function Show-Warn   { Write-Host "  [!] $args" -ForegroundColor Yellow }
function Show-Info   { Write-Host "  [*] $args" -ForegroundColor DarkGray }
function Show-Success { Write-Host ""; Write-Host "  SUCCESS: $args" -ForegroundColor Green; Write-Host "" }
function Show-Fail   { Write-Host ""; Write-Host "  FAIL: $args"; exit 1 }

function Test-Command {
    param($name)
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

# --- 1. RUST (bundled rustup-init.exe) ---
function Ensure-Rust {
    if (Test-Command rustc -and Test-Command cargo) {
        $ver = & rustc --version 2>&1
        Show-Ok "Rust $ver"
        return
    }

    Show-Warn "Rust is not installed. Installing from bundled deps..."

    # Use bundled rustup-init.exe
    if (-not (Test-Path $RUSTUP_EXE)) {
        Show-Warn "Bundled rustup not found in deps/. Downloading..."
        $RUSTUP_EXE = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $RUSTUP_EXE -UseBasicParsing
    }

    Show-Step "Installing Rust (non-interactive)..."
    $env:RUSTUP_INIT_NON_INTERACTIVE = "1"
    $env:RUSTUP_INIT_NO_MODIFY_PATH = "1"
    try {
        & $RUSTUP_EXE default-toolchain stable 2>&1 | Out-Null
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
        [System.Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")
    } catch {
        Show-Fail "Rust installation failed: $_"
    }
    finally {
        Remove-Item Env:\RUSTUP_INIT_NON_INTERACTIVE -ErrorAction SilentlyContinue
        Remove-Item Env:\RUSTUP_INIT_NO_MODIFY_PATH -ErrorAction SilentlyContinue
    }

    Start-Sleep -Seconds 3
    if (Test-Command rustc) {
        $ver = & rustc --version 2>&1
        Show-Ok "Rust $ver"
    } else {
        Show-Fail "Rust installation failed. Try running $RUSTUP_EXE manually."
    }
}

# --- 2. FZF (bundled zip) ---
function Ensure-Fzf {
    if (Test-Command fzf) {
        $ver = & fzf --version 2>&1 | Select-Object -First 1
        Show-Ok "fzf $ver"
        return
    }

    Show-Warn "fzf not found. Installing from bundled deps..."

    if (-not (Test-Path $FZF_ZIP)) {
        Show-Warn "Bundled fzf not found in deps/. Downloading..."
        $FZF_ZIP = "$env:TEMP\fzf.zip"
        Invoke-WebRequest -Uri "https://github.com/junegunn/fzf/releases/download/v0.73.1/fzf-0.73.1-windows_amd64.zip" -OutFile $FZF_ZIP -UseBasicParsing
    }

    $fzfDir = "$env:TEMP\fzf-install"
    Expand-Archive -Path $FZF_ZIP -DestinationPath $fzfDir -Force
    $fzfExe = Join-Path $fzfDir "fzf-0.73.1-windows_amd64" "fzf.exe"

    if (Test-Path $fzfExe) {
        New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
        Copy-Item $fzfExe (Join-Path $BIN_DIR "fzf.exe")
        $env:PATH = "$BIN_DIR;$env:PATH"
        [System.Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")
        Show-Ok "fzf installed to $BIN_DIR"
    } else {
        Show-Fail "Could not extract fzf.exe."
    }
}

# --- 3. RCLONE (bundled zip) ---
function Ensure-Rclone {
    if (Test-Command rclone) {
        $ver = & rclone --version 2>&1 | Select-Object -First 1
        Show-Ok "rclone $ver"
        return
    }

    Show-Warn "rclone not found. Installing from bundled deps..."

    if (-not (Test-Path $RCLONE_ZIP)) {
        Show-Warn "Bundled rclone not found in deps/. Downloading..."
        $RCLONE_ZIP = "$env:TEMP\rclone.zip"
        Invoke-WebRequest -Uri "https://github.com/rclone/rclone/releases/download/v1.71.0/rclone-v1.71.0-windows-amd64.zip" -OutFile $RCLONE_ZIP -UseBasicParsing
    }

    $rcloneDir = "$env:TEMP\rclone-install"
    Expand-Archive -Path $RCLONE_ZIP -DestinationPath $rcloneDir -Force
    $rcloneExe = Join-Path $rcloneDir "rclone-v1.71.0-windows-amd64" "rclone.exe"

    if (Test-Path $rcloneExe) {
        New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
        Copy-Item $rcloneExe (Join-Path $BIN_DIR "rclone.exe")
        $env:PATH = "$BIN_DIR;$env:PATH"
        [System.Environment]::SetEnvironmentVariable("PATH", $env:PATH, "User")
        Show-Ok "rclone installed to $BIN_DIR"
    } else {
        Show-Fail "Could not extract rclone.exe."
    }
}

# --- 4. CLONE / UPDATE ---
function Clone-Repo {
    if (Test-Path $DEST) {
        Show-Step "Updating existing repository..."
        git -C $DEST fetch origin $BRANCH 2>&1 | Out-Null
        git -C $DEST reset --hard "origin/$BRANCH" 2>&1 | Out-Null
    } else {
        Show-Step "Cloning repository..."
        git clone --branch $BRANCH --depth 1 $HTTPS_URL $DEST 2>&1 | Out-Null
        if (-not $?) {
            Show-Fail "Git clone failed. Check your network or use SSH."
        }
    }
    Show-Ok "Clone: $DEST"
}

# --- 5. BUILD ---
function Build-Binary {
    Show-Step "Compiling..."
    $cargoDir = "$DEST\backup-rs"
    $result = cargo build --release --manifest-path (Join-Path $cargoDir "Cargo.toml") 2>&1
    Write-Host $result

    if (-not $?) {
        Show-Fail "Build failed."
    }

    $binary = Join-Path $cargoDir "target\release\backup.exe"
    if (-not (Test-Path $binary)) {
        Show-Fail "Build failed - binary not found."
    }

    Show-Step "Installing..."
    New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
    Copy-Item $binary $BIN_PATH
    Show-Ok "Binary: $BIN_PATH"
}

# --- 6. POWERSHELL ALIAS ---
function Set-Alias {
    $profilePath = $PROFILE
    $aliasLine = "Set-Alias bckup '$BIN_PATH'"

    if (Test-Path $profilePath) {
        $content = Get-Content $profilePath -Raw -ErrorAction SilentlyContinue
        if ($content -match [regex]::Escape($aliasLine)) {
            Show-Ok "Alias: already set in $profilePath"
            return
        }
    }

    New-Item -ItemType Directory -Force -Path (Split-Path $profilePath) | Out-Null
    if (Test-Path $profilePath) {
        Add-Content -Path $profilePath -Value "`n# backup-restore`n$aliasLine"
    } else {
        Set-Content -Path $profilePath -Value "$aliasLine"
    }
    Show-Ok "Alias: injected into $profilePath"
    Show-Step "Run: bckup"
}

# --- 7. CONFIG ---
function Create-Config {
    $cfgDir = "$env:USERPROFILE\.config\backup-restore"
    $cfgFile = Join-Path $cfgDir "config"

    if (Test-Path $cfgFile) {
        Show-Ok "Config: $cfgFile"
        return
    }

    New-Item -ItemType Directory -Force -Path $cfgDir | Out-Null
    $configContent = @"
# backup-restore configuration for Windows
# Uncomment and edit to override defaults.

# BACKUP_BASE=E:\BACKUP
# BACKUP_EXTRA_DIRS=D:\MyDocuments,E:\Projects

# Browser Profiles - backs up Firefox, Chrome, Chromium, Brave
# Located in %APPDATA% and %LOCALAPPDATA%

# SSH & GPG - backs up ~/.ssh, ~/.gnupg, ~/.gitconfig
"@
    Set-Content -Path $cfgFile -Value $configContent -Encoding UTF8
    Show-Ok "Config: $cfgFile"
}

# --- MAIN ---
Show-Header

Show-Info "User: $env:USERNAME"
Show-Info "Target: $BIN_PATH"
Show-Info "Deps: $DEPS_DIR"

if (Test-Path $DEPS_DIR) {
    Show-Ok "Bundled deps found: $(Get-ChildItem $DEPS_DIR | Measure-Object | Select-Object -ExpandProperty Count) files"
} else {
    Show-Warn "No deps/ folder found. Will download dependencies."
}

Write-Host ""
Show-Section "1" "Installing Rust"
Ensure-Rust

Write-Host ""
Show-Section "2" "Fetching source"
Clone-Repo

Write-Host ""
Show-Section "3" "Dependencies"
Ensure-Fzf
Ensure-Rclone

Write-Host ""
Show-Section "4" "Building binary"
Build-Binary

Write-Host ""
Show-Section "5" "Setting up"
Set-Alias
Create-Config

Show-Success "Install complete!"
Show-Step "Run $Bbckup -b$N or $Bbckup --help$N"

Write-Host ""
Write-Host "  Press Enter to close..." -ForegroundColor DarkGray
Read-Host | Out-Null
