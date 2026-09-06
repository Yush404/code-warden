# Requires -RunAsAdministrator
[CmdletBinding()]
param(
    [switch]$Yes
)

$ErrorActionPreference = "Stop"

Write-Host "=== Code-Warden Complete Uninstallation ===" -ForegroundColor Red

if (-not $Yes) {
    $Confirm = Read-Host "This will remove all 14 cloned engines, cached environments, and the 'cwd.exe' binary. Proceed? (y/N)"
    if ($Confirm -notmatch "^[yY](es)?$") {
        Write-Host "Uninstallation aborted."
        exit 0
    }
}

# 1. Purge ~/.code-warden
$CwHome = Join-Path $env:USERPROFILE ".code-warden"
if (Test-Path $CwHome) {
    Write-Host "Removing $CwHome..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $CwHome
}

# 2. Remove Binary
$InstallBinDir = Join-Path $env:LOCALAPPDATA "Programs\CodeWarden"
if (Test-Path $InstallBinDir) {
    Write-Host "Removing Code-Warden executable..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $InstallBinDir
}

# 3. Clean User PATH
$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -like "*$InstallBinDir*") {
    $NewPath = ($UserPath.Split(';') | Where-Object { $_ -ne $InstallBinDir }) -join ';'
    [System.Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
}

Write-Host "`n[✔] Code-Warden successfully removed from Windows." -ForegroundColor Green
Write-Host "Note: System toolchains (Go, Python, OpenJDK, Ollama) were left intact to avoid disrupting other applications." -ForegroundColor Gray
