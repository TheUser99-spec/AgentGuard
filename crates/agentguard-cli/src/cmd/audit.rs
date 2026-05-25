use agentguard_core::GuardResult;
use agentguard_ipc::IpcClient;

pub async fn list(limit: usize) -> GuardResult<()> {
    let client = IpcClient::new();
    let status = client
        .get_status()
        .await
        .map_err(|_| agentguard_core::GuardError::DaemonNotRunning)?;

    let events = &status.recent_events;
    let shown = events.len().min(limit);

    if events.is_empty() {
        println!("No audit events recorded yet.");
        println!("Events are logged when agent access decisions are made.");
        return Ok(());
    }

    println!("Recent audit events ({} of {}):", shown, events.len());
    println!(
        "{:<10} {:<8} {:<10} {:<8} {:<6} FILE",
        "TIME", "DECISION", "LABEL", "PID", "OP"
    );
    println!("{}", "-".repeat(70));

    for e in events.iter().take(limit) {
        let ts = chrono::DateTime::from_timestamp(e.timestamp, 0)
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "--".into());

        let decision = e.decision.to_uppercase();
        let label = &e.agent_label;
        let pid = e.agent_pid;
        let op = &e.operation;
        let file = &e.file_path;

        println!("{ts:<10} {decision:<8} {label:<10} {pid:<8} {op:<6} {file}");
    }

    Ok(())
}
