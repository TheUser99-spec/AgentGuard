//! Notificaciones para el bucket [ask].
//!
//! Flujo:
//!   1. Agente intenta acceder a fichero en [ask]
//!   2. Daemon llama a Notifier::ask_user_blocking()
//!   3. Windows: MessageBoxW con MB_YESNO (Yes=AllowOnce, No=Deny)
//!      Unix: prompt de terminal (y/n)
//!   4. Sin respuesta / error -> Deny
//!
//! El caller (daemon) debe envolver en tokio::task::spawn_blocking
//! + tokio::time::timeout para control de timeout.

#![allow(unsafe_code)]

pub mod notifier;

pub use notifier::Notifier;
