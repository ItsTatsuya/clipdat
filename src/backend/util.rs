use std::path::PathBuf;

use anyhow::{anyhow, Result};

/// Finds a bundled tool: next to the exe (`bin\` or same folder), or the repo's `bin\` in dev builds.
pub fn sidecar(name: &str) -> Result<PathBuf> {
    let file = format!("{name}.exe");
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("CLIPDAT_BIN_DIR") {
        candidates.push(PathBuf::from(dir).join(&file));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        candidates.push(dir.join("bin").join(&file));
        candidates.push(dir.join(&file));
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin").join(&file));
    candidates
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow!("{file} not found (run scripts/fetch-deps.ps1)"))
}

/// A command for a sidecar that never flashes a console window.
pub fn sidecar_cmd(name: &str) -> Result<tokio::process::Command> {
    let mut c = tokio::process::Command::new(sidecar(name)?);
    c.creation_flags(0x0800_0000).kill_on_drop(true);
    Ok(c)
}

pub fn reveal_in_explorer(path: &str) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", path.replace('/', "\\")))
        .spawn();
}

pub fn open_default_apps_settings() {
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", "ms-settings:defaultapps?registeredAppUser=Clipdat"])
        .spawn();
}

#[cfg(windows)]
mod win {
    use std::sync::OnceLock;
    use windows::Win32::{
        Foundation::{HANDLE, HWND, LPARAM, WPARAM},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        UI::{
            Input::KeyboardAndMouse::{EnableWindow, ReleaseCapture},
            WindowsAndMessaging::{
                EnumChildWindows, GetClassNameW, SendMessageW, SetWindowPos, HTCAPTION, HWND_BOTTOM, SWP_NOACTIVATE,
                SWP_NOMOVE, SWP_NOSIZE, WM_NCLBUTTONDOWN,
            },
        },
    };

    struct Job(HANDLE);
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    fn job() -> Option<&'static Job> {
        static JOB: OnceLock<Option<Job>> = OnceLock::new();
        JOB.get_or_init(|| unsafe {
            let h = CreateJobObjectW(None, None).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .ok()?;
            Some(Job(h))
        })
        .as_ref()
    }

    /// Children die with us, even if the app crashes.
    pub fn bind_to_job(child: &tokio::process::Child) {
        if let (Some(job), Some(raw)) = (job(), child.raw_handle()) {
            unsafe {
                let _ = AssignProcessToJobObject(job.0, HANDLE(raw));
            }
        }
    }

    unsafe extern "system" fn find_mpv(h: HWND, lp: LPARAM) -> windows::core::BOOL {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(h, &mut buf) as usize;
        if String::from_utf16_lossy(&buf[..n]) == "mpv" {
            *(lp.0 as *mut isize) = h.0 as isize;
            return false.into();
        }
        true.into()
    }

    /// Keeps mpv's video window at the bottom and stops it taking mouse input,
    /// so every click lands on the GPUI layer above it.
    pub fn tame_mpv_window(parent: isize) -> bool {
        let mut found: isize = 0;
        unsafe {
            let _ = EnumChildWindows(Some(HWND(parent as _)), Some(find_mpv), LPARAM(&mut found as *mut isize as isize));
            if found == 0 {
                return false;
            }
            let h = HWND(found as _);
            let _ = SetWindowPos(h, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = EnableWindow(h, false);
        }
        true
    }

    /// Starts a native window move, as if the user grabbed the title bar.
    pub fn begin_window_drag(hwnd: isize) {
        unsafe {
            let _ = ReleaseCapture();
            SendMessageW(HWND(hwnd as _), WM_NCLBUTTONDOWN, Some(WPARAM(HTCAPTION as usize)), Some(LPARAM(0)));
        }
    }

    /// Put files on the clipboard the way Explorer's "Copy" does (CF_HDROP),
    /// so Ctrl+V in Discord attaches them.
    pub fn copy_files_to_clipboard(paths: &[String]) -> anyhow::Result<()> {
        use windows::Win32::{
            System::{
                DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
                Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT},
                Ole::CF_HDROP,
            },
            UI::Shell::DROPFILES,
        };
        let mut wide: Vec<u16> = Vec::new();
        for p in paths {
            wide.extend(p.encode_utf16());
            wide.push(0);
        }
        wide.push(0);
        let header = std::mem::size_of::<DROPFILES>();
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, header + wide.len() * 2)?;
            let p = GlobalLock(h) as *mut u8;
            let df = p as *mut DROPFILES;
            (*df).pFiles = header as u32;
            (*df).fWide = true.into();
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, p.add(header), wide.len() * 2);
            let _ = GlobalUnlock(h);
            let mut opened = false;
            for _ in 0..20 {
                if OpenClipboard(None).is_ok() {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            if !opened {
                anyhow::bail!("clipboard is busy");
            }
            let _ = EmptyClipboard();
            let r = SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(h.0)));
            let _ = CloseClipboard();
            r?;
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::{begin_window_drag, bind_to_job, copy_files_to_clipboard, tame_mpv_window};

pub const VIDEO_EXT: &[&str] = &[
    "mp4", "mkv", "mov", "webm", "avi", "m4v", "ts", "m2ts", "flv", "wmv", "mpg", "mpeg", "3gp", "ogv",
];

/// Registers Clipdat as a video player for the current user (HKCU, no admin needed),
/// so it shows up in Windows Settings → Default apps.
pub fn register_file_types() -> Result<()> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let exe = std::env::current_exe()?.to_string_lossy().into_owned();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey_with_flags("Software\\Classes", winreg::enums::KEY_ALL_ACCESS)?;
    let (prog, _) = classes.create_subkey("Clipdat.Video")?;
    prog.set_value("", &"Video (Clipdat)")?;
    prog.create_subkey("DefaultIcon")?.0.set_value("", &format!("\"{exe}\",0"))?;
    prog.create_subkey("shell\\open\\command")?.0.set_value("", &format!("\"{exe}\" \"%1\""))?;
    let (caps, _) = hkcu.create_subkey("Software\\Clipdat\\Capabilities")?;
    caps.set_value("ApplicationName", &"Clipdat")?;
    caps.set_value("ApplicationDescription", &"Video player and clipper")?;
    let (assoc, _) = caps.create_subkey("FileAssociations")?;
    for ext in VIDEO_EXT {
        assoc.set_value(format!(".{ext}"), &"Clipdat.Video")?;
        let (owp, _) = classes.create_subkey(format!(".{ext}\\OpenWithProgids"))?;
        owp.set_value("Clipdat.Video", &"")?;
    }
    let (app, _) = classes.create_subkey("Applications\\clipdat.exe")?;
    app.create_subkey("shell\\open\\command")?.0.set_value("", &format!("\"{exe}\" \"%1\""))?;
    let (types, _) = app.create_subkey("SupportedTypes")?;
    for ext in VIDEO_EXT {
        types.set_value(format!(".{ext}"), &"")?;
    }
    hkcu.create_subkey("Software\\RegisteredApplications")?.0.set_value("Clipdat", &"Software\\Clipdat\\Capabilities")?;
    Ok(())
}
