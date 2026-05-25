# 04 — Storage & Audit (`agentguard-store` + `agentguard-audit`)

## Database

SQLite 3 via `rusqlite` (bundled). Single file: `%APPDATA%\AgentGuard\agentguard.db`
(Windows) or `~/.local/share/agentguard/agentguard.db` (Unix).

### PRAGMA Configuration

```sql
PRAGMA busy_timeout   = 5000;    -- Wait 5s if DB is locked
PRAGMA journal_mode   = WAL;     -- Write-Ahead Logging (concurrent reads)
PRAGMA synchronous    = NORMAL;  -- Balance safety and performance
PRAGMA foreign_keys   = ON;      -- Referential integrity
PRAGMA cache_size     = -8000;   -- 8 MB memory buffer
```

### Thread Safety

`Store` wraps `Arc<Mutex<Connection>>`. All public methods acquire the mutex
via `self.lock()`. The mutex is **not reentrant** — methods that need to call
other Store methods while holding the lock must use internal helpers that take
`&Connection` directly.

### Schema (Migration v2)

```sql
-- Migration tracking
CREATE TABLE schema_version (
    version    INTEGER NOT NULL,
    applied_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- System-wide rules (applied to all projects)
CREATE TABLE global_rules (
    id      INTEGER PRIMARY KEY,
    bucket  TEXT NOT NULL CHECK(bucket IN ('deny','ask','full','delete','write','read')),
    pattern TEXT NOT NULL,
    created INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Registered project workspaces
CREATE TABLE watched_projects (
    id             INTEGER PRIMARY KEY,
    root           TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    registered_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    active         INTEGER NOT NULL DEFAULT 1
);

-- Every access decision attempt
CREATE TABLE audit_events (
    id          INTEGER PRIMARY KEY,
    agent_pid   INTEGER NOT NULL,
    agent_label TEXT NOT NULL,
    file_path   TEXT NOT NULL,
    operation   TEXT NOT NULL,
    decision    TEXT NOT NULL,
    source      TEXT NOT NULL,
    ts          INTEGER NOT NULL
);
CREATE INDEX idx_audit_ts ON audit_events(ts DESC);

-- Agent process sessions
CREATE TABLE agent_sessions (
    id         INTEGER PRIMARY KEY,
    pid        INTEGER NOT NULL,
    image_name TEXT NOT NULL,
    label      TEXT NOT NULL,
    workspace  TEXT,
    started_at INTEGER NOT NULL,
    ended_at   INTEGER
);

-- Key-value settings
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- User responses to [ask] prompts
CREATE TABLE ask_decisions (
    id         INTEGER PRIMARY KEY,
    request_id INTEGER NOT NULL,
    pid        INTEGER NOT NULL,
    file_path  TEXT NOT NULL,
    operation  TEXT NOT NULL,
    decision   TEXT NOT NULL,
    created    INTEGER NOT NULL DEFAULT (unixepoch())
);
```

### Migrations

Versioned, transactional. Applied in order on first run:

| Version | Description |
|---------|-------------|
| v1 | Create all base tables (global_rules, watched_projects, audit_events, agent_sessions, settings) |
| v2 | Add `ask_decisions` table |

The `run()` function checks `schema_version`, skips already-applied migrations,
and applies new ones inside SQLite transactions with rollback on failure.

## Store API

### Global Rules

```rust
store.insert_global_rule(Bucket::Deny, "C:\\Users\\*\\.ssh\\**") -> i64
store.delete_global_rule(id: i64) -> ()
store.list_global_rules() -> Vec<GlobalRule>
```

### Watched Projects

```rust
store.register_project(root: &Path, name: &str) -> i64
store.unregister_project(root: &Path) -> ()
store.active_projects() -> Vec<WatchedProject>
```

### Registered Projects (daemon)

```rust
store.list_projects() -> Vec<RegisteredProject> {
    path:      PathBuf,
    name:      String,
    added_at:  i64,
    toml_hash: String,      // From settings table
}

store.set_project_hash(root: &Path, hash: &str) -> ()
store.count_events_today() -> (total: u64, blocks: u64)
```

### Audit Events

```rust
store.insert_audit_event(event: &AuditEvent) -> i64
store.recent_audit_events(limit: usize) -> Vec<AuditEvent>
store.rotate_audit_events(max_rows: usize) -> u64  // Keeps last N rows
```

### Agent Sessions

```rust
store.start_session(session: &AgentSession) -> i64
store.end_session(pid: u32) -> ()
store.active_sessions() -> Vec<AgentSession>
```

### Settings

```rust
store.get_setting(key: &str) -> Option<String>
store.set_setting(key: &str, value: &str) -> ()
store.tier() -> String  // Returns "free" if not set
```

## Auditor (`agentguard-audit`)

Thin wrapper around the Store for decision logging. Fail-closed: if the DB is
unavailable, the error propagates and the caller should default to Deny.

```rust
let auditor = Auditor::new(store);

auditor.log_decision(
    agent_pid,      // Process ID
    agent_label,    // Definite/Probable/Inherited
    &file_path,     // Path to the file
    FileOp::Read,   // Attempted operation
    &PolicyDecision::Deny,  // Decision
    PolicySource::Project,  // Source layer
)?;
```

`Auditor` is instantiated in `DaemonState` and called from:
- `evaluate_inner()` — when a non-Allow decision is made
- `on_agent_detected()` — when protections are applied
