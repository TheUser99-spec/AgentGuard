$ErrorActionPreference = 'Stop'

$InstallDir = "$env:LOCALAPPDATA\AgentGuard\bin"
$Version = "0.1.2"
$Repo = "TheUser99-spec/AgentGuard"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

Write-Host "Downloading AgentGuard v$Version..."

$ExeUrl = "https://github.com/$Repo/releases/download/v$Version/agentguard.exe"
$DaemonUrl = "https://github.com/$Repo/releases/download/v$Version/agentguard-daemon.exe"

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

Invoke-WebRequest -Uri $ExeUrl -OutFile "$InstallDir\agentguard.exe" -UserAgent "chocolatey-agentguard"
Invoke-WebRequest -Uri $DaemonUrl -OutFile "$InstallDir\agentguard-daemon.exe" -UserAgent "chocolatey-agentguard"

# Add to PATH
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
}

Write-Host "AgentGuard v$Version installed to $InstallDir"
Write-Host "Run 'agentguard init' in any project to get started."
