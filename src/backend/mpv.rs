//! mpv runs as a sidecar process that renders straight into our window (`--wid`)
//! and is driven over its JSON IPC named pipe. GPUI composites its
//! transparent UI above that child window (topmost DirectComposition target).

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use super::Event;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::windows::named_pipe::ClientOptions,
    process::{Child, Command},
    sync::{mpsc, oneshot, Mutex},
};

/// Properties mirrored to the UI as `mpv-prop` events.
const OBSERVED: &[&str] = &[
    "time-pos",
    "duration",
    "pause",
    "path",
    "filename",
    "volume",
    "mute",
    "eof-reached",
    "idle-active",
    "width",
    "height",
    "container-fps",
    "track-list",
    "speed",
    "seeking",
];

#[allow(dead_code)]
pub struct Mpv {
    tx: mpsc::UnboundedSender<String>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    child: Mutex<Option<Child>>,
}

impl Mpv {
    pub async fn start(tx_ui: smol::channel::Sender<Event>, wid: isize) -> Result<Arc<Mpv>> {
        let pipe = format!(r"\\.\pipe\clipdat-mpv-{}", std::process::id());
        let exe = super::util::sidecar("mpv")?;

        let mut cmd = Command::new(&exe);
        cmd.args([
            format!("--wid={wid}"),
            format!("--input-ipc-server={pipe}"),
            "--no-config".into(),
            "--idle=yes".into(),
            "--force-window=yes".into(),
            "--keep-open=yes".into(),
            "--osd-level=0".into(),
            "--osd-bar=no".into(),
            "--input-default-bindings=no".into(),
            "--input-vo-keyboard=no".into(),
            "--input-cursor=no".into(),
            "--cursor-autohide=no".into(),
            "--window-dragging=no".into(),
            "--hwdec=auto-safe".into(),
            "--vo=gpu-next".into(),
            "--gpu-api=d3d11".into(),
            "--hr-seek=yes".into(),
            // Local files don't need a big cache; keep memory low.
            "--demuxer-max-bytes=48MiB".into(),
            "--demuxer-max-back-bytes=16MiB".into(),
            "--audio-display=no".into(),
            "--volume-max=150".into(),
            "--background-color=#000000".into(),
            "--terminal=no".into(),
        ]);
        cmd.kill_on_drop(true).creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let mut child = cmd.spawn().with_context(|| format!("spawning {}", exe.display()))?;
        super::util::bind_to_job(&child);

        // mpv creates the pipe shortly after start.
        let started = Instant::now();
        let client = loop {
            match ClientOptions::new().open(&pipe) {
                Ok(c) => break c,
                Err(_) if started.elapsed() < Duration::from_secs(10) => {
                    // Fail fast if mpv already quit (e.g. an unsupported option).
                    if let Ok(Some(status)) = child.try_wait() {
                        bail!("mpv exited at startup ({status})");
                    }
                    tokio::time::sleep(Duration::from_millis(15)).await
                }
                Err(e) => bail!("mpv IPC pipe never appeared: {e}"),
            }
        };
        let (rd, mut wr) = tokio::io::split(client);

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Some(mut line) = rx.recv().await {
                line.push('\n');
                if wr.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }
        });

        let mpv = Arc::new(Mpv {
            tx,
            next_id: AtomicU64::new(1),
            pending: Arc::new(Mutex::new(HashMap::new())),
            child: Mutex::new(Some(child)),
        });

        // Reader: replies resolve pending requests, events go to the UI.
        // time-pos fires every frame (144+/s); coalesce it to ~60 Hz.
        let pending = mpv.pending.clone();
        let latest_time: Arc<std::sync::Mutex<(Option<Value>, bool)>> =
            Arc::new(std::sync::Mutex::new((None, false)));
        {
            let ui = tx_ui.clone();
            let latest_time = latest_time.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(rd).lines();
                let mut last_emit = Instant::now() - Duration::from_secs(1);
                while let Ok(Some(line)) = lines.next_line().await {
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                    if let Some(id) = msg.get("request_id").and_then(Value::as_u64) {
                        if let Some(tx) = pending.lock().await.remove(&id) {
                            let ok = msg.get("error").and_then(Value::as_str) == Some("success");
                            let _ = tx.send(if ok {
                                Ok(msg.get("data").cloned().unwrap_or(Value::Null))
                            } else {
                                Err(msg["error"].as_str().unwrap_or("error").to_string())
                            });
                        }
                        continue;
                    }
                    match msg.get("event").and_then(Value::as_str) {
                        Some("property-change") => {
                            let name = msg["name"].as_str().unwrap_or_default();
                            if name == "time-pos" {
                                if last_emit.elapsed() >= Duration::from_millis(15) {
                                    last_emit = Instant::now();
                                    *latest_time.lock().unwrap() = (None, false);
                                    let _ = ui.try_send(Event::Prop(name.to_string(), msg["data"].clone()));
                                } else {
                                    *latest_time.lock().unwrap() = (Some(msg["data"].clone()), true);
                                }
                            } else {
                                let _ = ui.try_send(Event::Prop(name.to_string(), msg["data"].clone()));
                            }
                        }
                        Some(_) => {
                            let _ = ui.try_send(Event::Mpv(msg.clone()));
                        }
                        None => {}
                    }
                }
                let _ = ui.try_send(Event::Mpv(json!({"event": "mpv-exited"})));
            });
        }
        // Flush a coalesced time-pos that would otherwise be dropped (e.g. last seek while paused).
        {
            let ui = tx_ui.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_millis(30));
                loop {
                    tick.tick().await;
                    let v = {
                        let mut g = latest_time.lock().unwrap();
                        if g.1 {
                            g.1 = false;
                            g.0.take()
                        } else {
                            None
                        }
                    };
                    if let Some(v) = v {
                        let _ = ui.try_send(Event::Prop("time-pos".into(), v));
                    }
                }
            });
        }

        for (i, name) in OBSERVED.iter().enumerate() {
            mpv.send_raw(json!({"command": ["observe_property", i + 1, name]}));
        }

        // Keep mpv's child window disabled for input, so clicks reach GPUI.
        tokio::task::spawn_blocking(move || {
            let t = Instant::now();
            while t.elapsed() < Duration::from_secs(10) {
                if super::util::tame_mpv_window(wid) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });

        Ok(mpv)
    }

    fn send_raw(&self, v: Value) {
        let _ = self.tx.send(v.to_string());
    }

    /// Send a command and wait for mpv's reply.
    #[allow(dead_code)]
    pub async fn command(&self, args: Vec<Value>) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1_000;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.send_raw(json!({"command": args, "request_id": id}));
        match tokio::time::timeout(Duration::from_secs(10), rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(anyhow!("mpv: {e}")),
            _ => {
                self.pending.lock().await.remove(&id);
                Err(anyhow!("mpv did not reply"))
            }
        }
    }

    /// Fire-and-forget (used for scrubbing, where only the latest seek matters).
    pub fn command_nowait(&self, args: Vec<Value>) {
        self.send_raw(json!({"command": args}));
    }

    #[allow(dead_code)]
    pub async fn kill(&self) {
        if let Some(mut c) = self.child.lock().await.take() {
            let _ = c.kill().await;
        }
    }
}
