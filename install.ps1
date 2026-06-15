# ─────────────────────────────────────────────────────────────
# backup-restore installer for Windows
# ─────────────────────────────────────────────────────────────

$ErrorActionPreference = "Stop"

$REPO       = "arvin0494/backup-restore"
$BRANCH     = "feat/cross-platform"
$HTTPS_URL  = "https://github.com/$REPO.git"
$DEST       = "$env:USERPROFILE\Projects\backup-restore"
$BIN_NAME   = "backup.exe"
$BIN_DIR    = "$env:USERPROFILE\bin"
$BIN_PATH   = "$BIN_DIR\$BIN_NAME"

$R  = "`e[31m"; $G  = "`e[32m"; $Y  = "`e[33m"; $C  = "`e[36m"
$W  = "`e[37m"; $B = "`e[1m";  $D = "`e[2m"; $N = "`e[0m"

function Header {
    Write-Host ""
    Write-Host "   " -NoNewline; Write-Host "╭──────────────────────────────────────────╮" -ForegroundColor Cyan
    Write-Host "   " -NoNewline; Write-Host "│" -ForegroundColor Cyan -NoNewline
    Write-Host "     backup-restore installer" -ForegroundColor Cyan -BackgroundColor DarkCyan
    Write-Host "│" -ForegroundColor Cyan -NoNewline; Write-Host "   " -ForegroundColor Cyan
    Write-Host "   " -NoNewline; Write-Host "│" -ForegroundColor Cyan -NoNewline
    Write-Host "      Windows Backup & Restore Tool" -ForegroundColor Cyan -BackgroundColor DarkCyan
    Write-Host "│" -ForegroundColor Cyan -NoNewline; Write-Host "   " -ForegroundColor Cyan
    Write-Host "   " -NoNewline; Write-Host "╰──────────────────────────────────────────╯" -ForegroundColor Cyan
    Write-Host ""
}

function Section {
    param($n, $title)
    Write-Host "   " -NoNewline
    Write-Host "──" -ForegroundColor DarkGray -NoNewline
    Write-Host " $B$C$title$N $D($n)$N" -ForegroundColor DarkGray
}

function Step   { Write-Host "  " -NoNewline; Write-Host "◇" -ForegroundColor Cyan -NoNewline; Write-Host " $args" }
function Ok     { Write-Host "  " -NoNewline; Write-Host "◆" -ForegroundColor Green -NoNewline; Write-Host "  $args" -ForegroundColor Green }
function Warn   { Write-Host "  " -NoNewline; Write-Host "◇" -ForegroundColor Yellow -NoNewline; Write-Host " $args" }
function Info   { Write-Host "  " -NoNewline; Write-Host "◇" -ForegroundColor DarkGray -NoNewline; Write-Host "  $args" -ForegroundColor DarkGray }
function Success { Write-Host ""; Write-Host "  " -NoNewline; Write-Host "◆  $args" -ForegroundColor Green -BackgroundColor DarkGreen }
function Fail   { Write-Host ""; Write-Host "  " -NoNewline; Write-Host "◆  $args" -ForegroundColor Red; exit 1 }

function Spin {
    param($task, $cmd)
    $spins = @('⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏')
    $i = 0
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    Write-Host -NoNewline "  $C$spins[$i]$N $task"
    & $cmd 2>&1 | Out-Null
    $timer.Stop()
    Write-Host ""; Write-Host "  ◆" -ForegroundColor Green -NoNewline; Write-Host " $task"
}

