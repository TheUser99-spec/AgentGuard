# AGENTS.md — AgentGuard Codebase Guide

Este fichero existe para que los agentes de IA (Claude Code, Cursor, Copilot, etc.)
entiendan la arquitectura antes de tocar cualquier cosa.

---

## Qué es este proyecto

AgentGuard es una herramienta de seguridad para Windows que bloquea a otros agentes
de IA a nivel OS. Controla qué ficheros pueden leer, escribir o borrar.

Stack: **Rust** (todo excepto el driver). El driver kernel es C++ (Phase 2, carpeta `driver/`).
**No modifiques** la carpeta `driver/` a menos que se te pida explícitamente.

---

## Estructura del workspace

```
crates/
  agentguard-core/      ← TIPOS BASE. Zero deps externas. Si tocas esto, afectas todo.
  agentguard-manifest/  ← Parser agentguard.toml + GlobSets compilados
  agentguard-policy/    ← Motor de decisiones: deny > ask > full > delete > write > read
  agentguard-store/     ← SQLite (rusqlite). Todas las tablas. Nadie más toca la DB.
  agentguard-probe/     ← Poller ToolHelp32 + SubjectClassifier (Windows only)
  agentguard-enforce/   ← DENY ACEs + MIC labels (Windows only)
  agentguard-ipc/       ← Named pipe daemon <-> CLI
  agentguard-notify/    ← Toast notifications Windows para [ask]
  agentguard-audit/     ← Escribe AuditEvents a agentguard-store
  agentguard-daemon/    ← Windows Service. Orquesta todo. Binario principal.
  agentguard-cli/       ← CLI: agentguard init / status / project / daemon
  agentguard-tui/       ← Dashboard ratatui

modules/
  agentguard-scanner/   ← Phase 3. NO tocar hasta que se indique.
  agentguard-team/      ← Phase 4. NO tocar hasta que se indique.

driver/                 ← C++ minifilter. Phase 2. NO tocar.
docs/adr/               ← Architecture Decision Records. Leer antes de cambiar arquitectura.
```

---

## Reglas que DEBES seguir

### 1. Orden de dependencias — nunca romperlo

```
core ← manifest ← policy ← (enforce, audit, probe, notify, ipc) ← daemon
                                                                  ← cli
                                                                  ← tui
```

- `agentguard-core` NO puede depender de ningún otro crate del workspace.
- `agentguard-manifest` y `agentguard-policy` son portables — sin Windows APIs.
- `agentguard-store` es portable (rusqlite es cross-platform).
- `agentguard-probe` y `agentguard-enforce` son Windows-only.

### 2. Nunca toques agentguard-core sin consenso explícito

Los tipos en `agentguard-core` son la interfaz entre todos los crates.
Un cambio aquí rompe el workspace entero. Pregunta antes de modificar.

### 3. Realpath SIEMPRE antes de glob match

```rust
// CORRECTO
let canonical = std::fs::canonicalize(&path)?;
let relative  = canonical.strip_prefix(&workspace_root)?;
compiled_manifest.evaluate(relative, &op);

// INCORRECTO — symlink bypass (CVE-2025-59829)
compiled_manifest.evaluate(&path, &op);
```

### 4. Toda la DB pasa por agentguard-store

Ningún otro crate importa `rusqlite` directamente. Si necesitas leer o escribir
algo en la DB, añade un método a `agentguard_store::Store`.

### 5. Tests antes de dar nada por hecho

```bash
cargo test --workspace          # todos los tests
cargo test -p agentguard-manifest  # solo un crate
```

Los tests de `agentguard-manifest` y `agentguard-store` son los más críticos.
No mergees nada que rompa esos tests.

### 6. Los módulos (Phase 3/4) no tocan core

`agentguard-scanner` y `agentguard-team` implementan el trait `Module`:
```rust
trait Module: Send + Sync {
    fn name(&self) -> &str;
    fn on_agent_event(&self, event: &AgentEvent) -> GuardResult<()>;
}
```
No pueden importar `agentguard-enforce`, `agentguard-probe`, ni `agentguard-daemon`.

