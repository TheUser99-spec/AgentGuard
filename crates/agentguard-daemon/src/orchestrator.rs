use agentguard_audit::Auditor;
use agentguard_core::{AgentLabel, Bucket, FileOp, GuardResult, PolicyDecision, PolicySource};
use agentguard_manifest::{find_manifest, CompiledManifest, ProjectManifest};
use agentguard_probe::{AgentSessionTracker, SubjectClassifier};
use agentguard_store::Store;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct DaemonState {
    pub store: Arc<Store>,
    pub tracker: Arc<AgentSessionTracker>,
    auditor: Arc<Auditor>,
    projects: Arc<RwLock<HashMap<PathBuf, ProjectEntry>>>,
    global_manifest: Arc<RwLock<Option<CompiledManifest>>>,
    shutdown_tx: Arc<mpsc::Sender<()>>,
}

#[derive(Clone)]
struct ProjectEntry {
    manifest: CompiledManifest,
    #[allow(dead_code)]
    enforcer: agentguard_enforce::Enforcer,
    toml_hash: String,
}

impl DaemonState {
    pub fn new(db_path: &Path, shutdown_tx: mpsc::Sender<()>) -> GuardResult<Self> {
        let store = Arc::new(Store::open(db_path)?);
        let tracker = Arc::new(AgentSessionTracker::new(SubjectClassifier::with_defaults()));
        let auditor = Arc::new(Auditor::new(store.as_ref().clone()));

        let state = Self {
            store,
            tracker,
            auditor,
            projects: Arc::new(RwLock::new(HashMap::new())),
            global_manifest: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(shutdown_tx),
        };

        state.restore_projects()?;
        state.restore_global_rules()?;

        Ok(state)
    }

    // ── Project management ───────────────────────────────────────────────

    pub fn register_project(&self, workspace: PathBuf) -> GuardResult<()> {
        let workspace = normalize_path(workspace);

        let toml_path = find_manifest(&workspace)?;
        let manifest_raw = ProjectManifest::from_file(&toml_path)?;
        let hash = hash_file(&toml_path)?;

        let name = workspace
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let compiled = CompiledManifest::compile(&manifest_raw, workspace.clone())?;
        let enforcer = agentguard_enforce::Enforcer::new(workspace.clone());
        enforcer.apply_project_protections(&compiled)?;

        if let Err(e) = self.store.register_project(&workspace, &name) {
            let _ = enforcer.release_project_protections(&compiled);
            return Err(e);
        }
        if let Err(e) = self.store.set_project_hash(&workspace, &hash) {
            let _ = enforcer.release_project_protections(&compiled);
            return Err(e);
        }

        let mut projects = self.projects.write().unwrap_or_else(|e| e.into_inner());
        projects.insert(
            workspace.clone(),
            ProjectEntry {
                manifest: compiled,
                enforcer,
                toml_hash: hash,
            },
        );

        tracing::info!("Project registered + protected: {}", workspace.display());
        Ok(())
    }

    pub fn unregister_project(&self, workspace: &Path) -> GuardResult<()> {
        let path = normalize_path(workspace.to_path_buf());
        let entry = self
            .projects
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&path)
            .cloned();
        if let Some(entry) = entry {
            entry
                .enforcer
                .release_project_protections(&entry.manifest)?;
        }

