<div align="center">

![Stars](https://img.shields.io/github/stars/TheUser99-spec/Phylax?style=for-the-badge&color=f2c94c)
![Version](https://img.shields.io/github/v/release/TheUser99-spec/Phylax?style=for-the-badge&color=6cdda3)
![License](https://img.shields.io/badge/license-Apache%202.0-blue?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Windows-0078D6?style=for-the-badge&logo=windows)

</div>

<br>

## ⭐ Phylax — OS-level protection for AI coding agents

**Stops agents from reading secrets, deleting files, or touching anything outside your source code.**

- Blocks reads to `.env`, keys, secrets
- Blocks deletes to `migrations/`, config, infra
- Works with Claude Code, Cursor, Windsurf, Aider, OpenCode, Copilot
- Enforced by real Windows ACLs (kernel-level)
- Invisible daemon + live terminal dashboard (60fps)
- 100% local — no accounts, no cloud, no telemetry

<p align="center">
  <img src="assets/demo.gif" alt="Phylax Demo" width="720">
</p>

---

## What is Phylax

**Phylax is a safety boundary for AI coding agents.** It ensures agents can edit your source code — but never touch your secrets, configs, or system files.

Under the hood, it applies real Windows ACLs so the OS kernel itself returns `ACCESS_DENIED` before the agent ever touches a protected file. Claude Code, Cursor, OpenCode, Copilot, Windsurf, Aider — it doesn't matter which agent. If the kernel says no, the agent gets nothing.

---

## Why it exists

AI agents have unrestricted filesystem access. They can read your secrets, delete your migrations, or wipe your config files — without asking, without warning.

**Real examples from the wild:**

```
Claude tried to delete migrations/ → BLOCKED
Cursor tried to read .env          → BLOCKED
OpenCode tried to modify secrets/  → BLOCKED
```

Thousands of open issues across Claude Code, Cursor, Copilot, and others document agents silently destroying data. Not because they're malicious — because they don't understand context, value, or consequence.

Phylax draws a boundary. The agent can edit your source code. It can never touch your `.env`, your SSH keys, or your policy files.

---

## ⚡ Try it in 10 seconds

```powershell
irm https://raw.githubusercontent.com/TheUser99-spec/Phylax/main/install.ps1 | iex
phylax init
phylax run
```

Done. Your project is protected.

---

## Who is this for?

- **Vibe coders** using Claude, Cursor, Windsurf, or any AI coding tool
- **Developers** working with agents that hallucinate file operations
- **Anyone** with `.env`, API keys, configs, or infrastructure files
- **Teams** who want agent productivity without agent risk
- **People who've already lost data** to an AI agent and never want it to happen again

---

## Why Phylax is different

| Not this | This |
|---|---|
| Not a linter | **Kernel-level enforcement** |
| Not a sandbox | **Real Windows ACLs + MIC labels** |
| Not a plugin | **Works with all agents, no integration needed** |
| Not a prompt rule | **The OS blocks the I/O — the agent can't override it** |
| No cloud dependency | **100% local, zero telemetry** |

---

## How it works

1. **Detect** — Phylax identifies AI agent processes by name, environment variables, and command-line inspection
2. **Classify** — Every file I/O is checked against your `phylax.toml` rules
3. **Enforce** — Matched files get DENY ACEs + Mandatory Integrity Control labels. The Windows kernel blocks access at ring 3
4. **Audit** — Every blocked attempt is logged in local SQLite

---

## 🛡️ Anti-bypass (3 layers of protection)

Even if an agent tries to modify ACLs or take ownership, Phylax blocks it at the OS level.

| Layer | Mechanism | Blocks |
|---|---|---|
| 1 | DENY ACE → Everyone → GENERIC_ALL | Read, write, delete |
| 2 | DENY ACE → Everyone → WRITE_DAC, WRITE_OWNER, DELETE | ACL modification, ownership change |
| 3 | MIC label → High Integrity + NO_WRITE_UP | `icacls /remove:d` and privilege bypass |

Layer 3 is the kill shot: even if an agent runs `icacls /remove:d` to strip the DENY ACE, it fails because the agent runs at Medium integrity while the file is labeled High integrity with NO_WRITE_UP. The kernel rejects the write regardless of ownership.

---

## Permission model

Six buckets ordered by priority. **Deny always wins.**

| Priority | Bucket | Meaning |
|---|---|---|
| 1 | `[deny]` | Complete block |
| 2 | `[ask]` | User must approve |
| 3 | `[full]` | Unrestricted |
| 4 | `[delete]` | Read + Delete |
| 5 | `[write]` | Read + Write |
| 6 | `[read]` | Read only |

When no rule matches: read allowed, write asks, delete denied.

[Full permission model docs →](https://phylax.pages.dev/docs#permission-model)

---

## phylax.toml

```toml
[project]
name = "my-project"
default = "conservative"

[deny]
files = [".env", ".env.*", "secrets/**", "*.pem", "*.key"]

[ask]
files = ["Cargo.lock", "migrations/**"]

[write]
files = ["src/**", "tests/**"]

[read]
files = ["README.md", "docs/**"]
```

---

## Commands

| Command | What it does |
|---|---|
| `phylax init` | Create phylax.toml, start daemon, register project |
| `phylax run` | Start daemon + open dashboard (60fps) |
| `phylax stop` | Stop daemon (releases file locks) |
| `phylax status` | Live status: projects, agents, events, blocks |
| `phylax project validate` | Validate phylax.toml syntax |
| `phylax project check -f <f> -o <op>` | Dry-run file access check |
| `phylax project verify` | Audit protection coverage |
| `phylax global add deny "*.env"` | Add global deny rule |
| `phylax audit list` | View audit history |
| `phylax update` | Auto-update from GitHub |

---

## Build from source

```bash
git clone https://github.com/TheUser99-spec/Phylax.git
cd Phylax
cargo build --workspace --release
```

---

## Roadmap

- [x] Process detection & AI agent classification
- [x] phylax.toml parser with glob-based policy engine
- [x] Windows ACL/ACE enforcement
- [x] Three-layer anti-bypass (DENY ACEs + MIC labels)
- [x] SQLite audit log
- [x] IPC protocol (20 request types)
- [x] Terminal dashboard (ratatui, 60fps)
- [x] Unified CLI
- [x] Invisible daemon
- [ ] Kernel minifilter driver (Phase 2)
- [ ] Agent-only blocking (no need to stop daemon)
- [ ] Cross-platform (macOS/Linux)

---

## Docs

| Doc | Topic |
|---|---|
| [Quickstart](docs/quickstart.md) | Complete guide |
| [Architecture](docs/01-architecture.md) | System design |
| [Core types](docs/02-core-types.md) | Permission model |
| [Manifest & policy](docs/03-manifest-policy.md) | phylax.toml |
| [Storage & audit](docs/04-storage-audit.md) | SQLite schema |
| [Detection](docs/05-detection-enforcement.md) | Process classification |
| [IPC & daemon/CLI](docs/06-ipc-daemon-cli.md) | Protocol + lifecycle |
| [ADR index](docs/adr/README.md) | Architecture decisions |
| [Landing page](https://phylax.pages.dev) | Full product site |

---

## License

Phylax is open-source under the **Apache 2.0 License**. See [LICENSE](LICENSE).

Comes with **no warranty**. See [DISCLAIMER.md](DISCLAIMER.md).

---

<br>

<div align="center">

**If Phylax saved your `.env` today, you know what to do →**

[![Stars](https://img.shields.io/github/stars/TheUser99-spec/Phylax?style=social)](https://github.com/TheUser99-spec/Phylax)

<sub>Built with Rust — Windows-first, agent-proof.</sub>

</div>