# ── Check if a command exists ──
function Test-Command {
    param($name)
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

# ── 1. RUST ──
function Ensure-Rust {
    if (Test-Command rustc -and Test-Command cargo) {
        Ok "Rust" "$(rustc --version)"
        return
    }

    Write-Host ""
    Write-Host "  " -NoNewline; Write-Host "Rust is not installed." -ForegroundColor Yellow
    Write-Host "  " -NoNewline; Write-Host "  1)" -NoNewline -ForegroundColor Cyan
    Write-Host " rustup (recommended)"
    Write-Host "  " -NoNewline; Write-Host "  2)" -NoNewline -ForegroundColor Cyan
    Write-Host " winget install Rustlang.Rustup"
    Write-Host "  " -NoNewline; Write-Host "  3)" -NoNewline -ForegroundColor Cyan
    Write-Host " skip — I'll install it myself"
    Write-Host ""
    Write-Host "  " -NoNewline; Write-Host "Choose [1]:" -ForegroundColor Cyan -NoNewline

    $ans = Read-Host
    if ($ans -eq "" -or $ans -eq "1") {
        Step "Installing rustup..."
        $log = [System.IO.Path]::GetTempFileName()
        $url = "https://win.rustup.rs/x86_64"
        $rustupPath = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri $url -OutFile $rustupPath -UseBasicParsing | Out-Null
        & $rustupPath /y --no-modify-path 2>&1 | Out-Null
        Remove-Item $rustupPath -ErrorAction SilentlyContinue
        Remove-Item $log -ErrorAction SilentlyContinue

        if (Test-Path "$env:USERPROFILE\.cargo\bin\rustup") {
            $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
        }

        if (Test-Command rustc) {
            Ok "Rust" "$(rustc --version)"
        } else {
            Fail "Rust installation failed. Install manually: https://rustup.rs"
        }
    } elseif ($ans -eq "2") {
        Step "Installing via winget..."
        if (Test-Command winget) {
            winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements
            if (Test-Path "$env:USERPROFILE\.cargo\bin") {
                $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
            }
            if (Test-Command rustc) {
                Ok "Rust" "$(rustc --version)"
            } else {
                Fail "Rust not found after winget install."
            }
        } else {
            Fail "winget not found. Install Rust manually: https://rustup.rs"
        }
    } else {
        Fail "Rust is required."
    }
}

# ── 2. FZF ──
function Ensure-Fzf {
    if (Test-Command fzf) {
        Ok "fzf" "$(fzf --version | Select-Object -First 1)"
        return
    }

    Warn "fzf not found (needed for restore menu)"
    Write-Host "  " -NoNewline; Write-Host "Install fzf? [y/N]:" -ForegroundColor Cyan -NoNewline
    $ans = Read-Host
    if ($ans -ne "y" -and $ans -ne "Y") {
        Warn "fzf will be skipped. Restore menu will use readline instead."
        return
    }

    # Try winget
    if (Test-Command winget) {
        Step "Installing fzf via winget..."
        winget install --id Junegunn.fzf -e --silent --accept-package-agreements --accept-source-agreements
        if (Test-Command fzf) {
            Ok "fzf" "$(fzf --version | Select-Object -First 1)"
            return
        }
    }

    # Try choco
    if (Test-Command choco) {
        Step "Installing fzf via choco..."
        choco install fzf -y
        if (Test-Command fzf) {
            Ok "fzf" "$(fzf --version | Select-Object -First 1)"
            return
        }
    }

    # Manual download
    Step "Downloading fzf manually..."
    $fzfZip = "$env:TEMP\fzf.zip"
    $fzfVer = "0.73.1"
    Invoke-WebRequest -Uri "https://github.com/junegunn/fzf/releases/download/v$fzfVer/fzf-$fzfVer-windows.zip" -OutFile $fzfZip -UseBasicParsing
    $fzfDir = "$env:TEMP\fzf-install"
    Expand-Archive -Path $fzfZip -DestinationPath $fzfDir -Force
    Remove-Item $fzfZip -ErrorAction SilentlyContinue

    $fzfExe = Join-Path $fzfDir "fzf-$fzfVer-windows" "fzf.exe"
    if (Test-Path $fzfExe) {
        New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
        Copy-Item $fzfExe (Join-Path $BIN_DIR "fzf.exe")
        Ok "fzf" "installed to $BIN_DIR"
    } else {
        Fail "Could not extract fzf.exe."
    }
}

# ── 3. RCLONE ──
function Ensure-Rclone {
    if (Test-Command rclone) {
        Ok "rclone" "$(rclone --version | Select-Object -First 1)"
        return
    }

    Write-Host ""; Write-Host "  $Rrclone not found.$N"
    Write-Host "  $YInstalling rclone...$N"

    $tried = $false

    # winget (built into Win10 1809+, Win11)
    if (Test-Command winget) {
        $tried = $true
        Step "Installing rclone via winget..."
        winget install --id Rclone.Rclone -e --silent --accept-package-agreements --accept-source-agreements
        if (Test-Command rclone) {
            Ok "rclone" "$(rclone --version | Select-Object -First 1)"
            return
        }
    }

    # Chocolatey
    if (Test-Command choco) {
        $tried = $true
        Step "Installing rclone via choco..."
        choco install rclone -y
        if (Test-Command rclone) {
            Ok "rclone" "$(rclone --version | Select-Object -First 1)"
            return
        }
    }

    if (-not $tried) {
        Write-Host "  $RWinget and choco not found.$N"
        Write-Host "  Install winget: https://aka.ms/winget-cli/latest"
        Write-Host "  Install choco: https://chocolatey.org/install"
        Write-Host "  Or download rclone: https://rclone.org/downloads/"
        Fail "Could not install rclone automatically."
    } else {
        Write-Host "  $RFailed to install. Try manually:$N"
        Write-Host "    winget install Rclone.Rclone   or   choco install rclone"
        Write-Host "    Or download: https://rclone.org/downloads/"
        Fail "rclone installation failed."
    }
}

# ── 4. CLONE / UPDATE ──
function Clone-Repo {
    if (Test-Path $DEST) {
        Spin "Updating existing repository" { git -C $DEST fetch origin $BRANCH; git -C $DEST reset --hard "origin/$BRANCH" }
        Ok "Clone" "$DEST"
    } else {
        Spin "Cloning repository" { git clone --branch $BRANCH --depth 1 $HTTPS_URL $DEST }
        Ok "Clone" "$DEST"
    }
}

# ── 5. BUILD ──
function Build-Binary {
    Step "Compiling..."
    $cargoDir = "$DEST\backup-rs"
    $buildOut = cargo build --release --manifest-path (Join-Path $cargoDir "Cargo.toml") 2>&1

    foreach ($line in $buildOut) {
        if ($line -match "^\s*Compiling ") {
            Write-Host -NoNewline "`r  $C⠙$N $($line -replace "^\s*")"
        }
        if ($line -match "Finished") {
            Write-Host "`r  $G◆$N Build complete"
        }
    }

    $binary = Join-Path $cargoDir "target\release\backup.exe"
    if (-not (Test-Path $binary)) {
        Fail "Build failed."
    }

    Step "Installing..."
    New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
    Copy-Item $binary $BIN_PATH
    Ok "Binary" "$BIN_PATH"
}

# ── 6. POWERSHELL ALIAS ──
function Set-Alias {
    $profilePath = $PROFILE
    $aliasLine = "Set-Alias bckup '$BIN_PATH'"

    if (Test-Path $profilePath) {
        $content = Get-Content $profilePath -Raw
        if ($content -match [regex]::Escape($aliasLine)) {
            Ok "Alias" "already set in $profilePath"
            return
        }
    }

    New-Item -ItemType Directory -Force -Path (Split-Path $profilePath) | Out-Null
    if (Test-Path $profilePath) {
        Add-Content -Path $profilePath -Value "`n# backup-restore`n$aliasLine"
    } else {
        Set-Content -Path $profilePath -Value "$aliasLine"
    }
    Ok "Alias" "injected into $profilePath"
    Step "Run: bckup"
}

# ── 7. CONFIG ──
function Create-Config {
    $cfgDir = "$env:USERPROFILE\.config\backup-restore"
    $cfgFile = Join-Path $cfgDir "config"

    if (Test-Path $cfgFile) {
        Ok "Config" "$cfgFile"
        return
    }

    New-Item -ItemType Directory -Force -Path $cfgDir | Out-Null
    @'
# backup-restore configuration for Windows
# Uncomment and edit to override defaults.

# BACKUP_BASE=E:\BACKUP
# BACKUP_EXTRA_DIRS=D:\MyDocuments,E:\Projects

# ── Browser Profiles ────────────────────────────────────
# Backs up: Firefox, Chrome, Chromium, Brave
# Located in %APPDATA% and %LOCALAPPDATA%

# ── SSH & GPG ──────────────────────────────────────────
# Backs up: ~/.ssh, ~/.gnupg, ~/.gitconfig
'@ | Out-File -FilePath $cfgFile -Encoding utf8
    Ok "Config" "$cfgFile"
}

# ── MAIN ──
Header

Info "User" "$env:USERNAME"
Info "Target" "$BIN_PATH"

Write-Host ""
Section "1" "Fetching source"
Ensure-Rust
Clone-Repo

Write-Host ""
Section "1.5" "Checking dependencies"
Ensure-Fzf
Ensure-Rclone

Write-Host ""
Section "2" "Building binary"
Build-Binary

Write-Host ""
Section "3" "Setting up"
Set-Alias
Create-Config

Write-Host ""
Success "Install complete!"
Step "Run $B$bckup -b$N or $B$bckup --help$N"
Write-Host ""

# Auto-exit so the window closes
exit
