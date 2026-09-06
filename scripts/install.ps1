# Requires -RunAsAdministrator
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

Write-Host "=== Code-Warden (cwd) Windows Master Installer ===" -ForegroundColor Cyan

# 1. Ensure Winget is present
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Error "Winget (Windows Package Manager) was not found. Please install the App Installer from the Microsoft Store."
    exit 1
}

# 2. Package Dependency Definitions
$packages = @(
    @{ Name = "Git"; Id = "Git.Git" },
    @{ Name = "Go"; Id = "GoLang.Go" },
    @{ Name = "Python 3.12"; Id = "Python.Python.3.12" },
    @{ Name = "NodeJS"; Id = "OpenJS.NodeJS" },
    @{ Name = "OpenJDK 17"; Id = "Microsoft.OpenJDK.17" },
    @{ Name = "Ollama"; Id = "Ollama.Ollama" },
    @{ Name = "Rust Toolchain"; Id = "Rustlang.Rustup" },
    @{ Name = "Visual Studio 2022 Build Tools"; Id = "Microsoft.VisualStudio.2022.BuildTools" }
)

Write-Host "`n[1/5] Checking and Installing System Runtimes..." -ForegroundColor Yellow
foreach ($pkg in $packages) {
    Write-Host "Checking $($pkg.Name)... " -NoNewline
    $installed = winget list --id $pkg.Id --exact --accept-source-agreements 2>$null
    if ($LASTEXITCODE -eq 0 -and $installed -match [regex]::Escape($pkg.Id)) {
        Write-Host "[INSTALLED]" -ForegroundColor Green
    } else {
        Write-Host "[INSTALLING]" -ForegroundColor Yellow
        winget install --id $pkg.Id --exact --silent --accept-package-agreements --accept-source-agreements
    }
}

# Refresh Environment Variables for the current session
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# 3. Setup Directories
$CwHome = Join-Path $env:USERPROFILE ".code-warden"
$EnginesDir = Join-Path $CwHome "engines"
$BinDir = Join-Path $CwHome "bin"
$VenvDir = Join-Path $EnginesDir "venv"

New-Item -ItemType Directory -Force -Path $EnginesDir | Out-Null
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $CwHome "memory") | Out-Null

# 4. Clone Engine Repositories
Write-Host "`n[2/5] Cloning Persona Repositories..." -ForegroundColor Yellow

function Clone-Or-Pull($url, $folderName) {
    $dest = Join-Path $EnginesDir $folderName
    if (Test-Path (Join-Path $dest ".git")) {
        Write-Host "  -> Updating $folderName..." -ForegroundColor Gray
        git -C $dest pull --ff-only
    } else {
        Write-Host "  -> Cloning $folderName..." -ForegroundColor Gray
        git clone --depth 1 $url $dest
    }
}

Clone-Or-Pull "https://github.com/facebook/infer.git" "infer"
Clone-Or-Pull "https://github.com/pmd/pmd.git" "pmd"
Clone-Or-Pull "https://github.com/RetireJS/retire.js.git" "retire.js"
Clone-Or-Pull "https://github.com/semgrep/semgrep-rules.git" "semgrep-rules"
Clone-Or-Pull "https://github.com/protectai/modelscan.git" "modelscan"
Clone-Or-Pull "https://github.com/leondz/garak.git" "garak"
Clone-Or-Pull "https://github.com/trufflesecurity/trufflehog.git" "trufflehog"
Clone-Or-Pull "https://github.com/liamg/traitor.git" "traitor"
Clone-Or-Pull "https://github.com/rhysd/actionlint.git" "actionlint"
Clone-Or-Pull "https://github.com/bridgecrewio/checkov.git" "checkov"
Clone-Or-Pull "https://github.com/google/go-licenses.git" "go-licenses"
Clone-Or-Pull "https://github.com/nexB/scancode-toolkit.git" "scancode-toolkit"
Clone-Or-Pull "https://github.com/sqlfluff/sqlfluff.git" "sqlfluff"
Clone-Or-Pull "https://github.com/schemacrawler/SchemaCrawler.git" "schemacrawler"

# 5. Build Python & Go Tools
Write-Host "`n[3/5] Setting up Virtual Environment and Native Tools..." -ForegroundColor Yellow

if (-not (Test-Path $VenvDir)) {
    python -m venv $VenvDir
}
$PipExe = Join-Path $VenvDir "Scripts\pip.exe"
& $PipExe install --upgrade pip setuptools wheel --quiet

Write-Host "  -> Installing Python tools (modelscan, checkov, sqlfluff)..."
& $PipExe install (Join-Path $EnginesDir "modelscan") (Join-Path $EnginesDir "checkov") (Join-Path $EnginesDir "sqlfluff") --quiet

Write-Host "  -> Building Go tools..."
$ActionlintSrc = Join-Path $EnginesDir "actionlint"
$TrufflehogSrc = Join-Path $EnginesDir "trufflehog"
$GoLicensesSrc = Join-Path $EnginesDir "go-licenses"

Push-Location $ActionlintSrc; go build -o (Join-Path $BinDir "actionlint.exe") ./cmd/actionlint; Pop-Location
Push-Location $TrufflehogSrc; go build -o (Join-Path $BinDir "trufflehog.exe") .; Pop-Location
Push-Location $GoLicensesSrc; go build -o (Join-Path $BinDir "go-licenses.exe") .; Pop-Location

# 6. Build and Register cwd.exe
Write-Host "`n[4/5] Compiling and Registering Code-Warden (cwd.exe)..." -ForegroundColor Yellow
cargo build --release --bin cwd

$InstallBinDir = Join-Path $env:LOCALAPPDATA "Programs\CodeWarden"
New-Item -ItemType Directory -Force -Path $InstallBinDir | Out-Null
Copy-Item "target\release\cwd.exe" -Destination (Join-Path $InstallBinDir "cwd.exe") -Force

# Add to User PATH if missing
$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallBinDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallBinDir", "User")
    Write-Host "[✔] Added Code-Warden to User PATH." -ForegroundColor Green
}

# 7. Optional Gemini API Setup
Write-Host "`n[5/5] Configuration" -ForegroundColor Yellow
$ConfigPrompt = Read-Host "Would you like to configure your Gemini API Studio key now? (y/N)"
if ($ConfigPrompt -match "^[yY](es)?$") {
    $UserKey = Read-Host "Enter your Gemini API key"
    if (-not [string]::IsNullOrWhiteSpace($UserKey)) {
        $ConfigPath = Join-Path $CwHome "config.env"
        "GEMINI_API_KEY=$($UserKey.Trim())" | Out-File -FilePath $ConfigPath -Encoding utf8
        Write-Host "[✔] Key saved to $ConfigPath" -ForegroundColor Green
    }
} else {
    Write-Host "You can set your API key anytime later using: cwd config set-key" -ForegroundColor Gray
}

Write-Host "`n=== Installation Complete! ===" -ForegroundColor Green
Write-Host "Restart your terminal and run 'cwd audit' to begin."
