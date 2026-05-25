//! Escribe eventos de auditoria en agentguard-store.
//!
//! Cada decision de enforcement (allow, deny, ask) produce un AuditEvent.
//! Fail-closed: si la DB no esta disponible, se aplica deny por defecto.

use agentguard_core::{AgentLabel, AuditEvent, FileOp, GuardResult, PolicyDecision, PolicySource};
use agentguard_store::Store;
use chrono::Utc;
use std::path::Path;

pub struct Auditor {
    store: Store,
}

impl Auditor {
    pub fn new(store: Store) -> Self {
        Auditor { store }
    }

    pub fn log_decision(
        &self,
        agent_pid: u32,
        agent_label: AgentLabel,
        file_path: &Path,
        operation: FileOp,
        decision: &PolicyDecision,
        source: PolicySource,
    ) -> GuardResult<()> {
        let event = AuditEvent {
            id: None,
            agent_pid,
            agent_label,
            file_path: file_path.to_path_buf(),
            operation,
            decision: decision.clone(),
            source,
            timestamp: Utc::now(),
        };
        self.store.insert_audit_event(&event).map(|_| ())
    }
}
