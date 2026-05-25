use agentguard_core::GuardResult;
use agentguard_ipc::IpcClient;

pub async fn run(no_create: bool) -> GuardResult<()> {
    let cwd = std::env::current_dir().map_err(agentguard_core::GuardError::Io)?;

    let toml_path = cwd.join("agentguard.toml");
    if !no_create && !toml_path.exists() {
        let name = cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "my-project".to_string());

        let content = agentguard_manifest::ProjectManifest::example(&name);
        std::fs::write(&toml_path, content).map_err(agentguard_core::GuardError::Io)?;

        println!("+ Creado agentguard.toml");
    }

    match ensure_daemon_running().await {
        Ok(()) => {}
        Err(e) => {
            println!("- Daemon not available: {e}");
            println!("  Start it manually: cargo run -p agentguard-daemon");
            println!("  Or: agentguard daemon start");
            return Ok(());
        }
    }

    IpcClient::new().register_project(cwd.clone()).await?;

    println!("+ Proyecto registrado: {}", cwd.display());
    println!("+ AgentGuard activo -- los agentes seran vigilados en este workspace");
    println!();
    println!("  Edita agentguard.toml para personalizar los permisos.");
    println!("  El daemon recargara automaticamente cuando guardes cambios.");
    println!();
    println!("  agentguard status              -> ver estado");
    println!("  agentguard project check ...   -> dry-run de una operacion");

    Ok(())
}

async fn ensure_daemon_running() -> GuardResult<()> {
    if IpcClient::new().get_status().await.is_ok() {
        return Ok(());
    }
    super::daemon::start().await
}
