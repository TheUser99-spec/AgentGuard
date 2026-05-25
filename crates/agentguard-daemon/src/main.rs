//! AgentGuard Daemon — Windows Service.
//!
//! Orquesta: probe + policy + enforce + notify + audit.
//! IPC server para CLI + watcher para hot-reload de agentguard.toml.

#![allow(unsafe_code)]

mod handler;
mod orchestrator;
mod watcher;

use agentguard_ipc::IpcServer;
use agentguard_probe::ProcessPoller;
use orchestrator::DaemonState;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let db_path = agentguard_store::Store::default_path();
    eprintln!("[daemon] DB path: {}", db_path.display());

    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let (watcher_shutdown_tx, watcher_shutdown_rx) = mpsc::channel(1);

    eprintln!("[daemon] Initialising DaemonState...");
    let state = match DaemonState::new(&db_path, shutdown_tx) {
        Ok(s) => {
            eprintln!("[daemon] DaemonState ready.");
            s
        }
        Err(e) => {
            eprintln!("Failed to initialise daemon: {e}");
            std::process::exit(1);
        }
    };

    let state = Arc::new(state);

    // IPC request handler
    let ipc_state = Arc::clone(&state);
    let handler: agentguard_ipc::RequestHandler =
        Arc::new(move |req| handler::handle(Arc::clone(&ipc_state), req));

    let server = IpcServer::new(handler);

    // File watcher for hot-reload
    let watcher_state = Arc::clone(&state);

    // Poller for dynamic agent detection
    let poller = ProcessPoller::new(
        state.tracker.classifier.clone(),
        state.tracker.clone(),
    );
    let (poller_tx, mut poller_rx) = mpsc::channel(64);
    let (poller_stop_tx, poller_stop_rx) = mpsc::channel(1);

    // Spawn poller as a separate task — we await it on shutdown
    let poller_task = tokio::spawn(poller.run(poller_tx, poller_stop_rx, 750));

    // Spawn event processing loop
    let event_state = Arc::clone(&state);
    tokio::spawn(async move {
        while let Some(event) = poller_rx.recv().await {
            event_state.on_process_event(&event);
        }
    });

    println!("AgentGuard Daemon v{} started", env!("CARGO_PKG_VERSION"));
    println!("Press Ctrl+C to stop.");
    println!("Dynamic agent detection: ACTIVE (750ms polling)");

    // Run IPC server, watcher, and Ctrl+C handler concurrently
    tokio::select! {
        result = server.run(shutdown_rx) => {
            if let Err(e) = result {
                eprintln!("IPC server error: {e}");
            }
        }
        result = watcher::run_watcher(watcher_state, watcher_shutdown_rx) => {
            if let Err(e) = result {
                eprintln!("Watcher error: {e}");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nCtrl+C received, shutting down...");
        }
    }

    // Signal all components to stop
    drop(poller_stop_tx);
    drop(watcher_shutdown_tx);

    // Wait for poller to actually exit (spawn_blocking task)
    let _ = poller_task.await;

    println!("AgentGuard Daemon stopped.");
}
