//! Single instance: the first Clipdat owns a named pipe; later launches hand it their file and exit.

use std::io::Write;

use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::ServerOptions;

use super::Event;

// Dev builds use their own pipe, so they run next to an installed Clipdat instead of
// handing their file to it.
const PIPE: &str = if cfg!(debug_assertions) { r"\\.\pipe\clipdat-instance-dev" } else { r"\\.\pipe\clipdat-instance" };

/// Returns true if another instance took the file (this process should exit).
pub fn forward_to_running(file: Option<&str>) -> bool {
    match std::fs::OpenOptions::new().write(true).open(PIPE) {
        Ok(mut f) => {
            let _ = f.write_all(file.unwrap_or("").as_bytes());
            true
        }
        Err(_) => false,
    }
}

pub fn serve(ui: smol::channel::Sender<Event>) {
    super::rt().spawn(async move {
        let Ok(mut server) = ServerOptions::new().first_pipe_instance(true).create(PIPE) else { return };
        loop {
            if server.connect().await.is_err() {
                break;
            }
            let mut buf = Vec::new();
            let _ = server.read_to_end(&mut buf).await;
            let path = String::from_utf8_lossy(&buf).trim().to_string();
            let _ = ui.send(Event::OpenFile(path)).await;
            server = match ServerOptions::new().create(PIPE) {
                Ok(s) => s,
                Err(_) => break,
            };
        }
    });
}
