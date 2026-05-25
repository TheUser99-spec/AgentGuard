//! AgentGuard Team — Phase 4: Sync de politicas y dashboard para equipos.
//!
//! NO importa agentguard-enforce, agentguard-probe, ni agentguard-daemon.

use agentguard_core::GuardResult;

pub struct TeamSync;

impl TeamSync {
    pub fn name() -> &'static str {
        "team-sync"
    }
}
