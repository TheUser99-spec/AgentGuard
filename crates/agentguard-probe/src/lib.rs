//! ETW consumer + SubjectClassifier para deteccion de agentes de IA.
//!
//! Senales de clasificacion:
//!   S1: Variables de entorno conocidas (CLAUDE_CODE, ANTHROPIC_API_KEY...)
//!   S2: Nombre de imagen exacto (claude.exe, cursor.exe, goose.exe...)
//!   S3: node.exe con cmdline que menciona un agente (claude, cline...)
//!   S4: Proceso sin sesion interactiva (session_id==0, sin window station)
//!   S5: Herencia del padre (hijo de un agente -> Inherited)

#![allow(unsafe_code)]

pub mod classifier;
pub mod poller;
pub mod tracker;

pub use classifier::{ClassifierConfig, ProcessInfo, SubjectClassifier};
pub use poller::{ProcessEvent, ProcessPoller};
pub use tracker::AgentSessionTracker;
