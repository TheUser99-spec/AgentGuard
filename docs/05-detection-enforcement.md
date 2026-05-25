# 05 — Detection & Enforcement (`agentguard-probe` + `agentguard-enforce`)

## Agent Detection (`agentguard-probe`)

### SubjectClassifier — 5 Signals

The classifier takes a `ProcessInfo` and returns an `AgentLabel`. Signals are
evaluated in order (S1 first, S5 last). First match wins.

| Signal | Description | Result |
|--------|-------------|--------|
| **S1** | Known env vars present (`CLAUDE_CODE`, `ANTHROPIC_API_KEY`, `CURSOR_SESSION`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, etc.) | `Definite` |
| **S2** | Image name in known list (`claude.exe`, `cursor.exe`, `opencode.exe`, `aider.exe`, `goose.exe`, `cline`, `gh.exe`, `gemini.exe`) — except `node.exe` which needs S3 | `Definite` |
| **S3** | `node.exe` with cmdline containing agent keywords (`claude`, `cursor`, `cline`, `aider`, `goose`, `copilot`, `opencode`, `gemini-cli`) | `Definite` |
| **S4** | Session 0 + no window station (non-interactive service/process) | `Probable` |
| **S5** | Parent process is already classified as an agent | `Inherited` |
| default | None of the above | `Human` |

### ProcessInfo

```rust
pub struct ProcessInfo {
    pub pid:          u32,
    pub image_name:   String,   // Executable filename (lowercase)
    pub cmdline:      String,   // Full command line
    pub env_vars:     Vec<String>, // Environment variable keys
    pub session_id:   u32,      // 0 = non-interactive session
    pub has_window:   bool,     // Has interactive window station
    pub parent_pid:   Option<u32>,
}
```

### ClassifierConfig

Configurable list of known agent images and env var names. `Default` provides
a comprehensive baseline covering Claude, Cursor, OpenCode, Aider, Goose,
Cline, GitHub Copilot CLI, Gemini CLI, and generic agent environment variables.

```rust
let classifier = SubjectClassifier::with_defaults();
// Or custom:
let classifier = SubjectClassifier::new(ClassifierConfig {
    known_agent_images: my_images,
    agent_env_vars: my_env_vars,
});
```

### AgentSessionTracker

Thread-safe (`Arc<RwLock<HashMap<u32, TrackedProcess>>>`) tracker for agent
process lifecycles. Cloned via `Arc`, shared between daemon components.

```rust
let tracker = AgentSessionTracker::new(classifier);

// Called when a new process is detected:
let label = tracker.on_process_start(&info, workspace)?;

// Called when a process exits:
let session = tracker.on_process_exit(pid);

// Query:
let label = tracker.get_label(pid);           // -> Option<AgentLabel>
let sessions = tracker.active_sessions();     // -> Vec<AgentSession>
let count = tracker.active_count();           // -> usize
```

Inheritance (S5): If a process is classified as `Human` and its parent is a
tracked agent, the label is upgraded to `Inherited`.

## OS Enforcement (`agentguard-enforce`)

### Enforcer

Per-workspace coordinator. Called by the daemon when applying or releasing
protections.

```rust
let enforcer = Enforcer::new(workspace_root);

// Apply protections (called on agent detection or project registration):
enforcer.apply_project_protections(&manifest)?;

// Release protections (called on agent exit or project unregister):
enforcer.release_project_protections(&manifest)?;
```

`workspace_root` is canonicalized on construction to ensure consistent path
matching with the manifest.

### collect_paths_for_bucket()

Walks the workspace directory tree with `walkdir` (max depth 10, no symlink
following). For each file, calls `manifest.bucket_for_path(path)` to determine
the winning bucket. Collects files matching the target bucket.

```rust
let deny_files = enforcer.collect_paths_for_bucket(&manifest, Bucket::Deny);
// -> HashSet<PathBuf>
```

Only deny-bucket files receive OS-level ACEs in Phase 1. Ask is evaluation-only.

### Multi-Layer ACE Application

When `apply_deny_ace(path)` is called, three layers are applied:

**Layer 1 — Content Denial**
```
DENY_ACCESS for Everyone → GENERIC_ALL
```
Blocks read, write, and delete for all users.

**Layer 2 — Metadata Denial**
```
DENY_ACCESS for Everyone → WRITE_DAC | WRITE_OWNER | DELETE
```
Prevents changing the ACL, changing the owner, or deleting the file.

**Layer 3 — Mandatory Integrity Control**
```
SYSTEM_MANDATORY_LABEL_ACE
  SID: S-1-16-12288 (High Integrity)
  Policy: SYSTEM_MANDATORY_LABEL_NO_WRITE_UP (0x01)
```
Prevents any process at Medium or lower integrity from writing to the file.
Since `WRITE_DAC` is a write operation, this blocks `icacls /remove:d` bypass
even if the calling process owns the file.

