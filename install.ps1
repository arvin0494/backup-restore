# backup-restore installer for Windows

$ErrorActionPreference = "Continue"

$REPO       = "arvin0494/backup-restore"
$BRANCH     = "feat/cross-platform"
$HTTPS_URL  = "https://github.com/$REPO.git"
$DEST       = "$env:USERPROFILE\Projects\backup-restore"
$BIN_NAME   = "backup.exe"
$BIN_DIR    = "$env:USERPROFILE\bin"
$BIN_PATH   = "$BIN_DIR\$BIN_NAME"

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

function Wait-Spin {
    param($task, $cmd)
    $spins = @('-','\','|','/')
    $i = 0
    while ($true) {
        Write-Host -NoNewline "`r  [$($spins[$i % 4])] $task"
        $i++
        Start-Sleep -Milliseconds 100
        $result = & $cmd
        if ($null -ne $result -or $true) {
            break
        }
    }
    Write-Host ""
    Write-Host "  [OK] $task" -ForegroundColor Green
}

function Test-Command {
    param($name)
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

# --- 1. RUST ---
function Ensure-Rust {
    if (Test-Command rustc -and Test-Command cargo) {
        $ver = & rustc --version 2>&1
        Show-Ok "Rust $ver"
        return
    }

    Write-Host ""
    Show-Warn "Rust is not installed."
    Write-Host "    1) rustup (recommended)"
    Write-Host "    2) winget install Rustlang.Rustup"
    Write-Host "    3) skip - I will install it myself"
    Write-Host ""
    $ans = Read-Host "  Choose [1]"
    if ($ans -eq "" -or $ans -eq "1") {
        Show-Step "Installing rustup..."
        $rustupPath = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $rustupPath -UseBasicParsing
        & $rustupPath /y --no-modify-path 2>&1 | Out-Null
        Remove-Item $rustupPath -ErrorAction SilentlyContinue
        if (Test-Path "$env:USERPROFILE\.cargo\bin") {
            $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
        }
        if (Test-Command rustc) {
            $ver = & rustc --version 2>&1
            Show-Ok "Rust $ver"
        } else {
            Show-Fail "Rust installation failed. Install manually: https://rustup.rs"
        }
    } elseif ($ans -eq "2") {
        Show-Step "Installing via winget..."
        if (Test-Command winget) {
            winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements 2>&1 | Out-Null
            if (Test-Path "$env:USERPROFILE\.cargo\bin") {
                $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
            }
            if (Test-Command rustc) {
                $ver = & rustc --version 2>&1
                Show-Ok "Rust $ver"
                return
            }
        }
        Show-Fail "winget not found or install failed. Install manually: https://rustup.rs"
    } else {
        Show-Fail "Rust is required."
    }
}

# --- 2. FZF ---
function Ensure-Fzf {
    if (Test-Command fzf) {
        $ver = & fzf --version 2>&1 | Select-Object -First 1
        Show-Ok "fzf $ver"
        return
    }

    Show-Warn "fzf not found (needed for restore menu)"
    $ans = Read-Host "  Install fzf? [y/N]"
    if ($ans -ne "y" -and $ans -ne "Y") {
        Show-Warn "fzf will be skipped. Restore menu will use readline instead."
        return
    }

    if (Test-Command winget) {
        Show-Step "Installing fzf via winget..."
        winget install --id Junegunn.fzf -e --silent --accept-package-agreements --accept-source-agreements 2>&1 | Out-Null
        if (Test-Command fzf) {
            $ver = & fzf --version 2>&1 | Select-Object -First 1
            Show-Ok "fzf $ver"
            return
        }
    }

    if (Test-Command choco) {
        Show-Step "Installing fzf via choco..."
        choco install fzf -y 2>&1 | Out-Null
        if (Test-Command fzf) {
            $ver = & fzf --version 2>&1 | Select-Object -First 1
            Show-Ok "fzf $ver"
            return
        }
    }

    Show-Step "Downloading fzf manually..."
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
        Show-Ok "fzf installed to $BIN_DIR"
    } else {
        Show-Fail "Could not extract fzf.exe."
    }
}

# --- 3. RCLONE ---
function Ensure-Rclone {
    if (Test-Command rclone) {
        $ver = & rclone --version 2>&1 | Select-Object -First 1
        Show-Ok "rclone $ver"
        return
    }

    Write-Host ""
    Write-Host "  [ERROR] rclone not found." -ForegroundColor Red
    Show-Step "Installing rclone..."

    $tried = $false

    if (Test-Command winget) {
        $tried = $true
        Show-Step "Installing rclone via winget..."
        winget install --id Rclone.Rclone -e --silent --accept-package-agreements --accept-source-agreements 2>&1 | Out-Null
        if (Test-Command rclone) {
            $ver = & rclone --version 2>&1 | Select-Object -First 1
            Show-Ok "rclone $ver"
            return
        }
    }

    if (Test-Command choco) {
        $tried = $true
        Show-Step "Installing rclone via choco..."
        choco install rclone -y 2>&1 | Out-Null
        if (Test-Command rclone) {
            $ver = & rclone --version 2>&1 | Select-Object -First 1
            Show-Ok "rclone $ver"
            return
        }
    }

    if (-not $tried) {
        Write-Host "  [ERROR] Winget and choco not found." -ForegroundColor Red
        Write-Host "  Install winget: https://aka.ms/winget-cli/latest"
        Write-Host "  Install choco: https://chocolatey.org/install"
        Write-Host "  Or download rclone: https://rclone.org/downloads/"
        Show-Fail "Could not install rclone automatically."
    } else {
        Show-Fail "rclone installation failed. Install manually: https://rclone.org/downloads/"
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

Write-Host ""
Show-Section "1" "Fetching source"
Ensure-Rust
Clone-Repo

Write-Host ""
Show-Section "1.5" "Checking dependencies"
Ensure-Fzf
Ensure-Rclone

Write-Host ""
Show-Section "2" "Building binary"
Build-Binary

Write-Host ""
Show-Section "3" "Setting up"
Set-Alias
Create-Config

Show-Success "Install complete!"
Show-Step "Run $Bbckup -b$N or $Bbckup --help$N"

Write-Host ""
Write-Host "  Press Enter to close..." -ForegroundColor DarkGray
Read-Host | Out-Null
