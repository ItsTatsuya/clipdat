use std::path::PathBuf;

use anyhow::{anyhow, Result};

/// Sidecar binaries are copied next to the app executable by Tauri (dev and bundle).
pub fn sidecar(name: &str) -> Result<PathBuf> {
    // Tests run from target/debug/deps, so they point at the binaries folder directly.
    if let Ok(dir) = std::env::var("CLIPER_BIN_DIR") {
        let p = PathBuf::from(dir).join(format!("{name}-x86_64-pc-windows-msvc.exe"));
        if p.exists() {
            return Ok(p);
        }
    }
    let dir = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow!("no exe dir"))?
        .to_path_buf();
    let p = dir.join(format!("{name}.exe"));
    if p.exists() {
        Ok(p)
    } else {
        Err(anyhow!("{} not found (run scripts/fetch-deps.ps1)", p.display()))
    }
}

/// A command for a sidecar that never flashes a console window.
pub fn sidecar_cmd(name: &str) -> Result<tokio::process::Command> {
    let mut c = tokio::process::Command::new(sidecar(name)?);
    c.creation_flags(0x0800_0000).kill_on_drop(true);
    Ok(c)
}

#[cfg(windows)]
mod win {
    use std::sync::OnceLock;
    use windows::Win32::{
        Foundation::{HANDLE, HWND, LPARAM},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
        UI::WindowsAndMessaging::{
            EnumChildWindows, GetClassNameW, SetWindowPos, HWND_BOTTOM, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOSIZE,
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

    pub fn push_mpv_window_to_back(parent: isize) -> bool {
        let mut found: isize = 0;
        unsafe {
            let _ = EnumChildWindows(
                Some(HWND(parent as _)),
                Some(find_mpv),
                LPARAM(&mut found as *mut isize as isize),
            );
            if found == 0 {
                return false;
            }
            let _ = SetWindowPos(
                HWND(found as _),
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
        true
    }

    /// Put a file on the clipboard the way Explorer's "Copy" does (CF_HDROP),
    /// so Ctrl+V in Discord attaches it.
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
pub use win::{bind_to_job, copy_files_to_clipboard, push_mpv_window_to_back};

pub fn reveal_in_explorer(path: &str) {
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.replace('/', "\\")))
        .spawn();
}