The MIC label is stored in the SACL and evaluated by the kernel **before** the
DACL. This means a process running at Medium integrity (all user processes,
including AI agents) cannot remove or modify the protection.

### Protection Rollback

If `SetNamedSecurityInfoW` fails after applying the MIC label, the MIC label
is removed. If data persistence fails (DB write), ACEs are released:

```rust
if let Err(e) = self.store.register_project(&workspace, &name) {
    let _ = enforcer.release_project_protections(&compiled);
    return Err(e);
}
```

### ProtectionHealth

Post-apply verification returns a structured health check:

```rust
pub struct ProtectionHealth {
    pub exists:              bool,
    pub content_deny:        bool,  // Layer 1
    pub metadata_deny:       bool,  // Layer 2
    pub mic_high_no_write_up: bool, // Layer 3
}

impl ProtectionHealth {
    pub fn healthy(&self) -> bool {
        self.exists && self.content_deny && self.metadata_deny && self.mic_high_no_write_up
    }
}
```

If any layer is missing after application, `EnforcementFailed` is returned.

### Dev/Linux Stubs

On non-Windows platforms, ACE operations use `.agentguard-deny-*` marker files
for development and testing. These are plain text files with no actual OS
enforcement.

## Limitations (Phase 1)

| Limitation | Mitigation |
|------------|-----------|
| No real-time agent detection — relies on registration-time ACEs | `project on`/`project off` manual toggle |
| ACEs apply to Everyone, blocking the user too | `project off` to work, `project on` when agent runs |
| No cmdline for node.exe agents without ETW | S1 (env vars) and S2 (image name) catch most known agents |
| max_depth=10 in walkdir | Covers all standard workspace structures |
| Agent can `chdir` to bypass project workspace | Global rules for absolute paths outside any project |

## Dynamic Agent Detection — Phase 1.5 (`agentguard-probe::poller`)

### ProcessPoller

The poller uses Windows `ToolHelp32` snapshots to detect process births and deaths
without requiring admin privileges or ETW configuration.

```rust
let poller = ProcessPoller::new(classifier, tracker);
let (tx, rx) = mpsc::channel(64);
let (stop_tx, stop_rx) = mpsc::channel(1);

// Runs in a blocking tokio task
poller.run(tx, stop_rx, 750).await;
```

### Detection Loop

1. **Snapshot**: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` — captures all running processes
2. **Iterate**: `Process32FirstW` / `Process32NextW` — walks the snapshot
3. **Diff**: Compare current snapshot with previous → detect `Started` and `Exited` events
4. **Classify**: For each `Started`, call `SubjectClassifier::classify()` with available signals
5. **Track**: Feed classified process into `AgentSessionTracker` for inheritance (S5)
6. **Validate**: `GetProcessTimes` checks creation time to avoid PID reuse false positives
7. **Sleep**: 750ms between polls (configurable)

### Available Signals in Poller Mode

| Signal | Available | How |
|--------|-----------|-----|
| S1 (env vars) | ❌ | No cheap access to process environment variables via ToolHelp |
| S2 (image name) | ✅ | `PROCESSENTRY32.szExeFile` |
| S3 (node.exe cmdline) | ❌ | No cmdline access without WMI |
| S4 (session type) | ❌ | All processes assumed interactive (session_id=1, has_window=true) |
| S5 (inheritance) | ✅ | `PROCESSENTRY32.th32ParentProcessID` → `AgentSessionTracker` |

### Daemon Integration

```rust
// DaemonState::on_process_event()
match event {
    ProcessEvent::Started(info) => {
        let label = tracker.on_process_start(info, None);
        if label.is_agent() {
            protect_all_projects();  // Apply ACEs to all registered workspaces
        }
    }
    ProcessEvent::Exited(pid) => {
        tracker.on_process_exit(pid);
        if tracker.active_count() == 0 {
            release_all_projects();  // Remove ACEs when no agents remain
        }
    }
}
```

### Behavior

- **Conservative fallback**: If workspace attribution is not possible (no cmdline),
  ALL registered projects are protected when ANY agent is detected.
- **Two-polls-clean**: Protections are released only when the tracker confirms zero
  active agents (prevents flapping on brief process exits).
- **Graceful degradation**: On non-Windows platforms, the poller returns immediately
  without error (development mode).

### Limitations (Phase 1.5)

| Limitation | Detail |
|------------|--------|
| Polling interval (750ms) | Agent could start, read a file, and exit between polls |
| No cmdline | S3 detection not available; node-based agents may not be detected |
| No env vars | S1 detection not available; some known agents may be missed |
| Blocking thread | Poller runs on `spawn_blocking` — one dedicated OS thread |
| All-projects fallback | Unknown workspace → all projects protected (over-protection) |
| PID reuse window | Validated via `GetProcessTimes` but not atomic |
