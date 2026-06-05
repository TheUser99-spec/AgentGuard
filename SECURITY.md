# 🛡️ Security Policy

## Supported Versions

| Version | Phase | Supported |
|---|---|---|
| 0.1.x | Phase 1 (ACL Enforcement) | ✅ Active |
| 0.2.x | Phase 2 (Kernel Driver) | 🔜 In Development |

## Reporting a Vulnerability

**If you discover a security vulnerability in Phylax, please report it responsibly.**

### Do NOT:
- Open a public GitHub issue
- Post about it on social media
- Share exploit details publicly

### Do:
Email the details to the maintainers. We will respond within 48 hours with:
- Confirmation of receipt
- An initial assessment
- A timeline for the fix

### What to include:
- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Potential impact
- Any suggested fixes (optional)

## Security Model

Phylax applies three independent layers of Windows security:

1. **DENY ACEs** — Block read/write/delete at the ACL level
2. **WRITE_DAC protection** — Prevent ACL modification and ownership changes
3. **MIC Labels** — Mandatory Integrity Control with NO_WRITE_UP at High Integrity

### Known Limitations (Phase 1)

- Protection is active while the daemon runs. `phylax stop` removes DENY ACEs.
- ACEs apply to Everyone (including the human user).
- ~750ms polling window between agent detection and ACE application.
- Audit logs in SQLite are user-writable.
- Process-level bypass: killing the daemon removes protection.

These limitations are addressed in Phase 2 (kernel minifilter driver).

### Phase 2 Improvements (In Development)

- Kernel-level I/O IRP interception (ring 0)
- Protection survives daemon restart
- Agent-only blocking (humans retain access)
- Zero polling delay
- Tamper-proof kernel-level audit

## Disclosure Policy

We follow a coordinated disclosure process:

1. Vulnerability reported privately
2. Fix developed and tested
3. Release prepared with security advisory
4. Public disclosure after 30 days or upon mutual agreement

## Recognition

We maintain a hall of fame for security researchers who responsibly disclose vulnerabilities. Credit will be given in release notes and on the website (unless anonymity is requested).

---

**Phylax is a security tool. It does NOT guarantee absolute security.** See [DISCLAIMER.md](DISCLAIMER.md) for full details.
