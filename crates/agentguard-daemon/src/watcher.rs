use agentguard_core::GuardResult;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::orchestrator::DaemonState;

#[derive(Debug)]
#[allow(dead_code)]
pub enum WatchEvent {
    ManifestChanged(PathBuf),
}

pub async fn run_watcher(state: Arc<DaemonState>, stop_rx: mpsc::Receiver<()>) -> GuardResult<()> {
    #[cfg(windows)]
    return win_watch::run(state, stop_rx).await;

    #[cfg(not(windows))]
    return dev_poll::run(state, stop_rx).await;
}

#[cfg(windows)]
mod win_watch {
    use super::*;
    use std::collections::HashSet;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
        FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_INFORMATION,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    pub async fn run(state: Arc<DaemonState>, mut stop_rx: mpsc::Receiver<()>) -> GuardResult<()> {
        let mut watched = HashSet::new();

        loop {
            for workspace in state.list_projects() {
                if watched.insert(workspace.clone()) {
                    let s = Arc::clone(&state);
                    tokio::task::spawn_blocking(move || {
                        watch_directory(workspace, s);
                    });
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(500)) => {}
                _ = stop_rx.recv() => break,
            }
        }

        Ok(())
    }

    fn watch_directory(workspace: PathBuf, state: Arc<DaemonState>) {
        let wide: Vec<u16> = OsStr::new(&workspace.to_string_lossy().as_ref())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            tracing::error!("Watcher: cannot open {:?}", workspace);
            return;
        }

        let mut buf = vec![0u8; 4096];
        let mut bytes_returned: u32 = 0;

        loop {
            let ok = unsafe {
                ReadDirectoryChangesW(
                    handle,
                    buf.as_mut_ptr() as *mut _,
                    buf.len() as u32,
                    0, // watch subtree = false
                    FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_FILE_NAME,
                    &mut bytes_returned,
                    std::ptr::null_mut(),
                    None,
                )
            };

            if ok == 0 {
                break;
            }

            let mut offset = 0usize;
            loop {
                let info =
                    unsafe { &*(buf.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION) };
                let name_slice = unsafe {
                    std::slice::from_raw_parts(
                        info.FileName.as_ptr(),
                        info.FileNameLength as usize / 2,
                    )
                };
                let name = String::from_utf16_lossy(name_slice);

                if name.eq_ignore_ascii_case("agentguard.toml") {
                    tracing::info!("Watcher: agentguard.toml changed in {:?}", workspace);
                    if let Err(e) = state.reload_project(&workspace) {
                        tracing::error!("Hot-reload error: {e}");
                    }
                }

                if info.NextEntryOffset == 0 {
                    break;
                }
                offset += info.NextEntryOffset as usize;
            }
        }

        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
    }
}

#[cfg(not(windows))]
mod dev_poll {
    use super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;

    pub async fn run(state: Arc<DaemonState>, mut stop_rx: mpsc::Receiver<()>) -> GuardResult<()> {
        let mut last_modified: HashMap<PathBuf, SystemTime> = HashMap::new();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {
                    for workspace in state.list_projects() {
                        let toml = workspace.join("agentguard.toml");
                        if let Ok(meta) = std::fs::metadata(&toml) {
                            if let Ok(modified) = meta.modified() {
                                let prev = last_modified.get(&workspace).copied();
                                if prev.map(|p| p != modified).unwrap_or(true) {
                                    last_modified.insert(workspace.clone(), modified);
                                    if prev.is_some() {
                                        if let Err(e) = state.reload_project(&workspace) {
                                            tracing::error!("Hot-reload: {e}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ = stop_rx.recv() => break,
            }
        }
        Ok(())
    }
}
