use crate::classifier::ProcessInfo;
use crate::tracker::AgentSessionTracker;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ProcessPoller {
    tracker: Arc<AgentSessionTracker>,
}

#[derive(Debug)]
pub enum ProcessEvent {
    Started(ProcessInfo),
    Exited(u32),
}

impl ProcessPoller {
    pub fn new(
        _classifier: Arc<crate::classifier::SubjectClassifier>,
        tracker: Arc<AgentSessionTracker>,
    ) -> Self {
        Self { tracker }
    }

    pub async fn run(
        self,
        tx: mpsc::Sender<ProcessEvent>,
        mut stop_rx: mpsc::Receiver<()>,
        interval_ms: u64,
    ) {
        #[cfg(not(windows))]
        {
            let _ = (self, tx, interval_ms);
            let _ = stop_rx;
        }

        #[cfg(windows)]
        {
            let tracker = self.tracker;
            let stopped = Arc::new(AtomicBool::new(false));
            let stopped_flag = stopped.clone();

            let handle = tokio::task::spawn_blocking(move || {
                poll_loop(tracker, tx, stopped_flag, interval_ms);
            });

            // Wait for stop signal
            stop_rx.recv().await;

            // Signal poller to stop and wait for it
            stopped.store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = handle.await;
        }
    }
}

#[cfg(windows)]
fn poll_loop(
    tracker: Arc<AgentSessionTracker>,
    tx: mpsc::Sender<ProcessEvent>,
    stopped: Arc<AtomicBool>,
    interval_ms: u64,
) {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::Foundation::CloseHandle;

    let mut prev: HashMap<u32, ProcessSnapshot> = HashMap::new();

    loop {
        if stopped.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

        if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            std::thread::sleep(std::time::Duration::from_millis(interval_ms));
            continue;
        }

        let mut current: HashMap<u32, ProcessSnapshot> = HashMap::new();
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..unsafe { std::mem::zeroed() }
        };

        let mut ok = unsafe { Process32FirstW(handle, &mut entry) };
        while ok != 0 {
            let pid = entry.th32ProcessID;
            let parent_pid = entry.th32ParentProcessID;
            let image_name = OsString::from_wide(trim_null(&entry.szExeFile))
                .to_string_lossy()
                .to_string();

            let creation_time = get_creation_time(pid);

            current.insert(
                pid,
                ProcessSnapshot { creation_time },
            );

            if let Some(old) = prev.remove(&pid) {
                if old.creation_time != creation_time {
                    if let Some(info) = build_info(pid, &image_name, parent_pid) {
                        let _ = tx.blocking_send(ProcessEvent::Exited(pid));
                        tracker.on_process_start(&info, None);
                        let _ = tx.blocking_send(ProcessEvent::Started(info));
                    }
                }
            } else if let Some(info) = build_info(pid, &image_name, parent_pid) {
                tracker.on_process_start(&info, None);
                let _ = tx.blocking_send(ProcessEvent::Started(info));
            }

            ok = unsafe { Process32NextW(handle, &mut entry) };
        }

        for (pid, _snap) in prev.drain() {
            tracker.on_process_exit(pid);
            let _ = tx.blocking_send(ProcessEvent::Exited(pid));
        }

        prev = current;

        unsafe { CloseHandle(handle); }

        let mut elapsed = 0u64;
        while elapsed < interval_ms && !stopped.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            elapsed += 50;
        }
    }
}

#[cfg(windows)]
#[derive(Debug)]
struct ProcessSnapshot {
    creation_time: Option<u64>,
}

#[cfg(windows)]
fn get_creation_time(pid: u32) -> Option<u64> {
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }

    let mut creation = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut exit = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut kernel = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let mut user = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };

    let ok = unsafe {
        GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user)
    };

    unsafe { CloseHandle(handle); }

    if ok == 0 {
        return None;
    }

    Some(((creation.dwHighDateTime as u64) << 32) | (creation.dwLowDateTime as u64))
}

#[cfg(windows)]
fn build_info(pid: u32, image_name: &str, parent_pid: u32) -> Option<ProcessInfo> {
    Some(ProcessInfo {
        pid,
        image_name: image_name.to_string(),
        cmdline: String::new(),
        env_vars: vec![],
        session_id: 1,
        has_window: true,
        parent_pid: if parent_pid == 0 { None } else { Some(parent_pid) },
    })
}

#[cfg(windows)]
fn trim_null(wide: &[u16; 260]) -> &[u16] {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    &wide[..end]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn poller_creation() {
        let tracker = Arc::new(AgentSessionTracker::new(
            crate::classifier::SubjectClassifier::with_defaults(),
        ));
        let classifier = Arc::new(crate::classifier::SubjectClassifier::with_defaults());
        let _poller = ProcessPoller::new(classifier, tracker);
    }
}
