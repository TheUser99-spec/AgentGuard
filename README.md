# 🛡 AgentGuard — README Maestro

> *The invisible layer that tells AI agents what they can and cannot touch.*

---

## 📋 Index

- [[#Vision]]
- [[#The Shared Foundation — Permission Model]]
- [[#Two Surfaces, One Engine]]
- [[#Audiences]]
- [[#Monetization]]
- [[#Architecture]]
- [[#Phases and Deliverables]]
- [[#Development Principles]]
- [[#Current Status]]

---

## Vision

AgentGuard is an **operating-system-level security layer** that controls what AI agents (Claude Code, Cursor, Manus, Copilot, etc.) can do on your machine or project.

It is not an IDE wrapper. It is not a prompt rule. It is real OS-level enforcement: the agent receives `ACCESS_DENIED` from the operating system, not a suggestion from the LLM.

**Problem it solves:**
AI coding agents run autonomously, sometimes overnight, with full filesystem access. No layer exists today that says "you may read the source code but never touch `.env`, credentials, or git history" in a way that cannot be bypassed.

**Positioning:**
> *"Launch agents overnight with total confidence. Your secrets are protected at the operating system level."*

---

## The Shared Foundation — Permission Model

Both the **end-user** experience (machine protection) and the **dev/vibecoder** experience (project protection) share the same permission engine.

### The 6 Buckets

| Bucket | What it allows | Priority |
|--------|----------------|----------|
| `deny` | Nothing. Total block. Agent cannot read, write, or delete. | 1 — always wins |
| `ask` | Prompt the user before access. Timeout = deny. | 2 |
| `full` | Read + write + delete. Full access. | 3 |
| `delete` | Read + delete (no new file creation) | 4 |
| `write` | Read + write (no delete) | 5 |
| `read` | Read only. Never modify or delete. | 6 |

### Conflict Resolution Rule

If a file appears in multiple buckets: **the highest priority wins.**

```
deny > ask > full > delete > write > read > (default)
```

> [!IMPORTANT]
> `deny` always wins, no exceptions. If a file is in `deny` and in `write`, the result is `deny`.

### Default When No Rule Exists

```
read  → ALLOW
write → ASK the user
delete → DENY
```

---

## Two Surfaces, One Engine

The same permission engine is exposed in two different ways depending on who uses it.

---

### Surface 1 — System (end user)

**Storage:** Local SQLite database — `%APPDATA%\AgentGuard\agentguard.db`
**Scope:** The entire machine. Applies to any agent, in any project.
**Management:** The user configures it via the TUI or CLI. Never touches a text file.

**Why SQLite instead of a config file:**
- Global rules change frequently and need atomic writes — no corruption risk
- The audit log (thousands of events) cannot live in a TOML file
- The TUI needs to query events, filter, paginate — it needs queries, not text parsing
- A single `.db` file is the complete system state: easy to export, back up, migrate

**Main tables:**

```sql
-- User's global rules (what would previously be config.toml)
CREATE TABLE global_rules (
    id       INTEGER PRIMARY KEY,
    bucket   TEXT NOT NULL,  -- 'deny' | 'ask' | 'write' | 'read' | 'full' | 'delete'
    path     TEXT NOT NULL,  -- glob pattern, e.g.: C:\Users\*\.ssh\**
    created  INTEGER NOT NULL
);

-- Everything the agent attempted + result
CREATE TABLE audit_events (
    id          INTEGER PRIMARY KEY,
    agent_pid   INTEGER NOT NULL,
    agent_label TEXT NOT NULL,   -- DEFINITE | PROBABLE | INHERITED
    file_path   TEXT NOT NULL,
    operation   TEXT NOT NULL,   -- read | write | delete
    decision    TEXT NOT NULL,   -- allow | deny | ask
    source      TEXT NOT NULL,   -- global | project
    ts          INTEGER NOT NULL
);

-- Detected agent sessions
CREATE TABLE agent_sessions (
    id          INTEGER PRIMARY KEY,
    pid         INTEGER NOT NULL,
    image_name  TEXT NOT NULL,
    label       TEXT NOT NULL,
    workspace   TEXT,
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER
);

-- Preferences, tier, general configuration
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

**End-user use cases:**
- "No agent may ever read my SSH keys" → `INSERT INTO global_rules (bucket='deny', path='C:\Users\*\.ssh\**')`
- "Read-only in `Documents\`" → bucket `read`
- View what the agent touched → `SELECT * FROM audit_events ORDER BY ts DESC`

> [!NOTE]
> The user manages all of this from the TUI or CLI. They never see SQL or config files.

---

### Surface 2 — Project (dev / vibecoder)

**File:** `agentguard.toml` at the project root
**Scope:** Only that workspace. Activates when the agent opens that directory.
**Management:** The dev creates it, versions it in git, shares it with the team.

**Use cases:**
- "The agent can edit `src/` but never touch `Cargo.lock`"
- "It can delete `target/` and `node_modules/` but nothing else"
- "Must ask before modifying any config file"

```toml
# agentguard.toml — project root

[project]
name = "my-app"
default = "conservative"   # conservative | unrestricted

[deny]
files = [
    ".env", ".env.*",
    "*.pem", "*.key",
    ".git/**",
    "secrets/**",
]

[ask]
files = [
    "Cargo.lock",
    "package-lock.json",
    "*.config.js",
]

[write]
files = [
    "src/**/*.rs",
    "src/**/*.ts",
    "Cargo.toml",
    "tests/**",
]

[delete]
files = [
    "target/**",
    "node_modules/**",
    "dist/**",
    "*.log",
]

[read]
files = [
    "docs/**/*.md",
    "README.md",
    ".cursor/rules/**",
]
```

> [!TIP]
> The vibecoder doesn't need to understand security. They see buckets as permission folders. Simple.

---

### Layer Hierarchy

```
┌─────────────────────────────────────────┐
│  Global Layer  (agentguard.db)          │  ← end user
│  Applies to EVERYTHING. Always.         │
├─────────────────────────────────────────┤
│  Project Layer  (agentguard.toml)       │  ← dev / vibecoder
│  Applies only in that workspace.        │
│  Within the limits of the global layer  │
└─────────────────────────────────────────┘

Rule: the global layer ALWAYS wins over the project layer.
If global has ~/.ssh in deny, no project agentguard.toml
can override that.
```

---

## Audiences

### 👤 End User (non-dev)

**Who they are:** A person who uses Claude Code, Cursor, or other agents to program but doesn't want to manage technical configs. They want "install and forget."

**What they need:**
- Simple installer (`.msi` on Windows)
- TUI/GUI with clear state: what is protected, what is not
- Sensible default config without touching anything
- Notifications when the agent tries to access something restricted

**Their `agentguard.toml`:** They don't know about it. They don't need it. The global layer protects them.

---

### 💻 Dev / Vibecoder

**Who they are:** A developer who launches autonomous agents in their projects. They want fine-grained control over what the agent can and cannot do in their codebase.

**What they need:**
- `agentguard.toml` in their project
- CLI: `agentguard project init`, `project check`, `project validate`
- Dry-run: `agentguard check --file src/main.rs --op write`
- Hot-reload when they modify the `.toml`
- Audit log of everything the agent attempted

**Their global config:** Inherits base protections + adds project-specific ones.

---

## Monetization

### Tiers

| Tier | Price | What it unlocks |
|------|-------|-----------------|
| **Free** | €0 | Monitoring, audit log, `agentguard.toml` parsing, CLI |
| **Guardian** | €1.99/mo | Real enforcement (DENY ACEs), `[ask]` notifications, hot-reload |
| **Warden** | €4.99/mo | Kernel minifilter (unbypassable), basic code scanner |
| **Team** | €X/seat | Centralized policies, dashboard, sync, alerts |

### EV Cert Justification (€150/year)

```
Guardian: €1.99 × 100 users = €199/mo = €2,388/year
Warden:   €4.99 × 50  users = €249/mo = €2,988/year

The cert pays for itself with ~7 Guardian users.
```

### Future Monetization

- **Code Intelligence add-on:** Analysis of AI-generated code. 45–60% of AI code has errors or vulnerabilities — AgentGuard detects them. `agentguard analyze ./src`
- **AgentGuard for Teams:** Policy sync between devs. Web dashboard for project events.
- **API:** Third parties who want to integrate the permission engine into their tools.

---

## Architecture

### Stack

| Layer | Technology | Reason |
|-------|-----------|--------|
| Windows Service | Rust | Memory safety, performance, Windows crate ecosystem |
| Process detection | Rust (ToolHelp32 + ferrisetw) | Agent detection via process snapshot polling |
| Policy engine | Rust (globset) | Compiled GlobSet, O(1) matching |
| Kernel driver (Phase 2) | C++ | Mature WDK, windows-drivers-rs not production-ready yet |
| IPC driver↔service | Rust (FltPort bindings) | Same stack |
| Local database | SQLite (rusqlite) | Single file, ACID, serverless, queries for audit log and TUI |
| CLI + TUI | Rust (ratatui) | Consistent cross-platform |

### Crate Workspace

```
crates/
├── agentguard-core/       ← types, errors, traits. Zero external deps
├── agentguard-manifest/   ← agentguard.toml parsing + GlobSet compiler
├── agentguard-probe/      ← Poller ToolHelp32 + SubjectClassifier (agent detection)
├── agentguard-policy/     ← CompiledPolicy. deny→ask→allow evaluation
├── agentguard-enforce/    ← DENY ACEs + MIC labels (Phase 1)
├── agentguard-store/      ← SQLite via rusqlite. global_rules, audit_events, agent_sessions, settings
├── agentguard-ipc/        ← Named pipe daemon↔CLI
├── agentguard-notify/     ← Windows toast notifications for [ask]
├── agentguard-daemon/     ← Windows Service. Orchestrates everything
├── agentguard-cli/        ← CLI: init, status, check, project
├── agentguard-tui/        ← ratatui dashboard
└── modules/
    ├── agentguard-scanner/ ← Phase 3: AI code analysis
    └── agentguard-team/    ← Phase 4: sync, dashboard
driver/
└── agentguard.sys         ← C++ minifilter (Phase 2)
```

### Agent Detection (Windows)

The daemon detects agents by polling process snapshots (ToolHelp32) and accumulating signals:

```
S1: Env vars      → CLAUDE_CODE, CURSOR_SESSION, ANTHROPIC_API_KEY...
S2: Image name    → claude.exe, cursor.exe, goose.exe, aider...
S3: Cmdline (node)→ node.exe with agent keywords in command line
S4: Session type  → Session 0 / no window station = non-interactive
S5: Parent chain  → cursor.exe → node.exe → git.exe → inherited

Accumulated score → AGENT_DEFINITE | AGENT_PROBABLE | AGENT_INHERITED | HUMAN
```

---

## Phases and Deliverables

---

### 🟢 Phase 1 — Foundation + Enforcement (Weeks 1–6)

**Tier activated:** Free + Guardian (€1.99/mo)

**What we want to achieve:**
A real, installable product that effectively blocks Claude Code, Cursor, and Manus on a project's filesystem. The user runs `agentguard init`, the daemon starts, and from that moment the agent cannot touch what it shouldn't. No kernel driver, no special signing, no complications.

**The flow that must work 100% at the end of this phase:**

```
1. User installs AgentGuard (.msi)
2. User goes to their project and runs: agentguard init
3. CLI creates agentguard.toml interactively if it doesn't exist
4. CLI starts the daemon as a Windows Service (if not running)
5. CLI registers the project in agentguard.db
6. Daemon starts ReadDirectoryChangesW on the workspace
7. User launches Claude Code
8. Daemon detects claude.exe via ETW → AGENT_DEFINITE
9. Daemon applies DENY ACEs on [deny] project files
10. Claude Code tries to read .env → ACCESS_DENIED from OS
11. Toast notification: "Claude Code blocked on .env"
12. Daemon writes event to audit_events
13. User edits agentguard.toml → automatic hot-reload in <1s
```

**Crates to build (in dependency order):**

- [ ] `agentguard-core` — Shared types: `AgentLabel`, `FileOp`, `PolicyDecision`, `GuardError`. Zero external dependencies. Foundation for everything.
- [ ] `agentguard-store` — SQLite via `rusqlite`. Tables: `watched_projects`, `global_rules`, `audit_events`, `agent_sessions`, `settings`. Versioned migrations.
- [ ] `agentguard-manifest` — `agentguard.toml` parser. Glob validation. Compilation to `GlobSet` per bucket. `realpath()` before matching.
- [ ] `agentguard-policy` — `CompiledPolicy`. `evaluate_file_op(path, op) → PolicyDecision`. Chain: `deny > ask > full > delete > write > read > default`.
- [ ] `agentguard-probe` — ETW consumer (`ferrisetw`). `SubjectClassifier`: S1 env vars + S2 image name + S3 session type + S4 parent chain → `AgentLabel`.
- [ ] `agentguard-enforce` — DENY ACE applier via `SetNamedSecurityInfo`. ACE cleanup on agent death. Job Objects for containment.
- [ ] `agentguard-ipc` — Bidirectional named pipe daemon↔CLI. Message protocol: `RegisterProject`, `UnregisterProject`, `GetStatus`, `ReloadPolicy`.
- [ ] `agentguard-notify` — Windows toast notifications for `[ask]` bucket. User response → allow once / deny / allow always. Timeout = deny.
- [ ] `agentguard-audit` — Writes `AuditEvent` to `agentguard-store`. Paginated reads for TUI. Automatic size-based rotation.
- [ ] `agentguard-daemon` — Windows Service. Orchestrates: ETW probe + policy engine + enforce + notify + audit. `ReadDirectoryChangesW` for hot-reload. Manages `watched_projects` in DB.
- [ ] `agentguard-cli` — Commands:
  - `agentguard init` → creates `agentguard.toml`, starts daemon if not running, registers project
  - `agentguard status` → shows active agents, registered projects, tier
  - `agentguard project validate` → validates the toml in the current directory
  - `agentguard project check --file <path> --op <read|write|delete>` → dry-run
  - `agentguard project unregister` → removes the project from watch
  - `agentguard daemon start / stop / restart`
- [ ] `agentguard-tui` — ratatui dashboard: real-time active agents, latest audit events, active policy per project, daemon status.

**Tests that must pass before calling it "done":**

- [ ] Unit: `agentguard.toml` parser with all buckets and conflicts
- [ ] Unit: `evaluate_file_op()` with the full priority table
- [ ] Unit: `SubjectClassifier` with mocked ETW events
- [ ] Integration: simulate `claude.exe` trying to read `.env` → verify `ACCESS_DENIED`
- [ ] Integration: edit `agentguard.toml` → verify hot-reload in <1s
- [ ] Integration: `agentguard init` on a clean project → full end-to-end flow

**Required documentation:**

- [ ] ADR-001: ETW vs polling for process detection
- [ ] ADR-002: DENY ACEs vs minifilter for Phase 1
- [ ] ADR-003: SQLite vs config file for global rules
- [ ] ADR-004: Named pipe vs other IPC mechanisms for daemon↔CLI
- [ ] `agentguard.toml` full spec (format, buckets, examples)

**Definition of "done":**
> A vibecoder installs AgentGuard, runs `agentguard init` in their project, launches Claude Code, and without touching anything else the agent receives `ACCESS_DENIED` when trying to read `.env`. The TUI shows the event in real time.

**Tier sold upon completing this phase:** Free (monitoring) + Guardian €1.99 (active enforcement).

---

### 🔵 Phase 2 — Kernel Minifilter (Month 2–3)

**Tier activated:** Warden (€4.99/mo)

**What we want to achieve:**
Make the protection completely unbypassable. In Phase 1, an agent running with Administrator privileges could remove the DENY ACEs. In Phase 2, the driver intercepts file operations at the kernel level before the OS processes them — there is nothing to remove or evade.

Additionally, the `[ask]` mechanism becomes clean: the driver pauses the IRP (the I/O request) synchronously while the user decides. The agent doesn't receive `ACCESS_DENIED` — it simply waits.

**Economic prerequisite:** EV code signing cert (~€150/year). Pays for itself with 8 Guardian users or 4 Warden users.

**What we build:**

- [ ] `driver/agentguard.sys` — C++ minifilter at altitude 320000
  - `FLT_PREOP_CALLBACK` on `IRP_MJ_CREATE` (open/read), `IRP_MJ_WRITE`, `IRP_MJ_SET_INFORMATION` (delete/rename)
  - `PsSetCreateProcessNotifyRoutineEx2` — propagates `AgentLabel` on every fork/spawn via LRU hash
  - Communication with user-mode via `FltCommunicationPort`
- [ ] `agentguard-fltport` — Rust bindings for `FltCommunicationPort`. Protocol: driver asks daemon for `PolicyDecision` by `(pid, path, op)`.
- [ ] Update `agentguard-daemon` — connect with driver via FltPort. Driver handles the hot path, daemon handles business logic.
- [ ] WHCP submission — submit signed driver to Microsoft for official signature.
- [ ] VM tests — agents running as Administrator attempting bypass. Verify blocking.

**Clean `[ask]` in Phase 2:**

```
Agent tries to open Cargo.lock
        ↓
Driver intercepts IRP_MJ_CREATE (FLT_PREOP)
Driver asks daemon: what do I do?
Daemon sees [ask] in policy → pauses the IRP
Daemon launches toast: "Allow access to Cargo.lock?"
        ↓
User responds → daemon notifies driver
Driver decides: FLT_PREOP_SUCCESS or STATUS_ACCESS_DENIED
The agent never knew there was a pause
```

**Definition of "done":**
> An agent running as Administrator cannot access any file in `[deny]` nor remove the protection in any way.

---

### 🟣 Phase 3 — Code Scanner (Month 4–5)

**Tier activated:** Add-on on top of Guardian/Warden, or included in a future higher tier.

**What we want to achieve:**
A second revenue stream independent from protection. AgentGuard already has access to workspace code — that is a massive asset. The scanner analyzes the code the agent writes and detects vulnerabilities, hardcoded secrets, and insecure patterns.

Sales angle: 45–60% of AI-generated code has errors or security issues. AgentGuard catches them before they reach production.

**What we build:**

- [ ] `modules/agentguard-scanner/` — implements `Module` trait
- [ ] `agentguard analyze [path]` — CLI command, scans code in the workspace
- [ ] LLM integration (DeepSeek API or local model) for semantic analysis
- [ ] Static rules: hardcoded secrets, SQL injection, path traversal, vulnerable deps
- [ ] Report: colored terminal + JSON/HTML export
- [ ] Audit integration: when the agent writes a file, the scanner analyzes it automatically in the background

**Definition of "done":**
> `agentguard analyze ./src` returns a vulnerability report with severity, exact line, and fix suggestion in under 30 seconds for a mid-size project.

---

### 🟡 Phase 4 — Team + macOS (Month 6+)

**Tier activated:** Team (€X/seat)

**What we want to achieve:**
Enter B2B. Teams working with agents on shared projects need centralized policies — an `agentguard.toml` that syncs for the entire team, a dashboard to see what each dev's agents are doing, and alerts when something tries to bypass the rules.

macOS in parallel: same permission engine, Endpoint Security framework as enforcement instead of minifilter.

**What we build:**

- [ ] `modules/agentguard-team/` — `agentguard.toml` sync via central server
- [ ] Web dashboard — team audit events, active policy, alerts
- [ ] Team tier — seat management, roles (admin / member)
- [ ] macOS port — adapted `agentguard-daemon`, Endpoint Security framework for enforcement
- [ ] `agentguard-daemon` refactor — abstract the enforcement layer to be cross-platform

**Definition of "done":**
> A team of 5 devs has the same `agentguard.toml` synced. The tech lead sees all team agent access attempts in the dashboard in real time.

---

## Development Principles

> [!IMPORTANT]
> These principles are non-negotiable. When in a hurry, they still hold.

### 1. Document Architectural Decisions (ADR)

Every non-obvious decision → `docs/adr/NNN-title.md`

```
docs/adr/
├── 001-etw-vs-polling.md
├── 002-deny-ace-vs-minifilter-phase1.md
├── 003-agentguard-toml-format.md
└── 004-cpp-driver-vs-rust.md
```

Minimum format for each ADR:
```markdown
# NNN — Title
## Context
## Decision
## Alternatives considered
## Consequences
```

### 2. Tests Before Merge

- Unit tests in each crate
- Integration tests simulating real agent operations
- `cargo test --workspace` green before any commit to main

### 3. Modules = Pluggable, Core = Stable

```rust
trait Module: Send + Sync {
    fn name(&self) -> &str;
    fn on_agent_event(&self, event: &AgentEvent) -> ModuleResult;
}
```

Adding `code-scanner` or `team-sync` **does not touch** `agentguard-core` or `agentguard-policy`. Ever.

### 4. `agentguard.toml` is Backwards-Compatible Forever

If someone created an `agentguard.toml` in v0.1, it works in v1.0. No exceptions.

### 5. Realpath Before Glob Match

```rust
let canonical = std::fs::canonicalize(&requested_path)?;
let relative  = canonical.strip_prefix(&workspace_root)?;
// Now match against GlobSets
```

Protects against symlink bypass (CVE-2025-59829 in Claude Code).

---

## Current Status

```
🟢 Phase 1 — COMPLETED (11/12 crates, 100 tests)
    [x] agentguard-core        — Tipos base + errors (5 tests)
    [x] agentguard-store       — SQLite + migraciones v2 + list_projects + count_events_today (6 tests)
    [x] agentguard-manifest    — Parser TOML + GlobSets + discovery (12 tests)
    [x] agentguard-policy      — CompiledPolicy global > project (3 tests)
    [x] agentguard-probe       — SubjectClassifier 5 señales S1-S5 + AgentSessionTracker + ProcessPoller (10 tests)
    [x] agentguard-enforce     — DENY ACEs 3-capas via SetNamedSecurityInfoW + Enforcer walkdir (8 tests)
    [x] agentguard-ipc         — Named pipe bidireccional + codec + client/server + integración (28 tests)
    [x] agentguard-notify      — Windows MessageBoxW + Unix terminal prompt (6 tests: 5 Unix + 1 Windows)
    [x] agentguard-audit       — Auditor conectado al daemon, fail-closed (0 tests propios)
    [x] agentguard-daemon      — Orquestador + handler IPC + watcher ReadDirectoryChangesW + poller ToolHelp32 (0 tests propios)
    [x] agentguard-cli         — 14 comandos: init / status / project validate|check|show|off|on|unregister / global add|remove|list / daemon start|stop|restart / audit list (22 tests)
    [ ] agentguard-tui         — Dashboard ratatui — deferred to post-Phase 1
    [x] Unit + integration tests — 100 tests, 0 fallos, clippy limpio, cargo fmt OK
    [ ] ADRs (001–004)
    [ ] .msi installer
    [ ] Release v0.1.0 — Free + Guardian €1.99

🔵 Phase 1.5 — Dynamic Agent Detection ✅
    [x] ProcessPoller ToolHelp32 — snapshot cada 750ms
    [x] Detección de procesos Started/Exited
    [x] Classifier S2 + S5 (image name + inheritance via parent PID)
    [x] Validación PID reuse con GetProcessTimes
    [x] Protección automática al detectar agente
    [x] Liberación automática al desaparecer último agente
    [x] Daemon: "Dynamic agent detection: ACTIVE (750ms polling)"

🔵 Phase 2 — PENDING · Requires EV cert €150/year
    [ ] agentguard.sys (C++ minifilter)
    [ ] agentguard-fltport
    [ ] WHCP submission
    [ ] Warden €4.99 release

🟣 Phase 3 — PENDING
    [ ] agentguard-scanner module
    [ ] Code Intelligence add-on

🟡 Phase 4 — PENDING
    [ ] agentguard-team module
    [ ] macOS port
    [ ] Team tier
```

---

## Links and References

- [[AgentGuard - Business Strategy]]
- [[AgentGuard - Dev Guide]]
- [[AgentGuard - agentguard.toml Spec]]
- [[AgentGuard - Windows Architecture]]
- [[AgentGuard - Monetization]]

---

*Last updated: May 2026*
*Stack: Rust + C++ (driver) · Windows-first · macOS in Phase 4*
*Phase 1: 11/12 crates implemented · 100 tests · 0 clippy warnings*