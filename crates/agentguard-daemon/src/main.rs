//! AgentGuard Daemon — thin binary entry point.
//! Delegates to the library `agentguard_daemon::run_daemon()`.

#[tokio::main]
async fn main() {
    agentguard_daemon::run_daemon().await;
}
