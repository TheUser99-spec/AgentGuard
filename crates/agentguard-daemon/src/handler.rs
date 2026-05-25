use agentguard_core::Bucket;
use agentguard_core::{FileOp, GuardResult};
use agentguard_ipc::{
    ActiveAgent, AuditEventView, DaemonStatus, FileCheckResult, GlobalRuleInfo,
    GlobalRulesListData, IpcRequest, IpcResponse, PolicySummary, ProjectInfo, ValidationResult,
};
use agentguard_manifest::{find_manifest, ProjectManifest};
use std::sync::Arc;

use crate::orchestrator::DaemonState;

pub fn handle(state: Arc<DaemonState>, req: IpcRequest) -> IpcResponse {
    match handle_inner(state, req) {
        Ok(resp) => resp,
        Err(e) => IpcResponse::Error {
            message: e.to_string(),
        },
    }
}

fn handle_inner(state: Arc<DaemonState>, req: IpcRequest) -> GuardResult<IpcResponse> {
    match req {
        IpcRequest::RegisterProject { path } => {
            state.register_project(path)?;
            Ok(IpcResponse::Ok)
        }

        IpcRequest::UnregisterProject { path } => {
            state.unregister_project(&path)?;
            Ok(IpcResponse::Ok)
        }

        IpcRequest::ReloadPolicy { path } => {
            state.reload_project(&path)?;
            Ok(IpcResponse::Ok)
        }

        IpcRequest::GetStatus => {
            let projects: Vec<ProjectInfo> = state
                .store
                .list_projects()?
                .into_iter()
                .map(|p| {
                    let (deny, ask, write, read) = load_counts(&p.path);
                    ProjectInfo {
                        path: p.path,
                        toml_hash: p.toml_hash,
                        added_at: p.added_at,
                        deny_count: deny,
                        ask_count: ask,
                        write_count: write,
                        read_count: read,
                    }
                })
                .collect();

            let active_agents: Vec<ActiveAgent> = state
                .tracker
                .active_sessions()
                .into_iter()
                .map(|s| ActiveAgent {
                    pid: s.pid,
                    image_name: s.image_name,
                    label: s.label,
                    workspace: s.workspace,
                    started_at: s.started_at.timestamp(),
                })
                .collect();

            let (events, blocks) = state.store.count_events_today().unwrap_or((0, 0));

            let recent_events: Vec<AuditEventView> = state
                .store
                .recent_audit_events(50)
                .unwrap_or_default()
                .into_iter()
                .map(|e| AuditEventView {
                    id: e.id.unwrap_or(0),
                    agent_pid: e.agent_pid,
                    agent_label: e.agent_label.as_str().to_string(),
                    file_path: e.file_path.to_string_lossy().to_string(),
                    operation: e.operation.as_str().to_string(),
                    decision: e.decision.as_str().to_string(),
                    source: e.source.as_str().to_string(),
                    timestamp: e.timestamp.timestamp(),
                })
                .collect();

            Ok(IpcResponse::Status(DaemonStatus {
                running: true,
                version: env!("CARGO_PKG_VERSION").to_string(),
                projects,
                active_agents,
                events_today: events,
                blocks_today: blocks,
                recent_events,
            }))
        }

        IpcRequest::ValidateProject { path } => {
            let toml_path = match find_manifest(&path) {
                Ok(p) => p,
                Err(e) => {
                    return Ok(IpcResponse::ProjectValidation(ValidationResult {
                        valid: false,
                        errors: vec![e.to_string()],
                        warnings: vec![],
                        summary: empty_summary(),
                    }))
                }
            };

            let manifest = match ProjectManifest::from_file(&toml_path) {
                Ok(m) => m,
                Err(e) => {
                    return Ok(IpcResponse::ProjectValidation(ValidationResult {
                        valid: false,
                        errors: vec![e.to_string()],
                        warnings: vec![],
                        summary: empty_summary(),
                    }))
                }
            };

            let mut errors = vec![];
            let mut warnings = vec![];

            for (bucket, patterns) in [
                ("deny", &manifest.deny.files),
                ("ask", &manifest.ask.files),
                ("write", &manifest.write.files),
                ("delete", &manifest.delete.files),
                ("read", &manifest.read.files),
                ("full", &manifest.full.files),
            ] {
                for p in patterns {
                    if globset::Glob::new(p).is_err() {
                        errors.push(format!("[{bucket}] invalid glob: '{p}'"));
                    }
                    if p == "**" || p == "**/*" {
                        warnings.push(format!("[{bucket}] '**' matches EVERYTHING — intentional?"));
                    }
                }
            }

            Ok(IpcResponse::ProjectValidation(ValidationResult {
                valid: errors.is_empty(),
                errors,
                warnings,
                summary: PolicySummary {
                    deny_patterns: manifest.deny.files.len(),
                    ask_patterns: manifest.ask.files.len(),
                    write_patterns: manifest.write.files.len(),
                    delete_patterns: manifest.delete.files.len(),
                    read_patterns: manifest.read.files.len(),
                    full_patterns: manifest.full.files.len(),
                    default_mode: format!("{:?}", manifest.project.default),
                },
            }))
        }

        IpcRequest::CheckFileAccess { path, op } => {
            let file_op = match op.as_str() {
                "read" => FileOp::Read,
                "write" => FileOp::Write,
                "delete" => FileOp::Delete,
                other => {
                    return Err(agentguard_core::GuardError::IpcError(format!(
                        "Invalid op: '{other}'. Use: read, write, delete"
                    )))
                }
            };

            let decision = state.evaluate_access_dry_run(&path, &file_op);

            Ok(IpcResponse::FileCheck(FileCheckResult {
                path: path.clone(),
                op: op.clone(),
                decision,
                source: "policy".to_string(),
                reason: format!("dry-run evaluation for {op} on {}", path.display()),
            }))
        }

        IpcRequest::Shutdown => {
            tracing::info!("Shutdown requested via CLI");
            state.signal_shutdown();
            Ok(IpcResponse::Ok)
        }

        IpcRequest::AskResponse {
            request_id,
            allowed,
            remember,
        } => {
            tracing::info!("AskResponse id={request_id} allowed={allowed} remember={remember}");
            Ok(IpcResponse::Ok)
        }

        IpcRequest::AddGlobalRule { bucket, pattern } => {
            const MAX_PATTERN_LEN: usize = 1024;
            if pattern.trim().is_empty() {
                return Err(agentguard_core::GuardError::IpcError(
                    "Pattern cannot be empty".into(),
                ));
            }
            if pattern.len() > MAX_PATTERN_LEN {
                return Err(agentguard_core::GuardError::IpcError(format!(
                    "Pattern too long (max {MAX_PATTERN_LEN} chars)"
                )));
            }
            if globset::Glob::new(&pattern).is_err() {
                return Err(agentguard_core::GuardError::IpcError(format!(
                    "Invalid glob pattern: '{pattern}'"
                )));
            }

            let bucket = match bucket.as_str() {
                "deny" => Bucket::Deny,
                "ask" => Bucket::Ask,
                "full" => Bucket::Full,
                "delete" => Bucket::Delete,
                "write" => Bucket::Write,
                "read" => Bucket::Read,
                other => {
                    return Err(agentguard_core::GuardError::IpcError(format!(
                        "Invalid bucket: '{other}'. Use: deny, ask, full, delete, write, read"
                    )))
                }
            };
            let id = state.add_global_rule(bucket, &pattern)?;
            tracing::info!("Global rule added: id={id} [{bucket}] {pattern}");
            Ok(IpcResponse::Ok)
        }

        IpcRequest::RemoveGlobalRule { id } => {
            let before = state.store.list_global_rules()?.len();
            state.remove_global_rule(id)?;
            let after = state.store.list_global_rules()?.len();
            if before == after {
                return Err(agentguard_core::GuardError::IpcError(format!(
                    "Global rule {id} not found"
                )));
            }
            tracing::info!("Global rule removed: id={id}");
            Ok(IpcResponse::Ok)
        }

        IpcRequest::EnableProtection { path } => {
            state.enable_protection(&path)?;
            Ok(IpcResponse::Ok)
        }

        IpcRequest::DisableProtection { path } => {
            state.disable_protection(&path)?;
            Ok(IpcResponse::Ok)
        }

        IpcRequest::ListGlobalRules => {
            let rules: Vec<GlobalRuleInfo> = state
                .store
                .list_global_rules()?
                .into_iter()
                .map(|r| GlobalRuleInfo {
                    id: r.id.unwrap_or(0),
                    bucket: r.bucket.as_str().to_string(),
                    pattern: r.pattern,
                    created_at: r.created.format("%Y-%m-%d %H:%M").to_string(),
                })
                .collect();
            Ok(IpcResponse::GlobalRulesList(GlobalRulesListData { rules }))
        }
    }
}

fn load_counts(path: &std::path::Path) -> (usize, usize, usize, usize) {
    let toml_path = path.join("agentguard.toml");
    if let Ok(m) = ProjectManifest::from_file(&toml_path) {
        (
            m.deny.files.len(),
            m.ask.files.len(),
            m.write.files.len(),
            m.read.files.len(),
        )
    } else {
        (0, 0, 0, 0)
    }
}

fn empty_summary() -> PolicySummary {
    PolicySummary {
        deny_patterns: 0,
        ask_patterns: 0,
        write_patterns: 0,
        delete_patterns: 0,
        read_patterns: 0,
        full_patterns: 0,
        default_mode: "conservative".to_string(),
    }
}
