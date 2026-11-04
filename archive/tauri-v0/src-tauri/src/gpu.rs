//! Hardware detection: GPU names via DXGI, working encoders via 1-frame test encodes.

use serde::Serialize;

use crate::util::sidecar_cmd;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub adapters: Vec<String>,
    pub encoders: Vec<String>,
}

const CANDIDATES: &[&str] = &[
    "h264_nvenc", "hevc_nvenc", "av1_nvenc",
    "h264_amf", "hevc_amf", "av1_amf",
    "h264_qsv", "hevc_qsv", "av1_qsv",
];

#[cfg(windows)]
fn adapters() -> Vec<String> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    let mut out = Vec::new();
    unsafe {
        let Ok(f) = CreateDXGIFactory1::<IDXGIFactory1>() else { return out };
        let mut i = 0;
        while let Ok(a) = f.EnumAdapters1(i) {
            i += 1;
            if let Ok(d) = a.GetDesc1() {
                if d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                    continue;
                }
                let n = d.Description.iter().position(|c| *c == 0).unwrap_or(d.Description.len());
                let name = String::from_utf16_lossy(&d.Description[..n]);
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    }
    out
}

async fn works(enc: &'static str) -> bool {
    let Ok(mut c) = sidecar_cmd("ffmpeg") else { return false };
    c.args([
        "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=black:s=256x256:r=30",
        "-frames:v", "3", "-c:v", enc, "-f", "null", "-",
    ])
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
    matches!(tokio::time::timeout(std::time::Duration::from_secs(8), c.status()).await, Ok(Ok(s)) if s.success())
}

pub async fn detect() -> GpuInfo {
    let tasks: Vec<_> = CANDIDATES.iter().map(|e| (e, tokio::spawn(works(e)))).collect();
    let mut encoders = Vec::new();
    for (e, t) in tasks {
        if t.await.unwrap_or(false) {
            encoders.push(e.to_string());
        }
    }
    GpuInfo { adapters: adapters(), encoders }
}
