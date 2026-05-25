//! Parser y validador del fichero `agentguard.toml`.
//! Convierte el TOML en un `ProjectManifest` y lo compila
//! a `CompiledManifest` con GlobSets listos para matching O(1).

mod compiled;
mod discovery;
mod parser;

pub use compiled::CompiledManifest;
pub use discovery::find_manifest;
pub use parser::ProjectManifest;
