# Contributing to Phylax

Thank you for your interest in contributing to Phylax! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please read our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) first.

## Ways to Contribute

- **Report bugs** — Found an issue? File a bug report with reproduction steps.
- **Suggest features** — Have an idea? Open a discussion or feature request.
- **Improve documentation** — Help clarify docs, examples, or architecture.
- **Submit code** — Fix bugs, add features, or improve performance.
- **Test on Windows** — Help test across different Windows versions and configurations.

## Getting Started

### 1. Fork & Clone
```bash
git clone https://github.com/YOUR-USERNAME/Phylax.git
cd Phylax
```

### 2. Build from Source
```bash
cargo build --workspace --release
```

### 3. Run Tests
```bash
cargo test --workspace
```

### 4. Create a Branch
```bash
git checkout -b feature/your-feature-name
```

## Development Guidelines

### Code Style
- Follow Rust conventions (use `cargo fmt` and `cargo clippy`).
- Keep commits atomic and descriptive.
- Reference issues in commit messages: `Fixes #123`.

### Architecture Rules
- See [AGENTS.md](../AGENTS.md) for workspace structure and constraints.
- Preserve dependency direction (core → manifest → policy → enforce/audit/etc → daemon).
- Never modify `driver/` without explicit approval.
- Test before claiming behavior.

### Testing
- Write tests for new features.
- Run `cargo test -p <crate>` for focused testing.
- Ensure all tests pass before submitting a PR.

### Documentation
- Update [docs/](../docs/) if your change affects architecture or behavior.
- Add ADRs for non-obvious architectural decisions in [docs/adr/](../docs/adr/).

## Submitting a Pull Request

1. **Keep it focused** — One feature or bug fix per PR.
2. **Write a clear title** — E.g., "Add per-agent policy overrides" or "Fix ACL race condition".
3. **Describe what changed and why** — Use the PR template.
4. **Link related issues** — Use `Fixes #123` or `Relates to #456`.
5. **Ensure tests pass** — `cargo test --workspace` must be green.

## Reporting Bugs

Use the bug report template and include:
- Windows version and build number
- Reproduction steps
- Expected vs. actual behavior
- Relevant logs from `%APPDATA%\Phylax\phylax.db` (SQLite audit log)

## Questions?

- Check [docs/quickstart.md](../docs/quickstart.md)
- Read [docs/01-architecture.md](../docs/01-architecture.md)
- Open a Discussion

Thank you for making Phylax better! 🛡️
