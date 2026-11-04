//! Probing, keyframe index and timeline thumbnails (ffprobe / ffmpeg sidecars).

use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::util::sidecar_cmd;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioTrack {
    pub index: usize,
    pub title: String,
    pub codec: String,
    pub channels: u32,
    pub layout: String,
    pub sample_rate: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub path: String,
    pub duration: f64,
    pub start: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub vcodec: String,
    pub pix_fmt: String,
    pub hdr: bool,
    pub bitrate: u64,
    pub size: u64,
    pub container: String,
    pub audio: Vec<AudioTrack>,
}

fn rate(s: &str) -> f64 {
    let mut it = s.split('/');
    let n: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let d: f64 = it.next().and_then(|x| x.parse().ok()).unwrap_or(1.0);
    if d == 0.0 { 0.0 } else { n / d }
}

pub async fn probe(path: &str) -> Result<MediaInfo> {
    let out = sidecar_cmd("ffprobe")?
        .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams", path])
        .output()
        .await?;
    if !out.status.success() {
        return Err(anyhow!("ffprobe: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let j: Value = serde_json::from_slice(&out.stdout)?;
    let streams = j["streams"].as_array().cloned().unwrap_or_default();
    let fmt = &j["format"];
    let v = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
        .ok_or_else(|| anyhow!("no video stream"))?;
    let num = |x: &Value| -> f64 {
        x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
    };
    let mut fps = rate(v["avg_frame_rate"].as_str().unwrap_or("0/1"));
    if !(1.0..=1000.0).contains(&fps) {
        fps = rate(v["r_frame_rate"].as_str().unwrap_or("0/1"));
    }
    let trc = v["color_transfer"].as_str().unwrap_or_default();
    let audio = streams
        .iter()
        .filter(|s| s["codec_type"] == "audio")
        .enumerate()
        .map(|(i, s)| AudioTrack {
            index: i,
            title: s["tags"]["title"]
                .as_str()
                .or(s["tags"]["handler_name"].as_str())
                .unwrap_or_default()
                .to_string(),
            codec: s["codec_name"].as_str().unwrap_or_default().to_string(),
            channels: s["channels"].as_u64().unwrap_or(0) as u32,
            layout: s["channel_layout"].as_str().unwrap_or_default().to_string(),
            sample_rate: num(&s["sample_rate"]) as u32,
        })
        .collect();
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let duration = num(&fmt["duration"]).max(num(&v["duration"]));
    let mut bitrate = num(&fmt["bit_rate"]) as u64;
    if bitrate == 0 && duration > 0.0 {
        bitrate = (size as f64 * 8.0 / duration) as u64;
    }
    Ok(MediaInfo {
        path: path.to_string(),
        duration,
        start: num(&fmt["start_time"]),
        width: v["width"].as_u64().unwrap_or(0) as u32,
        height: v["height"].as_u64().unwrap_or(0) as u32,
        fps,
        vcodec: v["codec_name"].as_str().unwrap_or_default().to_string(),
        pix_fmt: v["pix_fmt"].as_str().unwrap_or_default().to_string(),
        hdr: trc == "smpte2084" || trc == "arib-std-b67",
        bitrate,
        size,
        container: fmt["format_name"].as_str().unwrap_or_default().to_string(),
        audio,
    })
}

/// Presentation times of every video keyframe (reads packets only, no decoding).
pub async fn keyframes(path: &str) -> Result<Vec<f64>> {
    let mut child = sidecar_cmd("ffprobe")?
        .args([
            "-v", "error", "-select_streams", "v:0",
            "-show_entries", "packet=pts_time,flags", "-of", "csv=p=0", path,
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut out = Vec::new();
    while let Some(l) = lines.next_line().await? {
        let mut it = l.split(',');
        if let (Some(t), Some(f)) = (it.next(), it.next()) {
            if f.contains('K') {
                if let Ok(t) = t.parse::<f64>() {
                    out.push(t);
                }
            }
        }
    }
    child.wait().await?;
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out.dedup();
    Ok(out)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Thumb {
    pub t: f64,
    pub path: String,
}

fn cache_key(path: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    if let Ok(m) = std::fs::metadata(path) {
        m.len().hash(&mut h);
        if let Ok(t) = m.modified() {
            t.hash(&mut h);
        }
    }
    format!("{:016x}", h.finish())
}

/// ~`count` evenly spaced thumbnails, decoded from keyframes only. Cached per file.
pub async fn thumbnails(cache_root: &Path, path: &str, duration: f64, count: u32) -> Result<Vec<Thumb>> {
    let dir: PathBuf = cache_root.join("thumbs").join(cache_key(path));
    let index = dir.join("index.json");
    if let Ok(s) = tokio::fs::read_to_string(&index).await {
        if let Ok(v) = serde_json::from_str::<Vec<Thumb>>(&s) {
            if !v.is_empty() {
                return Ok(v);
            }
        }
    }
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await?;
    let interval = (duration / count.max(1) as f64).max(0.5);
    let select = format!("select='isnan(prev_selected_t)+gte(t-prev_selected_t,{interval:.3})'");

    // Try the GPU path first, fall back to CPU decoding.
    let attempts: [(&[&str], String); 2] = [
        (
            &["-hwaccel", "cuda", "-hwaccel_output_format", "cuda"],
            format!("{select},scale_cuda=-2:108,hwdownload,format=nv12,showinfo"),
        ),
        (&[], format!("{select},scale=-2:108:flags=fast_bilinear,showinfo")),
    ];
    for (hw, vf) in attempts.iter() {
        let pattern = dir.join("%04d.jpg");
        let mut cmd = sidecar_cmd("ffmpeg")?;
        cmd.args(["-hide_banner", "-nostats", "-y", "-skip_frame", "nokey"])
            .args(hw.iter())
            .arg("-i")
            .arg(path)
            .args(["-an", "-sn", "-dn", "-vf", vf, "-fps_mode", "vfr", "-q:v", "5"])
            .arg(&pattern)
            .stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn()?;
        let mut lines = BufReader::new(child.stderr.take().unwrap()).lines();
        let mut times = Vec::new();
        while let Some(l) = lines.next_line().await? {
            if l.contains("Parsed_showinfo") {
                if let Some(p) = l.find("pts_time:") {
                    let rest = &l[p + 9..];
                    let num: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
                    if let Ok(t) = num.parse::<f64>() {
                        times.push(t);
                    }
                }
            }
        }
        let ok = child.wait().await.map(|s| s.success()).unwrap_or(false);
        if ok && !times.is_empty() {
            let thumbs: Vec<Thumb> = times
                .iter()
                .enumerate()
                .map(|(i, t)| Thumb {
                    t: *t,
                    path: dir.join(format!("{:04}.jpg", i + 1)).to_string_lossy().into_owned(),
                })
                .filter(|th| Path::new(&th.path).exists())
                .collect();
            tokio::fs::write(&index, serde_json::to_string(&thumbs)?)
                .await
                .context("writing thumbnail index")?;
            return Ok(thumbs);
        }
    }
    Err(anyhow!("could not generate thumbnails"))
}