        self.store.unregister_project(&path)?;
        let mut projects = self.projects.write().unwrap_or_else(|e| e.into_inner());
        projects.remove(&path);
        tracing::info!("Project unregistered: {}", path.display());
        Ok(())
    }

    pub fn enable_protection(&self, workspace: &Path) -> GuardResult<()> {
        let path = normalize_path(workspace.to_path_buf());
        let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = projects.get(&path) {
            entry.enforcer.apply_project_protections(&entry.manifest)?;
            tracing::info!("Protection enabled: {}", path.display());
        }
        Ok(())
    }

    pub fn disable_protection(&self, workspace: &Path) -> GuardResult<()> {
        let path = normalize_path(workspace.to_path_buf());
        let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = projects.get(&path) {
            entry.enforcer.release_project_protections(&entry.manifest)?;
            tracing::info!("Protection disabled: {}", path.display());
        }
        Ok(())
    }

    pub fn reload_project(&self, workspace: &Path) -> GuardResult<()> {
        let workspace = normalize_path(workspace.to_path_buf());

        let old_entry = {
            let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
            match projects.get(&workspace) {
                Some(entry) => entry.clone(),
                None => {
                    tracing::warn!(
                        "Hot-reload ignored for unregistered project: {}",
                        workspace.display()
                    );
                    return Ok(());
                }
            }
        };

        let toml_path = find_manifest(&workspace)?;
        let new_hash = hash_file(&toml_path)?;

        if old_entry.toml_hash == new_hash {
            return Ok(());
        }

        let manifest_raw = ProjectManifest::from_file(&toml_path)?;
        let compiled = CompiledManifest::compile(&manifest_raw, workspace.clone())?;
        let enforcer = agentguard_enforce::Enforcer::new(workspace.clone());

        old_entry
            .enforcer
            .release_project_protections(&old_entry.manifest)?;
        if let Err(e) = enforcer.apply_project_protections(&compiled) {
            let _ = old_entry
                .enforcer
                .apply_project_protections(&old_entry.manifest);
            return Err(e);
        }

        self.store.set_project_hash(&workspace, &new_hash)?;

        let mut projects = self.projects.write().unwrap_or_else(|e| e.into_inner());
        projects.insert(
            workspace.clone(),
            ProjectEntry {
                manifest: compiled,
                enforcer,
                toml_hash: new_hash,
            },
        );

        tracing::info!("Hot-reload: {}", workspace.display());
        Ok(())
    }

    // ── Global rules ─────────────────────────────────────────────────────

    pub fn add_global_rule(&self, bucket: Bucket, pattern: &str) -> GuardResult<i64> {
        let id = self.store.insert_global_rule(bucket, pattern)?;
        self.rebuild_global_manifest()?;
        Ok(id)
    }

    pub fn remove_global_rule(&self, id: i64) -> GuardResult<()> {
        self.store.delete_global_rule(id)?;
        self.rebuild_global_manifest()?;
        Ok(())
    }

    fn rebuild_global_manifest(&self) -> GuardResult<()> {
        let rules = self.store.list_global_rules()?;
        if rules.is_empty() {
            *self
                .global_manifest
                .write()
                .unwrap_or_else(|e| e.into_inner()) = None;
            return Ok(());
        }

        let mut manifest = ProjectManifest::default();
        for rule in &rules {
            let pattern = expand_global_pattern(&rule.pattern);
            match rule.bucket {
                Bucket::Deny => manifest.deny.files.push(pattern),
                Bucket::Ask => manifest.ask.files.push(pattern),
                Bucket::Full => manifest.full.files.push(pattern),
                Bucket::Delete => manifest.delete.files.push(pattern),
                Bucket::Write => manifest.write.files.push(pattern),
                Bucket::Read => manifest.read.files.push(pattern),
            }
        }

        let compiled = CompiledManifest::compile(&manifest, PathBuf::new())?;
        *self
            .global_manifest
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(compiled);

        tracing::info!("Global manifest rebuilt: {} rules", rules.len());
        Ok(())
    }

    // ── Dynamic agent detection ─────────────────────────────────────────

    pub fn on_process_event(&self, event: &agentguard_probe::ProcessEvent) {
        match event {
            agentguard_probe::ProcessEvent::Started(info) => {
                let label = self.tracker.on_process_start(info, None);
                if label.is_agent() {
                    self.protect_all_projects();
                    let _ = self.auditor.log_decision(
                        info.pid,
                        label,
                        &PathBuf::from(info.image_name.clone()),
                        agentguard_core::FileOp::Read,
                        &agentguard_core::PolicyDecision::Deny,
                        agentguard_core::PolicySource::Project,
                    );
                    tracing::info!(
                        "Agent detected: {} (PID={}) {:?}",
                        info.image_name, info.pid, label
                    );
                }
            }
            agentguard_probe::ProcessEvent::Exited(pid) => {
                if self.tracker.on_process_exit(*pid).is_some() {
                    tracing::info!("Agent exited: PID={pid}");
                    if self.tracker.active_count() == 0 {
                        self.release_all_projects();
                        tracing::info!("All agents gone — protections released");
                    }
                }
            }
        }
    }

    fn protect_all_projects(&self) {
        let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
        for entry in projects.values() {
            if let Err(e) = entry.enforcer.apply_project_protections(&entry.manifest) {
                tracing::error!("Failed to apply protections: {e}");
            }
        }
    }

    fn release_all_projects(&self) {
        let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
        for entry in projects.values() {
            if let Err(e) = entry.enforcer.release_project_protections(&entry.manifest) {
                tracing::error!("Failed to release protections: {e}");
            }
        }
    }

    // ── Agent lifecycle ──────────────────────────────────────────────────

    // Future dynamic-enforcement hook. Phase 1.5 protects on registration.
    #[allow(dead_code)]
    pub fn on_agent_detected(&self, pid: u32, workspace: &Path) -> GuardResult<()> {
        let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = projects.get(workspace) {
            tracing::info!(
                "Agent PID={pid} detected in {}. Applying protections...",
                workspace.display()
            );
            entry.enforcer.apply_project_protections(&entry.manifest)?;

            self.auditor.log_decision(
                pid,
                AgentLabel::Definite,
                workspace,
                FileOp::Read,
                &PolicyDecision::Deny,
                PolicySource::Project,
            )?;
        }
        Ok(())
    }

    // Future dynamic-enforcement hook. Phase 1.5 keeps static project protection.
    #[allow(dead_code)]
    pub fn on_agent_exited(&self, pid: u32) -> GuardResult<()> {
        if let Some(session) = self.tracker.on_process_exit(pid) {
            if let Some(workspace) = &session.workspace {
                let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
                if let Some(entry) = projects.get(workspace) {
                    entry
                        .enforcer
                        .release_project_protections(&entry.manifest)?;
                }
            }
            tracing::info!("Agent PID={pid} exited. Temp protections released.");
        }
        Ok(())
    }

    // ── Access evaluation ────────────────────────────────────────────────

    // Intended for an eventual live probe/runner path; IPC uses dry-run checks today.
    #[allow(dead_code)]
    pub fn evaluate_access(&self, pid: u32, abs_path: &Path, op: &FileOp) -> PolicyDecision {
        let label = match self.tracker.get_label(pid) {
            Some(l) if l.is_agent() => l,
            _ => return PolicyDecision::Allow,
        };

        let path = normalize_path(abs_path.to_path_buf());
        self.evaluate_inner(pid, label, &path, op)
    }

    pub fn evaluate_access_dry_run(&self, abs_path: &Path, op: &FileOp) -> PolicyDecision {
        let path = normalize_path(abs_path.to_path_buf());
        self.evaluate_inner(0, AgentLabel::Definite, &path, op)
    }

    fn evaluate_inner(
        &self,
        pid: u32,
        label: AgentLabel,
        path: &Path,
        op: &FileOp,
    ) -> PolicyDecision {
        // 1. Global rules take precedence
        if let Some(global) = self
            .global_manifest
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let (decision, _source) = global.evaluate(path, op);
            if decision != PolicyDecision::Allow {
                let _ = self.auditor.log_decision(
                    pid,
                    label,
                    path,
                    *op,
                    &decision,
                    PolicySource::Global,
                );
                return decision;
            }
        }

        // 2. Project rules
        let projects = self.projects.read().unwrap_or_else(|e| e.into_inner());
        for (workspace, entry) in projects.iter() {
            if path.starts_with(workspace) || is_in_workspace(path, workspace) {
                let (decision, _source) = entry.manifest.evaluate(path, op);

                if decision != PolicyDecision::Allow {
                    let _ = self.auditor.log_decision(
                        pid,
                        label,
                        path,
                        *op,
                        &decision,
                        PolicySource::Project,
                    );
                    return decision;
                }

                // Path is in project but no explicit rule matched:
                // fall through to project's default_mode
                return entry.manifest.apply_default(path, op);
            }
        }

        // 3. Path not in any project: default conservative
        PolicyDecision::Allow
    }

    // ── Shutdown ─────────────────────────────────────────────────────────

    pub fn signal_shutdown(&self) {
        self.shutdown_tx.try_send(()).ok();
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    pub fn list_projects(&self) -> Vec<PathBuf> {
        self.projects
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }

    fn restore_projects(&self) -> GuardResult<()> {
        let registered = self.store.list_projects()?;
        for p in registered {
            if p.path.exists() {
                if let Err(e) = self.register_project(p.path.clone()) {
                    tracing::warn!("Failed to restore project {:?}: {e}", p.path);
                }
            }
        }
        Ok(())
    }

    fn restore_global_rules(&self) -> GuardResult<()> {
        self.rebuild_global_manifest()
    }
}

fn hash_file(path: &Path) -> GuardResult<String> {
    let bytes =
        std::fs::read(path).map_err(|e| agentguard_core::GuardError::EnforcementFailed {
            path: path.display().to_string(),
            reason: format!("cannot read for hashing: {e}"),
        })?;

    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{h:016x}"))
}

fn normalize_path(path: PathBuf) -> PathBuf {
    match std::fs::canonicalize(&path) {
        Ok(p) => strip_verbatim_prefix(p),
        Err(_) => {
            if path.is_absolute() {
                path
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(&path)
            } else {
                path
            }
        }
    }
}

fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix("\\\\?\\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}

fn expand_global_pattern(pattern: &str) -> String {
    if pattern.contains('\\') || pattern.contains('/') || pattern.contains("**") {
        pattern.to_string()
    } else {
        format!("**/{pattern}")
    }
}

fn is_in_workspace(path: &Path, workspace: &Path) -> bool {
    path.starts_with(workspace)
        || std::fs::canonicalize(path)
            .map(|p| strip_verbatim_prefix(p).starts_with(workspace))
            .unwrap_or(false)
}
