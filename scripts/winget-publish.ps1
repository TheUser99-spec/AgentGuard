# AgentGuard Winget Publisher
# Automates the winget-pkgs submission. One script, no manual steps.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\winget-publish.ps1

param(
    [string]$GitHubUser = "TheUser99-spec"
)

$ErrorActionPreference = "Stop"
$ManifestVersion = "0.1.2"
$TempDir = "$env:TEMP\winget-pkgs-fork"
$AgentGuardRoot = (Resolve-Path "$PSScriptRoot\..").Path

Write-Host "=== AgentGuard Winget Publisher v$ManifestVersion ===" -ForegroundColor Cyan
Write-Host ""

# ── Step 1: Fork winget-pkgs ─────────────────────────────────

$forkUrl = "https://github.com/microsoft/winget-pkgs/fork"
Write-Host "[1/4] You need a fork of microsoft/winget-pkgs." -ForegroundColor Yellow
Write-Host "      Opening fork page in your browser..." -ForegroundColor Yellow
Write-Host "      Click 'Create fork' and then press ENTER here." -ForegroundColor Yellow
Start-Process $forkUrl
Read-Host "Press ENTER after you've created the fork"

# ── Step 2: Clone the fork ───────────────────────────────────

Write-Host "[2/4] Cloning your fork..." -ForegroundColor Yellow
if (Test-Path $TempDir) {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

$cloneUrl = "https://github.com/$GitHubUser/winget-pkgs.git"
git clone --depth 1 $cloneUrl $TempDir 2>&1 | Out-Null

if (-not (Test-Path "$TempDir\.git")) {
    Write-Host "ERROR: Clone failed. Make sure you created the fork." -ForegroundColor Red
    exit 1
}

Push-Location $TempDir

# ── Step 3: Copy manifests ───────────────────────────────────

$ManifestPath = "manifests\t\$GitHubUser\AgentGuard\$ManifestVersion"
Write-Host "[3/4] Copying manifests to $ManifestPath..." -ForegroundColor Yellow

New-Item -ItemType Directory -Force -Path $ManifestPath | Out-Null
Copy-Item "$AgentGuardRoot\winget\*" $ManifestPath\ -Force

# ── Step 4: Commit, push, open PR ────────────────────────────

Write-Host "[4/4] Pushing and creating PR..." -ForegroundColor Yellow

$Branch = "agentguard-$ManifestVersion"
git checkout -b $Branch 2>$null
git checkout $Branch
git add $ManifestPath
git commit -m "Add AgentGuard version $ManifestVersion"

# Push - will prompt for credentials if needed
Write-Host "      Pushing to your fork (you may be asked to login)..." -ForegroundColor Yellow
git push origin $Branch

Pop-Location

# Open PR in browser
$PrTitle = "Add AgentGuard version $ManifestVersion"
$PrBody = "OS-level file safety for AI coding agents.`n`nRepo: https://github.com/$GitHubUser/AgentGuard"
$PrUrl = "https://github.com/microsoft/winget-pkgs/compare/main...${GitHubUser}:$Branch`?expand=1&title=$([uri]::EscapeDataString($PrTitle))&body=$([uri]::EscapeDataString($PrBody))"

Write-Host ""
Write-Host "=== Done! Opening PR in browser... ===" -ForegroundColor Green
Start-Process $PrUrl
Write-Host ""
Write-Host "If the browser doesn't open, go to:" -ForegroundColor Yellow
Write-Host "  $PrUrl"
Write-Host ""
Write-Host "After the PR is merged (~1-2 days), users can install with:" -ForegroundColor White
Write-Host "  winget install TheUser99-spec.AgentGuard" -ForegroundColor Cyan
