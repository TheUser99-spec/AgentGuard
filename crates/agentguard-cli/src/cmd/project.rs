use agentguard_core::GuardResult;
use agentguard_ipc::IpcClient;
use agentguard_manifest::{find_manifest, ProjectManifest};
use std::path::PathBuf;

pub async fn validate(path: PathBuf) -> GuardResult<()> {
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());

    let client = IpcClient::new();
    let result = client.validate_project(abs).await?;

    if result.valid {
        println!("+ agentguard.toml valido");
    } else {
        println!("- agentguard.toml invalido");
        for e in &result.errors {
            println!("  error: {e}");
        }
    }

    for w in &result.warnings {
        println!("  warning: {w}");
    }

    let s = &result.summary;
    println!();
    println!("  Resumen de politica:");
    println!("  default_mode : {}", s.default_mode);
    println!("  [deny]       : {} patrones", s.deny_patterns);
    println!("  [ask]        : {} patrones", s.ask_patterns);
    println!("  [write]      : {} patrones", s.write_patterns);
    println!("  [delete]     : {} patrones", s.delete_patterns);
    println!("  [read]       : {} patrones", s.read_patterns);

    Ok(())
}

pub async fn check(file: PathBuf, op: String) -> GuardResult<()> {
    let abs = file
        .canonicalize()
        .or_else(|_| {
            std::env::current_dir()
                .map(|cwd| cwd.join(&file))
                .and_then(|p| p.canonicalize())
        })
        .unwrap_or_else(|_| file.clone());

    let client = IpcClient::new();
    let result = client.check_file(abs, op).await?;

    let (icon, color) = match &result.decision {
        agentguard_core::PolicyDecision::Allow => ("+", "\x1b[32m"),
        agentguard_core::PolicyDecision::Deny => ("-", "\x1b[31m"),
        agentguard_core::PolicyDecision::Ask { .. } => ("?", "\x1b[33m"),
    };

    println!(
        "{color}{icon} {} -- {} -> {}\x1b[0m",
        result.path.display(),
        result.op,
        result.decision,
    );
    println!("  fuente : {}", result.source);
    println!("  razon  : {}", result.reason);

    Ok(())
}

pub async fn unregister(path: PathBuf) -> GuardResult<()> {
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());

    IpcClient::new().unregister_project(abs.clone()).await?;
    println!("+ Proyecto eliminado de la vigilancia: {}", abs.display());

    Ok(())
}

pub async fn off(path: PathBuf) -> GuardResult<()> {
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    IpcClient::new().disable_protection(abs.clone()).await?;
    println!("+ Protecciones desactivadas: {}", abs.display());
    Ok(())
}

pub async fn on(path: PathBuf) -> GuardResult<()> {
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    IpcClient::new().enable_protection(abs.clone()).await?;
    println!("+ Protecciones reactivadas: {}", abs.display());
    Ok(())
}

pub async fn reload(path: PathBuf) -> GuardResult<()> {
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    IpcClient::new()
        .send(agentguard_ipc::IpcRequest::ReloadPolicy { path: abs.clone() })
        .await?;
    println!("+ Policy reloaded from disk: {}", abs.display());
    Ok(())
}

pub async fn show() -> GuardResult<()> {
    let cwd = std::env::current_dir().map_err(agentguard_core::GuardError::Io)?;
    let toml_path = find_manifest(&cwd)?;
    let manifest = ProjectManifest::from_file(&toml_path)?;

    println!("Project policy: {}", toml_path.display());
    println!();
    println!("  default mode: {:?}", manifest.project.default);
    println!();

    print_bucket("deny", &manifest.deny.files);
    print_bucket("ask", &manifest.ask.files);
    print_bucket("full", &manifest.full.files);
    print_bucket("delete", &manifest.delete.files);
    print_bucket("write", &manifest.write.files);
    print_bucket("read", &manifest.read.files);

    Ok(())
}

fn print_bucket(name: &str, files: &[String]) {
    if files.is_empty() {
        return;
    }
    let color = match name {
        "deny" => "\x1b[31m",
        "ask" => "\x1b[33m",
        "write" => "\x1b[36m",
        _ => "\x1b[0m",
    };
    println!("{color}[{name}]\x1b[0m ({})", files.len());
    for f in files {
        println!("    {f}");
    }
    println!();
}
