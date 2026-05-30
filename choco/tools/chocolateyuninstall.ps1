$InstallDir = "$env:LOCALAPPDATA\AgentGuard\bin"

if (Test-Path $InstallDir) {
    # Stop daemon if running
    $agentguard = Join-Path $InstallDir "agentguard.exe"
    if (Test-Path $agentguard) {
        try {
            & $agentguard daemon stop 2>$null
        } catch {}
        Start-Sleep -Milliseconds 500
    }

    Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue
}

# Remove from PATH
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -like "*$InstallDir*") {
    $NewPath = ($CurrentPath -split ';' | Where-Object { $_ -ne $InstallDir }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
}

Write-Host "AgentGuard uninstalled."
