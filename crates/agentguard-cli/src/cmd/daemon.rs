use agentguard_core::{GuardError, GuardResult};
use agentguard_ipc::IpcClient;
use std::time::Duration;

pub async fn start() -> GuardResult<()> {
    #[cfg(windows)]
    {
        let exe = daemon_exe_path();
        if !exe.exists() {
            return Err(GuardError::IpcError(format!(
                "Daemon binary not found at {:?}. Build it first: cargo build -p agentguard-daemon",
                exe
            )));
        }

        std::process::Command::new(&exe)
            .spawn()
            .map_err(|e| GuardError::IpcError(format!("Failed to spawn {:?}: {e}", exe)))?;

        let client = IpcClient::new();
        for _ in 0..30 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let ok = tokio::time::timeout(Duration::from_millis(500), client.get_status())
                .await
                .map(|r| r.is_ok())
                .unwrap_or(false);

            if ok {
                println!("+ Daemon started");
                return Ok(());
            }
        }
        Err(GuardError::IpcError(
            "Daemon not responding after 3s — check daemon output for errors".into(),
        ))
    }

    #[cfg(not(windows))]
    {
        println!("* Daemon only available on Windows.");
        println!("  Dev: cargo run -p agentguard-daemon");
        Ok(())
    }
}

pub async fn stop() -> GuardResult<()> {
    IpcClient::new().shutdown().await?;
    println!("+ Daemon stopped");
    Ok(())
}

pub async fn restart() -> GuardResult<()> {
    stop().await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    start().await
}

#[cfg(windows)]
fn daemon_exe_path() -> std::path::PathBuf {
    let mut exe = std::env::current_exe().unwrap_or_default();
    exe.set_file_name("agentguard-daemon.exe");
    exe
}