---

## El modelo de permisos — entiéndelo antes de tocar policy

Los 6 buckets en orden de prioridad:

| Prioridad | Bucket | Read | Write | Delete |
|-----------|--------|------|-------|--------|
| 1 (máx)   | deny   | ✗    | ✗     | ✗      |
| 2         | ask    | ?    | ?     | ?      |
| 3         | full   | ✓    | ✓     | ✓      |
| 4         | delete | ✓    | ✗     | ✓      |
| 5         | write  | ✓    | ✓     | ✗      |
| 6         | read   | ✓    | ✗     | ✗      |

**deny SIEMPRE gana**, incluso si el fichero también aparece en write o full.
Si un fichero no aparece en ningún bucket → se aplica `default_mode`:
- `conservative`: read=Allow, write=Ask, delete=Deny
- `unrestricted`: todo Allow

---

## Qué está implementado (Phase 1 en curso)

| Crate | Estado |
|-------|--------|
| agentguard-core | ✅ Tipos base + errors (5 tests) |
| agentguard-manifest | ✅ Parser + GlobSets + discovery (12 tests) |
| agentguard-store | ✅ SQLite + migraciones + list_projects + count_events_today (6 tests) |
| agentguard-policy | ✅ CompiledPolicy global > project (3 tests) |
| agentguard-audit | ✅ Auditor funcional (0 tests propios, 6 store) |
| agentguard-probe | ✅ SubjectClassifier 5 señales + AgentSessionTracker + ProcessPoller (10 tests) |
| agentguard-enforce | ✅ DENY ACEs 3-capas + MIC + Enforcer walkdir coordinator (8 tests) |
| agentguard-ipc | ✅ Named pipe bidirectional + codec + integration tests (28 tests) |
| agentguard-notify | ✅ Windows MessageBoxW + Unix terminal prompt (6 tests: 5 Unix + 1 Windows) |
| agentguard-daemon | ✅ Entry point + orchestrator + watcher + poller + handler IPC (0 tests propios) |
| agentguard-cli | ✅ 14 comandos conectados a IPC vía IpcClient (22 tests parsing) |
| agentguard-tui | ⏳ Dashboard ratatui — deferred to post-Phase 1 |

---

## Phase 1 — 100% implementado (11/12 crates, 100 tests)

Todos los crates Phase 1 compilan, pasan tests y clippy sin warnings propios.

## Phase 1.5 — Dynamic Agent Detection ✅

Poller ToolHelp32 (`agentguard-probe::poller`) implementado y conectado al daemon:
- Snapshot cada 750ms de todos los procesos del sistema
- Detecta procesos nuevos (Started) y desaparecidos (Exited)
- Classifier S2 (image name) + S5 (inheritance via parent PID)
- Validación PID reuse con `GetProcessTimes`
- Al detectar agente → protege todos los proyectos registrados
- Al desaparecer el último agente → libera todas las protecciones
- El daemon muestra `Dynamic agent detection: ACTIVE (750ms polling)` al arrancar

Lo que queda para release v0.1.0: ADRs, .msi installer, Windows Service wrapper.

---

## Comandos útiles

```bash
# Compilar todo el workspace
cargo build --workspace

# Tests de los crates portables (funcionan en cualquier OS)
cargo test -p agentguard-core
cargo test -p agentguard-manifest
cargo test -p agentguard-store
cargo test -p agentguard-policy

# Ver árbol de dependencias
cargo tree -p agentguard-daemon

# Formatear
cargo fmt --all

# Linter
cargo clippy --workspace
```

---

## Ficheros que NO debes modificar sin permiso explícito

- `Cargo.toml` raíz del workspace (añadir deps compartidas requiere consenso)
- `crates/agentguard-core/src/types.rs` (cambiar tipos rompe todo)
- `crates/agentguard-store/src/migrations.rs` (cambiar schema requiere migración)
- `driver/**` (C++, Phase 2)
- `modules/**` (Phase 3/4)
- `AGENTS.md` (este fichero)
- `docs/adr/**` (solo añadir, nunca borrar)
