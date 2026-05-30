//! agentguard run — starts daemon + TUI together.
//!
//! Spawns agentguard-daemon.exe as a child process, waits for IPC,
//! then opens the TUI dashboard. Ctrl+C in the TUI sends Shutdown IPC
//! to the daemon.

use agentguard_core::{GuardError, GuardResult};
use agentguard_ipc::IpcClient;
use std::time::Duration;

pub async fn run() -> GuardResult<()> {
    // Check if daemon is already running
    if IpcClient::new().get_status().await.is_ok() {
        eprintln!("+ Daemon already running, launching TUI...");
        return super::ui::run().await;
    }

    // Spawn daemon
    let daemon_exe = daemon_binary_path();
    if !daemon_exe.exists() {
        return Err(GuardError::IpcError(format!(
            "Daemon binary not found at {:?}. Build or install AgentGuard first.",
            daemon_exe
        )));
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        std::process::Command::new(&daemon_exe)
            .creation_flags(CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| GuardError::IpcError(format!("Failed to spawn daemon: {e}")))?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new(&daemon_exe)
            .spawn()
            .map_err(|e| GuardError::IpcError(format!("Failed to spawn daemon: {e}")))?;
    }

    eprintln!("+ Waiting for daemon...");
    let client = IpcClient::new();
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if client.get_status().await.is_ok() {
            eprintln!("+ Daemon ready, launching TUI...");
            return super::ui::run().await;
        }
    }

    Err(GuardError::IpcError(
        "Daemon not responding after 3s — check daemon output".into(),
    ))
}

fn daemon_binary_path() -> std::path::PathBuf {
    let mut exe = std::env::current_exe().unwrap_or_default();
    exe.set_file_name("agentguard-daemon.exe");
    exe
}
