# AgentGuard

**OS-level file safety for AI coding agents.** AgentGuard constrains what AI agents can read, write, or delete on your filesystem using explicit security policies.

```
agentguard init          # One command to protect any project
agentguard run           # Start daemon + dashboard together
agentguard status        # Live status of all protected workspaces
```

---

## Quick install (Windows)

```powershell
irm https://raw.githubusercontent.com/TheUser99-spec/AgentGuard/main/install.ps1 | iex
```

Then restart your terminal and run:

```powershell
agentguard init      # Creates agentguard.toml + registers your project
agentguard run       # Opens the dashboard
```

---

## Commands

| Command | Description |
|---|---|
| `agentguard init` | Create `agentguard.toml`, start daemon, register project |
| `agentguard run` | Start daemon + open TUI dashboard |
| `agentguard ui` | Open TUI (daemon must be running) |
| `agentguard daemon start/stop/restart` | Daemon lifecycle |
| `agentguard status` | Show running status, projects, agents |
| `agentguard project validate/check/unregister/show` | Project operations |
| `agentguard project verify` | Audit effective protection coverage |
| `agentguard global add/remove/list` | Global rules (apply to all projects) |
| `agentguard agent add/remove/list` | Per-agent rules (cursor.exe, claude.exe, etc.) |
| `agentguard audit list` | View audit history |
| `agentguard update` | Auto-update from GitHub |
| `agentguard update --check` | Check for updates (no install) |

---

## How it works

AgentGuard monitors your workspace in real time:

1. **Detects** AI agent processes (Cursor, Claude, OpenCode, Copilot, etc.)
2. **Evaluates** every file access against your policy
3. **Enforces** decisions at the OS level (ACL/ACE-based for now)
4. **Audits** everything — full history in SQLite

### Permission model

```
deny > ask > full > delete > write > read
```

- `deny` — Agent can never touch these files
- `ask` — Agent must request permission (you approve/deny)
- `full` — Read, write, and delete allowed
- `delete` — Read and delete, no write
- `write` — Read and write, no delete
- `read` — Read-only

Default when no rule matches: `conservative` (read=Allow, write=Ask, delete=Deny).

---

## agentguard.toml

```toml
[project]
name = "my-project"
default = "conservative"

[deny]
files = [".env", ".env.*", "secrets/**"]

[ask]
files = ["Cargo.lock"]

[write]
files = ["src/**"]

[read]
files = ["README.md"]
```

---

## Architecture

```
Probe/Poller → Classifier → Orchestrator → Policy + Enforce + Audit + Store
                                     |
                                     +→ IPC server (named pipe) → CLI / TUI
```

### Crates

| Crate | Role |
|---|---|
| `agentguard-core` | Base types and shared errors |
| `agentguard-manifest` | `agentguard.toml` parser + glob compiler |
| `agentguard-policy` | Decision engine |
| `agentguard-store` | SQLite persistence |
| `agentguard-probe` | Process polling + AI agent classification |
| `agentguard-enforce` | ACL/ACE enforcement on Windows |
| `agentguard-ipc` | Named-pipe protocol (client + server) |
| `agentguard-notify` | User prompts and notifications |
| `agentguard-audit` | Audit logging |
| `agentguard-daemon` | Main orchestrator / service |
| `agentguard-cli` | CLI entrypoint (all commands) |
| `agentguard-tui` | Terminal dashboard (ratatui) |
| `agentguard-mascot` | Optional terminal mascot |

---

## Build from source

```bash
cargo build --workspace --release

# Binaries in target/release/
#   agentguard.exe          # CLI + TUI
#   agentguard-daemon.exe   # Background daemon
```

---

## Docs

- [Architecture](docs/01-architecture.md)
- [Core types](docs/02-core-types.md)
- [Manifest & policy](docs/03-manifest-policy.md)
- [Storage & audit](docs/04-storage-audit.md)
- [Detection & enforcement](docs/05-detection-enforcement.md)
- [IPC, daemon & CLI](docs/06-ipc-daemon-cli.md)
- [ADR index](docs/adr/README.md)

---

## License

MIT
