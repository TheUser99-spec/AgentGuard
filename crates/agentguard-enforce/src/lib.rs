//! Aplicador de DENY ACEs via SetNamedSecurityInfo + Job Objects para containment.
//!
//! Phase 1: DENY ACEs sobre archivos protegidos.
//! ACE cleanup al morir el agente o al desregistrar el proyecto.

#![allow(unsafe_code)]

pub mod ace;
pub mod coordinator;

pub use coordinator::Enforcer;
