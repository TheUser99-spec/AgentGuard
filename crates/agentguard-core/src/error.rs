use thiserror::Error;

#[derive(Debug, Error)]
pub enum GuardError {
    // ── Store ──
    #[error("database error: {0}")]
    Database(String),

    #[error("migration failed at version {version}: {reason}")]
    Migration { version: u32, reason: String },

    // ── Manifest ──
    #[error("failed to parse phylax.toml: {0}")]
    ManifestParse(String),

    #[error("invalid glob pattern '{pattern}': {reason}")]
    InvalidGlob { pattern: String, reason: String },

    #[error("phylax.toml not found in '{path}'")]
    ManifestNotFound { path: String },

    // ── Policy ──
    #[error("policy evaluation error: {0}")]
    PolicyError(String),

    // ── IPC ──
    #[error("IPC connection failed: {0}")]
    IpcConnect(String),

    #[error("IPC message serialization error: {0}")]
    IpcSerialize(String),

    #[error("IPC timeout after {ms}ms")]
    IpcTimeout { ms: u64 },

    #[error("IPC error: {0}")]
    IpcError(String),

    #[error("daemon is not running")]
    DaemonNotRunning,

    // ── Probe ──
    #[error("ETW session error: {0}")]
    EtwSession(String),

    #[error("process classification error: {0}")]
    Classification(String),

    // ── Enforce ──
    #[error("failed to apply ACE on '{path}': {reason}")]
    AceApply { path: String, reason: String },

    #[error("failed to remove ACE on '{path}': {reason}")]
    AceRemove { path: String, reason: String },

    #[error("enforcement failed for '{path}': {reason}")]
    EnforcementFailed { path: String, reason: String },

    // ── Notify ──
    #[error("notification error: {0}")]
    Notification(String),

    // ── Daemon ──
    #[error("daemon error: {0}")]
    Daemon(String),

    // ── Generic ──
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type GuardResult<T> = Result<T, GuardError>;
