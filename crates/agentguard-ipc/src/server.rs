use agentguard_core::{GuardError, GuardResult};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::protocol::{self, IpcRequest, IpcResponse};

pub type RequestHandler = Arc<dyn Fn(IpcRequest) -> IpcResponse + Send + Sync>;

pub struct IpcServer {
    handler: RequestHandler,
    pipe_override: Option<String>,
}

impl IpcServer {
    pub fn new(handler: RequestHandler) -> Self {
        Self {
            handler,
            pipe_override: None,
        }
    }

    pub fn with_pipe(handler: RequestHandler, pipe: String) -> Self {
        Self {
            handler,
            pipe_override: Some(pipe),
        }
    }

    fn pipe_name(&self) -> String {
        self.pipe_override
            .clone()
            .unwrap_or_else(|| crate::protocol::pipe_name().to_string())
    }

    pub async fn run(self, shutdown_rx: mpsc::Receiver<()>) -> GuardResult<()> {
        #[cfg(windows)]
        self.run_windows(shutdown_rx).await?;

        #[cfg(not(windows))]
        self.run_unix(shutdown_rx).await?;

        Ok(())
    }

    #[cfg(not(windows))]
    async fn run_unix(self, mut shutdown_rx: mpsc::Receiver<()>) -> GuardResult<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::UnixListener;

        let path = self.pipe_name();
        let _ = std::fs::remove_file(&path);

        let listener = UnixListener::bind(&path)
            .map_err(|e| GuardError::IpcError(format!("failed to bind {path}: {e}")))?;

        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok((mut stream, _)) => {
                            let handler = Arc::clone(&self.handler);
                            tokio::spawn(async move {
                                loop {
                                    let req: IpcRequest = match protocol::recv(&mut stream).await {
                                        Ok(r) => r,
                                        Err(_) => break,
                                    };
                                    let resp = (handler)(req);
                                    if protocol::send(&mut stream, &resp).await.is_err() {
                                        break;
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("IPC accept error: {e}");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    let _ = std::fs::remove_file(&path);
                    break;
                }
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    async fn run_windows(self, mut shutdown_rx: mpsc::Receiver<()>) -> GuardResult<()> {
        use tokio::net::windows::named_pipe::ServerOptions;

        let pipe_name = self.pipe_name();
        let mut pipe_security = PipeSecurity::new()?;
        let mut server = ServerOptions::new()
            .first_pipe_instance(false)
            .create_with_security_attributes(&pipe_name, &mut pipe_security)
            .map_err(|e| create_pipe_error(&pipe_name, e))?;

        loop {
            tokio::select! {
                connect = server.connect() => {
                    match connect {
                        Ok(()) => {
                            let mut pipe = server;
                            server = ServerOptions::new()
                                .first_pipe_instance(false)
                                .create_with_security_attributes(&pipe_name, &mut pipe_security)
                                .map_err(|e| create_pipe_error(&pipe_name, e))?;
                            let handler = Arc::clone(&self.handler);
                            tokio::spawn(async move {
                                loop {
                                    let req: IpcRequest = match protocol::recv(&mut pipe).await {
                                        Ok(r) => r,
                                        Err(_) => break,
                                    };
                                    let resp = (handler)(req);
                                    if protocol::send(&mut pipe, &resp).await.is_err() {
                                        break;
                                    }
                                }
                            });
                        }
                        Err(e) => eprintln!("IPC connect error: {e}"),
                    }
                }
                _ = shutdown_rx.recv() => break,
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
fn create_pipe_error(pipe_name: &str, error: std::io::Error) -> GuardError {
    GuardError::IpcError(format!("failed to create named pipe {pipe_name}: {error}"))
}

#[cfg(windows)]
trait ServerOptionsSecurityExt {
    fn create_with_security_attributes(
        &mut self,
        pipe_name: &str,
        security: &mut PipeSecurity,
    ) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeServer>;
}

#[cfg(windows)]
impl ServerOptionsSecurityExt for tokio::net::windows::named_pipe::ServerOptions {
    fn create_with_security_attributes(
        &mut self,
        pipe_name: &str,
        security: &mut PipeSecurity,
    ) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
        unsafe { self.create_with_security_attributes_raw(pipe_name, security.as_mut_ptr()) }
    }
}

#[cfg(windows)]
struct PipeSecurity {
    descriptor: *mut std::ffi::c_void,
    attrs: windows_sys::Win32::Security::SECURITY_ATTRIBUTES,
}

#[cfg(windows)]
unsafe impl Send for PipeSecurity {}

#[cfg(windows)]
impl PipeSecurity {
    fn new() -> GuardResult<Self> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Security::Authorization::{
            ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
        };

        // Local-only pipe. Remote clients remain rejected by ServerOptions.
        let sddl = std::ffi::OsStr::new("D:P(A;;GA;;;WD)")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        let mut descriptor = std::ptr::null_mut();
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(GuardError::IpcError(
                "failed to build named pipe security descriptor".into(),
            ));
        }

        Ok(Self {
            descriptor,
            attrs: windows_sys::Win32::Security::SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>()
                    as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            },
        })
    }

    fn as_mut_ptr(&mut self) -> *mut std::ffi::c_void {
        (&mut self.attrs as *mut windows_sys::Win32::Security::SECURITY_ATTRIBUTES).cast()
    }
}

#[cfg(windows)]
impl Drop for PipeSecurity {
    fn drop(&mut self) {
        if !self.descriptor.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::LocalFree(self.descriptor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_with_pipe_overrides_default() {
        let handler: RequestHandler = Arc::new(|_req| IpcResponse::Ok);
        let s = IpcServer::with_pipe(handler, "\\\\.\\pipe\\custom".into());
        assert_eq!(s.pipe_name(), "\\\\.\\pipe\\custom");
    }

    #[test]
    fn server_new_uses_default_pipe() {
        let handler: RequestHandler = Arc::new(|_req| IpcResponse::Ok);
        let s = IpcServer::new(handler);
        let name = s.pipe_name();
        assert!(!name.is_empty());
    }
}
