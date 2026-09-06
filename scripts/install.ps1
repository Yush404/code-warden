# Requires -RunAsAdministrator
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

Write-Host "=== Code-Warden (cwd) Windows Master Installer ===" -ForegroundColor Cyan

if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Error "Winget (Windows Package Manager) was not found."
    exit 1
}

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

Write-Host "`n[1/6] Checking and Installing System Runtimes..." -ForegroundColor Yellow
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

$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

Write-Host "`n[2/6] Verifying Ollama & Pulling Qwen 2.5 Coder..." -ForegroundColor Yellow
Start-Process "ollama" -ArgumentList "serve" -WindowStyle Hidden -ErrorAction SilentlyContinue
Start-Sleep -Seconds 3
& ollama pull qwen2.5-coder:1.5b

$CwHome = Join-Path $env:USERPROFILE ".code-warden"
$EnginesDir = Join-Path $CwHome "engines"
$BinDir = Join-Path $CwHome "bin"
$VenvDir = Join-Path $EnginesDir "venv"

New-Item -ItemType Directory -Force -Path $EnginesDir | Out-Null
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $CwHome "memory") | Out-Null

Write-Host "`n[3/6] Cloning Persona Repositories..." -ForegroundColor Yellow

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

Write-Host "`n[4/6] Setting up Virtual Environment and Native Tools..." -ForegroundColor Yellow

if (-not (Test-Path $VenvDir)) {
    python -m venv $VenvDir
}
$PipExe = Join-Path $VenvDir "Scripts\pip.exe"
& $PipExe install --upgrade pip setuptools wheel --quiet
& $PipExe install (Join-Path $EnginesDir "modelscan") (Join-Path $EnginesDir "checkov") (Join-Path $EnginesDir "sqlfluff") --quiet

Push-Location (Join-Path $EnginesDir "actionlint"); go build -o (Join-Path $BinDir "actionlint.exe") ./cmd/actionlint; Pop-Location
Push-Location (Join-Path $EnginesDir "trufflehog"); go build -o (Join-Path $BinDir "trufflehog.exe") .; Pop-Location
Push-Location (Join-Path $EnginesDir "go-licenses"); go build -o (Join-Path $BinDir "go-licenses.exe") .; Pop-Location

Write-Host "`n[5/6] Compiling and Registering Code-Warden (cwd.exe)..." -ForegroundColor Yellow
cargo build --release --bin cwd

$InstallBinDir = Join-Path $env:LOCALAPPDATA "Programs\CodeWarden"
New-Item -ItemType Directory -Force -Path $InstallBinDir | Out-Null
Copy-Item "target\release\cwd.exe" -Destination (Join-Path $InstallBinDir "cwd.exe") -Force

$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallBinDir*") {
    [System.Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallBinDir", "User")
}

Write-Host "`n[6/6] Configuration" -ForegroundColor Yellow
Write-Host "Select AI Provider Mode:"
Write-Host "  1) Hybrid (Gemini with automatic Ollama fallback) [Default]"
Write-Host "  2) Local Only (Ollama qwen2.5-coder:1.5b)"
Write-Host "  3) Gemini Only"
$Choice = Read-Host "Enter choice [1-3]"

$ConfigPath = Join-Path $CwHome "config.env"
if ($Choice -eq "2") {
    "DEFAULT_PROVIDER=ollama" | Out-File -FilePath $ConfigPath -Encoding utf8
} elseif ($Choice -eq "3") {
    "DEFAULT_PROVIDER=gemini" | Out-File -FilePath $ConfigPath -Encoding utf8
    $Key = Read-Host "Enter Gemini API key"
    if (-not [string]::IsNullOrWhiteSpace($Key)) {
        "GEMINI_API_KEY=$($Key.Trim())" | Out-File -FilePath $ConfigPath -Append -Encoding utf8
    }
} else {
    "DEFAULT_PROVIDER=hybrid" | Out-File -FilePath $ConfigPath -Encoding utf8
    $Key = Read-Host "Enter Gemini API key (Press enter to skip)"
    if (-not [string]::IsNullOrWhiteSpace($Key)) {
        "GEMINI_API_KEY=$($Key.Trim())" | Out-File -FilePath $ConfigPath -Append -Encoding utf8
    }
}

Write-Host "`n=== Installation Complete! ===" -ForegroundColor Green
