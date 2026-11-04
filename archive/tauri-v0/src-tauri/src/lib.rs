mod export;
mod gpu;
mod media;
mod mpv;
mod util;

use std::sync::{Arc, LazyLock};

use serde_json::Value;
use tauri::{Emitter, Manager, State};
use tokio::sync::OnceCell;

use mpv::Mpv;

type CmdResult<T> = Result<T, String>;
fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

static JOBS: LazyLock<export::Jobs> = LazyLock::new(export::Jobs::default);
static GPU: OnceCell<gpu::GpuInfo> = OnceCell::const_new();

struct AppState {
    mpv: Arc<Mpv>,
    initial_file: std::sync::Mutex<Option<String>>,
}

/// First non-flag CLI argument that points at an existing file.
fn file_arg(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|a| !a.starts_with('-') && std::path::Path::new(a).is_file())
        .cloned()
}

// ---------- player ----------

#[tauri::command]
async fn mpv_command(state: State<'_, AppState>, args: Vec<Value>) -> CmdResult<Value> {
    state.mpv.command(args).await.map_err(err)
}

#[tauri::command]
fn mpv_nowait(state: State<'_, AppState>, args: Vec<Value>) {
    state.mpv.command_nowait(args);
}

#[tauri::command]
fn take_initial_file(state: State<'_, AppState>) -> Option<String> {
    state.initial_file.lock().unwrap().take()
}

// ---------- media ----------

#[tauri::command]
async fn probe(path: String) -> CmdResult<media::MediaInfo> {
    media::probe(&path).await.map_err(err)
}

#[tauri::command]
async fn keyframes(path: String) -> CmdResult<Vec<f64>> {
    media::keyframes(&path).await.map_err(err)
}

#[tauri::command]
async fn thumbnails(app: tauri::AppHandle, path: String, duration: f64, count: u32) -> CmdResult<Vec<media::Thumb>> {
    let root = app.path().app_local_data_dir().map_err(err)?;
    media::thumbnails(&root, &path, duration, count).await.map_err(err)
}

#[tauri::command]
async fn gpu_info() -> gpu::GpuInfo {
    GPU.get_or_init(gpu::detect).await.clone()
}

// ---------- export ----------

#[tauri::command]
fn export_start(app: tauri::AppHandle, req: export::ExportReq) -> CmdResult<u64> {
    export::start(app, &JOBS, req).map_err(err)
}

#[tauri::command]
fn export_cancel(id: u64) {
    JOBS.cancel(id);
}

#[tauri::command]
fn export_preview(req: export::ExportReq) -> String {
    export::preview(&req)
}

#[tauri::command]
async fn copy_files(paths: Vec<String>) -> CmdResult<()> {
    tokio::task::spawn_blocking(move || util::copy_files_to_clipboard(&paths))
        .await
        .map_err(err)?
        .map_err(err)
}

#[tauri::command]
fn reveal(path: String) {
    util::reveal_in_explorer(&path);
}

// ---------- settings ----------

fn settings_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_config_dir().map_err(err)?.join("settings.json"))
}

#[tauri::command]
fn settings_load(app: tauri::AppHandle) -> Value {
    settings_path(&app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null)
}

#[tauri::command]
fn settings_save(app: tauri::AppHandle, value: Value) -> CmdResult<()> {
    let p = settings_path(&app)?;
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(err)?;
    }
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&value).map_err(err)?).map_err(err)?;
    std::fs::rename(tmp, p).map_err(err)
}

#[tauri::command]
fn default_output_dir(app: tauri::AppHandle) -> String {
    app.path()
        .video_dir()
        .map(|d| d.join("Clips"))
        .unwrap_or_else(|_| std::env::temp_dir())
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
fn open_default_apps_settings() {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps"])
        .spawn();
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            if let Some(f) = file_arg(&argv) {
                let _ = app.emit("open-file", f);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let win = app.get_webview_window("main").expect("main window");
            let hwnd = win.hwnd()?.0 as isize;
            let mpv = tauri::async_runtime::block_on(Mpv::start(app.handle().clone(), hwnd))?;
            let args: Vec<String> = std::env::args().collect();
            app.manage(AppState { mpv, initial_file: std::sync::Mutex::new(file_arg(&args)) });
            // Warm the encoder probe in the background.
            tauri::async_runtime::spawn(async {
                GPU.get_or_init(gpu::detect).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            mpv_command,
            mpv_nowait,
            take_initial_file,
            probe,
            keyframes,
            thumbnails,
            gpu_info,
            export_start,
            export_cancel,
            export_preview,
            copy_files,
            reveal,
            settings_load,
            settings_save,
            default_output_dir,
            open_default_apps_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(s) = app.try_state::<AppState>() {
                    tauri::async_runtime::block_on(s.mpv.kill());
                }
            }
        });
}
