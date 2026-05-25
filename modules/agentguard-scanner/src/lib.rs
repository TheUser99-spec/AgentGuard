//! AgentGuard Scanner — Phase 3: Analisis de codigo generado por IA.
//!
//! NO importa agentguard-enforce, agentguard-probe, ni agentguard-daemon.

use agentguard_core::GuardResult;

pub struct Scanner;

impl Scanner {
    pub fn name() -> &'static str {
        "code-scanner"
    }
}
