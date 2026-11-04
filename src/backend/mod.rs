//! Everything that isn't UI: mpv control, probing, NVENC exports, settings, OS glue.
//! Runs on a tokio runtime; talks to the GPUI app through `Event`s on a channel.

pub mod export;
pub mod gpu;
pub mod instance;
pub mod media;
pub mod mpv;
pub mod settings;
pub mod util;

use std::sync::OnceLock;

use serde_json::Value;

pub enum Event {
    /// An observed mpv property changed.
    Prop(String, Value),
    /// Any other mpv event (file-loaded, end-file, …).
    Mpv(Value),
    /// Export job progress / completion.
    Job(export::JobEvent),
    /// Another instance (or Explorer) asked us to open a file.
    OpenFile(String),
}

pub fn rt() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("cliper-io")
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

/// Run a future on the backend runtime and await it from any executor (e.g. GPUI's).
pub fn run<F, T>(f: F) -> impl std::future::Future<Output = T>
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    rt().spawn(async move {
        let _ = tx.send(f.await);
    });
    async move { rx.await.expect("backend task dropped") }
}
