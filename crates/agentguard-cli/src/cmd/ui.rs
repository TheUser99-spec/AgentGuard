//! agentguard ui — opens the TUI dashboard.
//! Requires the daemon to be already running.

use agentguard_core::GuardResult;

pub async fn run() -> GuardResult<()> {
    agentguard_tui::run_tui()
        .await
        .map_err(|e| agentguard_core::GuardError::IpcError(format!("TUI error: {e}")))
}
