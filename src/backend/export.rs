//! Export jobs: lossless stream copy, NVENC re-encode (fit-to-size / quality),
//! multi-segment merge, size verification + retry, clipboard hand-off.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use super::Event;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    sync::watch,
};

use super::util::{bind_to_job, sidecar_cmd};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Adv {
    pub rc: String,
    pub peak: f64,
    pub multipass: String,
    pub lookahead: u32,
    pub preset: String,
    pub tune: String,
    pub profile: String,
    pub depth: u32,
    pub bframes: u32,
    pub bref: String,
    pub gop_sec: f64,
    pub refs: String,
    pub spatial_aq: bool,
    pub aq_strength: u32,
    pub temporal_aq: bool,
    pub weighted_pred: bool,
    pub scale: String,
    pub crop: String,
    pub hdr: String,
    pub range: String,
    pub vsync: String,
    pub sharpen: f64,
    pub tracks: Vec<usize>,
    pub mix: bool,
    pub acodec: String,
    pub abitrate: u32,
    pub channels: String,
    pub sample_rate: u32,
    pub game_vol: f64,
    pub mic_vol: f64,
    pub loudnorm: bool,
    pub faststart: bool,
    pub keep_meta: bool,
    pub hwdec: bool,
    pub extra: String,
}

impl Default for Adv {
    fn default() -> Self {
        Adv {
            rc: "vbr".into(),
            peak: 1.15,
            multipass: "fullres".into(),
            lookahead: 20,
            preset: "p6".into(),
            tune: "hq".into(),
            profile: "high".into(),
            depth: 8,
            bframes: 3,
            bref: "middle".into(),
            gop_sec: 2.0,
            refs: "auto".into(),
            spatial_aq: true,
            aq_strength: 8,
            temporal_aq: true,
            weighted_pred: false,
            scale: "lanczos".into(),
            crop: "src".into(),
            hdr: "keep".into(),
            range: "auto".into(),
            vsync: "cfr".into(),
            sharpen: 0.0,
            tracks: vec![0, 1],
            mix: true,
            acodec: "aac".into(),
            abitrate: 128,
            channels: "2".into(),
            sample_rate: 48000,
            game_vol: 0.0,
            mic_vol: 0.0,
            loudnorm: false,
            faststart: true,
            keep_meta: false,
            hwdec: true,
            extra: String::new(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExportReq {
    pub input: String,
    pub segments: Vec<[f64; 2]>,
    /// "size" | "quality" | "lossless"
    pub mode: String,
    pub limit_mb: f64,
    /// "h264" | "hevc" | "av1"
    pub codec: String,
    /// 0..100, quality mode only
    pub quality: f64,
    pub height: u32,
    pub fps: f64,
    pub video_kbps: u32,
    pub merge: bool,
    pub out_dir: String,
    pub name: String,
    pub copy_clipboard: bool,
    pub reveal: bool,
    pub src_width: u32,
    pub src_height: u32,
    pub src_fps: f64,
    pub audio_tracks: usize,
    pub adv: Adv,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub id: u64,
    pub state: String,
    pub phase: String,
    pub progress: f64,
    pub fps: f64,
    pub speed: f64,
    pub message: String,
    pub outputs: Vec<String>,
    pub bytes: u64,
}

#[derive(Default)]
pub struct Jobs {
    next: AtomicU64,
    cancel: Mutex<HashMap<u64, watch::Sender<bool>>>,
}

impl Jobs {
    pub fn cancel(&self, id: u64) {
        if let Some(tx) = self.cancel.lock().unwrap().get(&id) {
            let _ = tx.send(true);
        }
    }
}

fn encoder(codec: &str) -> &'static str {
    match codec {
        "hevc" => "hevc_nvenc",
        "av1" => "av1_nvenc",
        _ => "h264_nvenc",
    }
}

fn s<T: ToString>(v: T) -> String {
    v.to_string()
}

/// Video crop (x, y, w, h) for the chosen aspect preset.
fn crop_rect(preset: &str, w: u32, h: u32) -> Option<(u32, u32, u32, u32)> {
    let ratio = match preset {
        "916" => 9.0 / 16.0,
        "11" => 1.0,
        "43" => 4.0 / 3.0,
        "219" => 21.0 / 9.0,
        _ => return None,
    };
    let (w, h) = (w as f64, h as f64);
    let (cw, ch) = if w / h > ratio { (h * ratio, h) } else { (w, w / ratio) };
    let even = |v: f64| ((v / 2.0).floor() * 2.0) as u32;
    let (cw, ch) = (even(cw), even(ch));
    Some((even((w - cw as f64) / 2.0), even((h - ch as f64) / 2.0), cw, ch))
}

/// Audio maps / filters / codec args. Returns (args, number of audio streams written).
fn audio_args(req: &ExportReq) -> (Vec<String>, usize) {
    let a = &req.adv;
    let tracks: Vec<usize> = a.tracks.iter().copied().filter(|t| *t < req.audio_tracks).collect();
    let mut out = vec![s("-map"), s("0:v:0")];
    if a.acodec == "none" || tracks.is_empty() {
        out.push(s("-an"));
        return (out, 0);
    }
    let vol = |t: usize| if t == 0 { a.game_vol } else if t == 1 { a.mic_vol } else { 0.0 };
    let copy = a.acodec == "copy";
    let n_out;
    if a.mix && tracks.len() > 1 && !copy {
        let mut fc = String::new();
        let mut labels = String::new();
        for t in &tracks {
            if vol(*t) != 0.0 {
                fc.push_str(&format!("[0:a:{t}]volume={}dB[v{t}];", vol(*t)));
                labels.push_str(&format!("[v{t}]"));
            } else {
                labels.push_str(&format!("[0:a:{t}]"));
            }
        }
        fc.push_str(&format!("{labels}amix=inputs={}:normalize=0:duration=longest", tracks.len()));
        if a.loudnorm {
            fc.push_str(",loudnorm=I=-14:TP=-1.5:LRA=11");
        }
        fc.push_str("[aout]");
        out.extend([s("-filter_complex"), fc, s("-map"), s("[aout]")]);
        n_out = 1;
    } else {
        for (i, t) in tracks.iter().enumerate() {
            out.extend([s("-map"), format!("0:a:{t}")]);
            if !copy {
                let mut f = Vec::new();
                if vol(*t) != 0.0 {
                    f.push(format!("volume={}dB", vol(*t)));
                }
                if a.loudnorm {
                    f.push(s("loudnorm=I=-14:TP=-1.5:LRA=11"));
                }
                if !f.is_empty() {
                    out.extend([format!("-filter:a:{i}"), f.join(",")]);
                }
            }
        }
        n_out = tracks.len();
    }
    if copy {
        out.extend([s("-c:a"), s("copy")]);
    } else {
        out.extend([s("-c:a"), a.acodec.clone(), s("-b:a"), format!("{}k", a.abitrate)]);
        if a.channels != "src" {
            out.extend([s("-ac"), a.channels.clone()]);
        }
        let sr = if a.acodec == "libopus" { 48000 } else { a.sample_rate };
        out.extend([s("-ar"), s(sr)]);
    }
    (out, n_out)
}

fn tail_args(req: &ExportReq) -> Vec<String> {
    let mut out = Vec::new();
    if req.adv.faststart {
        out.extend([s("-movflags"), s("+faststart")]);
    }
    if !req.adv.keep_meta {
        out.extend([s("-map_metadata"), s("-1"), s("-map_chapters"), s("-1")]);
    }
    out.extend(req.adv.extra.split_whitespace().map(String::from));
    out
}

fn input_args(seg: [f64; 2], input: &str) -> Vec<String> {
    vec![s("-ss"), format!("{:.3}", seg[0]), s("-to"), format!("{:.3}", seg[1]), s("-i"), s(input)]
}

/// Full argument list for one segment.
pub fn segment_args(req: &ExportReq, seg: [f64; 2], out: &str, video_kbps: u32) -> Vec<String> {
    let a = &req.adv;
    let mut args = vec![s("-hide_banner"), s("-y"), s("-nostats"), s("-progress"), s("pipe:1")];

    if req.mode == "lossless" {
        args.extend(input_args(seg, &req.input));
        args.extend(audio_args(req).0);
        args.extend([s("-c:v"), s("copy"), s("-avoid_negative_ts"), s("make_zero")]);
        args.extend(tail_args(req));
        args.push(s(out));
        return args;
    }

    let crop = crop_rect(&a.crop, req.src_width, req.src_height);
    let cpu_filters = crop.is_some() || a.hdr == "tonemap" || a.sharpen > 0.0;
    let gpu_frames = a.hwdec && !cpu_filters;
    if a.hwdec {
        args.extend([s("-hwaccel"), s("cuda")]);
        if gpu_frames {
            args.extend([s("-hwaccel_output_format"), s("cuda")]);
        }
    }
    args.extend(input_args(seg, &req.input));

    let ten_bit = a.depth == 10 && req.codec != "h264";
    let scale_needed = req.height > 0 && req.height < req.src_height;
    let fps_needed = req.fps > 0.0 && req.fps < req.src_fps - 0.5;
    let mut vf: Vec<String> = Vec::new();
    if gpu_frames {
        if scale_needed || ten_bit {
            let h = if scale_needed { req.height } else { req.src_height };
            let mut f = format!("scale_cuda=w=-2:h={h}:interp_algo={}", a.scale);
            if ten_bit {
                f.push_str(":format=p010le");
            }
            vf.push(f);
        }
    } else {
        if let Some((x, y, w, h)) = crop {
            vf.push(format!("crop={w}:{h}:{x}:{y}"));
        }
        if a.hdr == "tonemap" {
            vf.push(s("zscale=t=linear:npl=100,format=gbrpf32le,zscale=p=bt709,tonemap=hable:desat=0,zscale=t=bt709:m=bt709:r=tv"));
        }
        if scale_needed {
            vf.push(format!("scale=-2:{}:flags={}", req.height, a.scale));
        }
        if a.sharpen > 0.0 {
            vf.push(format!("unsharp=5:5:{:.2}", a.sharpen));
        }
        vf.push(s(if ten_bit { "format=p010le" } else { "format=yuv420p" }));
    }
    if fps_needed {
        vf.push(format!("fps={}", req.fps));
    }
    if !vf.is_empty() {
        args.extend([s("-vf"), vf.join(",")]);
    }

    let (aargs, _) = audio_args(req);
    args.extend(aargs);

    args.extend([s("-c:v"), s(encoder(&req.codec)), s("-preset"), a.preset.clone(), s("-tune"), a.tune.clone()]);
    // Only pass a profile the encoder accepts; 10-bit HEVC needs Main 10.
    let profile = match req.codec.as_str() {
        "hevc" if ten_bit => Some("main10"),
        "hevc" => Some(if a.profile == "main10" { "main10" } else { "main" }),
        "av1" => None, // av1_nvenc has no -profile option
        _ => ["high", "main", "baseline"].into_iter().find(|p| *p == a.profile).or(Some("high")),
    };
    if let Some(p) = profile {
        args.extend([s("-profile:v"), s(p)]);
    }
    match (req.mode.as_str(), a.rc.as_str()) {
        ("size", "cbr") | ("quality", "cbr") => {
            args.extend([s("-rc"), s("cbr"), s("-b:v"), format!("{video_kbps}k"), s("-bufsize"), format!("{}k", video_kbps * 2)]);
        }
        ("size", _) | ("quality", "vbr") => {
            let max = (video_kbps as f64 * a.peak).round() as u32;
            args.extend([
                s("-rc"), s("vbr"), s("-b:v"), format!("{video_kbps}k"),
                s("-maxrate"), format!("{max}k"), s("-bufsize"), format!("{}k", video_kbps * 2),
            ]);
        }
        (_, "qp") => {
            let qp = (34.0 - req.quality * 0.18).round() as i32;
            args.extend([s("-rc"), s("constqp"), s("-qp"), s(qp)]);
        }
        _ => {
            let cq = (34.0 - req.quality * 0.18).round() as i32;
            args.extend([s("-rc"), s("vbr"), s("-cq"), s(cq), s("-b:v"), s("0")]);
        }
    }
    args.extend([s("-multipass"), a.multipass.clone()]);
    if a.lookahead > 0 {
        args.extend([s("-rc-lookahead"), s(a.lookahead)]);
    }
    if a.weighted_pred {
        args.extend([s("-weighted_pred"), s("1"), s("-bf"), s("0")]);
    } else {
        args.extend([s("-bf"), s(a.bframes)]);
        if a.bframes > 0 {
            args.extend([s("-b_ref_mode"), a.bref.clone()]);
        }
    }
    let out_fps = if fps_needed { req.fps } else { req.src_fps.max(1.0) };
    args.extend([s("-g"), s((a.gop_sec * out_fps).round() as u32)]);
    if a.refs != "auto" {
        args.extend([s("-refs"), a.refs.clone()]);
    }
    if a.spatial_aq {
        args.extend([s("-spatial-aq"), s("1"), s("-aq-strength"), s(a.aq_strength)]);
    }
    if a.temporal_aq {
        args.extend([s("-temporal-aq"), s("1")]);
    }
    if a.range != "auto" {
        args.extend([s("-color_range"), a.range.clone()]);
    }
    args.extend([s("-fps_mode"), s(if a.vsync == "vfr" { "passthrough" } else { "cfr" })]);
    args.extend(tail_args(req));
    args.push(s(out));
    args
}

/// The command shown in the Advanced panel (first segment).
pub fn preview(req: &ExportReq) -> String {
    let seg = req.segments.first().copied().unwrap_or([0.0, 0.0]);
    let out = Path::new(&req.out_dir).join(format!("{}.mp4", req.name));
    let args = segment_args(req, seg, &out.to_string_lossy(), req.video_kbps);
    let q = |a: &String| {
        if a.contains(' ') || a.contains(';') || a.contains('[') || a.contains('\'') {
            format!("\"{a}\"")
        } else {
            a.clone()
        }
    };
    let mut s = String::from("ffmpeg");
    for a in args.iter().filter(|a| !matches!(a.as_str(), "-hide_banner" | "-nostats" | "-y")) {
        if a == "-progress" || a == "pipe:1" {
            continue;
        }
        s.push(' ');
        s.push_str(&q(a));
    }
    s
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let clean: String = name
        .chars()
        .map(|c| if r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    let mut p = dir.join(format!("{clean}.mp4"));
    let mut n = 2;
    while p.exists() {
        p = dir.join(format!("{clean} ({n}).mp4"));
        n += 1;
    }
    p
}

/// Runs ffmpeg, reporting `out_time` progress in seconds.
async fn run_ffmpeg(
    args: &[String],
    cancel: &mut watch::Receiver<bool>,
    mut on_progress: impl FnMut(f64, f64, f64),
) -> Result<()> {
    let mut child = sidecar_cmd("ffmpeg")?
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    bind_to_job(&child);
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let err_task = tokio::spawn(async move {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf).await;
        buf
    });
    let mut lines = BufReader::new(stdout).lines();
    let (mut fps, mut speed) = (0.0, 0.0);
    let run = async {
        while let Ok(Some(l)) = lines.next_line().await {
            if let Some((k, v)) = l.split_once('=') {
                match k {
                    "fps" => fps = v.trim().parse().unwrap_or(fps),
                    "speed" => speed = v.trim().trim_end_matches('x').parse().unwrap_or(speed),
                    "out_time_us" | "out_time_ms" => {
                        if let Ok(us) = v.trim().parse::<f64>() {
                            on_progress(us / 1_000_000.0, fps, speed);
                        }
                    }
                    _ => {}
                }
            }
        }
        child.wait().await
    };
    tokio::select! {
        status = run => {
            let status = status?;
            if !status.success() {
                let err = err_task.await.unwrap_or_default();
                let tail: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
                let tail = tail[tail.len().saturating_sub(4)..].join("\n");
                bail!("ffmpeg failed: {tail}");
            }
            Ok(())
        }
        _ = cancel.changed() => {
            bail!("cancelled")
        }
    }
}

pub fn start(ui: smol::channel::Sender<Event>, jobs: &'static Jobs, req: ExportReq) -> Result<u64> {
    if req.segments.is_empty() {
        bail!("nothing to export");
    }
    let id = jobs.next.fetch_add(1, Ordering::Relaxed) + 1;
    let (tx, rx) = watch::channel(false);
    jobs.cancel.lock().unwrap().insert(id, tx);
    tokio::spawn(async move {
        let emit = |e: JobEvent| {
            let _ = ui.try_send(Event::Job(e));
        };
        let result = run_job(id, &req, rx, &emit).await;
        jobs.cancel.lock().unwrap().remove(&id);
        match result {
            Ok((outputs, bytes)) => {
                let mut msg = String::new();
                if req.copy_clipboard {
                    let o = outputs.clone();
                    match tokio::task::spawn_blocking(move || super::util::copy_files_to_clipboard(&o)).await {
                        Ok(Ok(())) => msg = "copied".into(),
                        _ => msg = "clipboard failed".into(),
                    }
                }
                if req.reveal {
                    if let Some(f) = outputs.first() {
                        super::util::reveal_in_explorer(f);
                    }
                }
                emit(JobEvent { id, state: "done".into(), phase: "done".into(), progress: 1.0, outputs, bytes, message: msg, ..Default::default() });
            }
            Err(e) => {
                let cancelled = e.to_string() == "cancelled";
                emit(JobEvent {
                    id,
                    state: if cancelled { "cancelled" } else { "error" }.into(),
                    message: e.to_string(),
                    ..Default::default()
                });
            }
        }
    });
    Ok(id)
}

async fn run_job(
    id: u64,
    req: &ExportReq,
    mut cancel: watch::Receiver<bool>,
    emit: &impl Fn(JobEvent),
) -> Result<(Vec<String>, u64)> {
    let out_dir = PathBuf::from(&req.out_dir);
    tokio::fs::create_dir_all(&out_dir).await?;
    let tmp = std::env::temp_dir().join(format!("clipdat-job-{}-{id}", std::process::id()));
    tokio::fs::create_dir_all(&tmp).await?;
    let total: f64 = req.segments.iter().map(|s| (s[1] - s[0]).max(0.01)).sum();
    let merge = req.merge && req.segments.len() > 1;
    let limit_bytes = (req.limit_mb * 1_000_000.0) as u64;
    let phase_label = if req.mode == "lossless" { "copying" } else { "encoding" };

    // Final destinations.
    let finals: Vec<PathBuf> = if merge || req.segments.len() == 1 {
        vec![unique_path(&out_dir, &req.name)]
    } else {
        let mut v = Vec::new();
        for i in 0..req.segments.len() {
            v.push(unique_path(&out_dir, &format!("{}_{}", req.name, i + 1)));
        }
        v
    };

    let mut kbps = req.video_kbps.max(100);
    let max_attempts = if req.mode == "size" { 2 } else { 1 };
    let mut result = Err(anyhow!("not run"));
    for attempt in 0..max_attempts {
        let phase = if attempt == 0 { phase_label } else { "re-encoding" };
        let mut done = 0.0;
        let mut seg_files = Vec::new();
        for (i, seg) in req.segments.iter().enumerate() {
            let target = if merge { tmp.join(format!("seg{i}.mp4")) } else { finals[i].clone() };
            let args = segment_args(req, *seg, &target.to_string_lossy(), kbps);
            let seg_len = (seg[1] - seg[0]).max(0.01);
            run_ffmpeg(&args, &mut cancel, |t, fps, speed| {
                let p = ((done + t.min(seg_len)) / total).clamp(0.0, 1.0);
                emit(JobEvent { id, state: "running".into(), phase: phase.into(), progress: p * if merge { 0.97 } else { 1.0 }, fps, speed, ..Default::default() });
            })
            .await?;
            done += seg_len;
            seg_files.push(target);
        }
        if merge {
            emit(JobEvent { id, state: "running".into(), phase: "merging".into(), progress: 0.98, ..Default::default() });
            let list = tmp.join("list.txt");
            let body: String = seg_files
                .iter()
                .map(|p| format!("file '{}'\n", p.to_string_lossy().replace('\'', "'\\''")))
                .collect();
            tokio::fs::write(&list, body).await?;
            let mut args = vec![
                s("-hide_banner"), s("-y"), s("-nostats"), s("-f"), s("concat"), s("-safe"), s("0"),
                s("-i"), list.to_string_lossy().into_owned(), s("-map"), s("0"), s("-c"), s("copy"),
            ];
            if req.adv.faststart {
                args.extend([s("-movflags"), s("+faststart")]);
            }
            args.push(finals[0].to_string_lossy().into_owned());
            run_ffmpeg(&args, &mut cancel, |_, _, _| {}).await?;
        }

        // Size check: never hand over a file that's too big.
        let sizes: Vec<u64> = finals.iter().map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)).collect();
        let worst = sizes.iter().copied().max().unwrap_or(0);
        result = Ok((finals.iter().map(|p| p.to_string_lossy().into_owned()).collect(), sizes.iter().sum()));
        if req.mode != "size" || worst <= limit_bytes {
            break;
        }
        if attempt + 1 < max_attempts {
            let ratio = limit_bytes as f64 / worst as f64 * 0.95;
            kbps = ((kbps as f64) * ratio).max(100.0) as u32;
        } else {
            result = Err(anyhow!(
                "Still {:.1} MB after a retry, over the {} MB limit. Trim the clip or pick a bigger limit.",
                worst as f64 / 1e6,
                req.limit_mb
            ));
        }
    }
    let _ = tokio::fs::remove_dir_all(&tmp).await;
    if result.is_err() {
        for f in &finals {
            let _ = tokio::fs::remove_file(f).await;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    //! Real encodes against a sample file. Run with:
    //! CLIPDAT_BIN_DIR=binaries CLIPDAT_SAMPLE=path/to/sample.mkv cargo test --release -- --nocapture --test-threads=1
    use super::*;

    fn sample() -> Option<String> {
        std::env::var("CLIPDAT_SAMPLE").ok().filter(|p| Path::new(p).exists())
    }

    async fn req(mode: &str, codec: &str, segs: Vec<[f64; 2]>, name: &str) -> ExportReq {
        let input = sample().unwrap();
        let info = crate::backend::media::probe(&input).await.unwrap();
        ExportReq {
            input,
            segments: segs,
            mode: mode.into(),
            limit_mb: 20.0,
            codec: codec.into(),
            quality: 68.0,
            height: 1080,
            fps: 60.0,
            video_kbps: 0,
            merge: true,
            out_dir: std::env::temp_dir().join("clipdat-tests").to_string_lossy().into_owned(),
            name: name.into(),
            copy_clipboard: false,
            reveal: false,
            src_width: info.width,
            src_height: info.height,
            src_fps: info.fps,
            audio_tracks: info.audio.len(),
            adv: Adv::default(),
        }
    }

    async fn run(r: &ExportReq) -> (Vec<String>, u64) {
        let (_tx, rx) = watch::channel(false);
        let t = std::time::Instant::now();
        let out = run_job(1, r, rx, &|e: JobEvent| {
            if e.phase != "encoding" && e.phase != "copying" {
                eprintln!("  phase {} {:.0}%", e.phase, e.progress * 100.0);
            }
        })
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", r.name));
        eprintln!("{}: {:?} {:.2} MB in {:.1}s", r.name, out.0, out.1 as f64 / 1e6, t.elapsed().as_secs_f64());
        out
    }

    #[tokio::test]
    async fn fit_to_size_merged_h264() {
        if sample().is_none() { return; }
        let mut r = req("size", "h264", vec![[2.0, 14.0], [20.0, 36.0]], "t_size_h264").await;
        // 28 s total at 20 MB: same math as the UI planner.
        r.video_kbps = (20.0 * 8000.0 * 0.96 / 28.0 - 128.0) as u32;
        let (files, bytes) = run(&r).await;
        assert_eq!(files.len(), 1);
        assert!(bytes <= 20_000_000, "over limit: {bytes}");
        let info = crate::backend::media::probe(&files[0]).await.unwrap();
        assert_eq!(info.height, 1080);
        assert_eq!(info.audio.len(), 1, "tracks should be mixed into one");
        assert!((info.duration - 28.0).abs() < 0.5, "duration {}", info.duration);
    }

    #[tokio::test]
    async fn lossless_copy() {
        if sample().is_none() { return; }
        let r = req("lossless", "h264", vec![[5.3, 15.0]], "t_lossless").await;
        let (files, _) = run(&r).await;
        let info = crate::backend::media::probe(&files[0]).await.unwrap();
        assert_eq!(info.height, 1440);
        assert!(info.duration > 9.0 && info.duration < 12.5, "duration {}", info.duration);
    }

    #[tokio::test]
    async fn quality_hevc10_vertical_crop() {
        if sample().is_none() { return; }
        let mut r = req("quality", "hevc", vec![[0.0, 6.0]], "t_hevc10_916").await;
        r.adv.rc = "cq".into();
        r.adv.depth = 10;
        r.adv.profile = "main10".into();
        r.adv.crop = "916".into();
        r.height = 1280;
        let (files, _) = run(&r).await;
        let info = crate::backend::media::probe(&files[0]).await.unwrap();
        assert_eq!(info.vcodec, "hevc");
        assert!(info.pix_fmt.contains("10"), "pix_fmt {}", info.pix_fmt);
        assert!(info.height > info.width, "should be vertical: {}x{}", info.width, info.height);
    }

    #[tokio::test]
    async fn av1_separate_files() {
        if sample().is_none() { return; }
        let mut r = req("size", "av1", vec![[1.0, 5.0], [10.0, 14.0]], "t_av1_sep").await;
        r.merge = false;
        r.limit_mb = 10.0;
        r.video_kbps = (10.0 * 8000.0 * 0.96 / 4.0 - 128.0) as u32;
        let (files, _) = run(&r).await;
        assert_eq!(files.len(), 2);
        for f in &files {
            assert!(std::fs::metadata(f).unwrap().len() <= 10_000_000);
            assert_eq!(crate::backend::media::probe(f).await.unwrap().vcodec, "av1");
        }
    }
}
