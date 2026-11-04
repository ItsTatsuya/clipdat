//! Fit-to-size planning: the best resolution/fps that keeps a clip under a size limit.

use crate::backend::settings::Codec;

/// Minimum bits-per-pixel that still looks good, per codec.
fn bpp(c: Codec) -> f64 {
    match c {
        Codec::H264 => 0.040,
        Codec::Hevc => 0.028,
        Codec::Av1 => 0.022,
    }
}

const LADDER: &[(u32, u32)] = &[(2160, 60), (1440, 60), (1080, 60), (720, 60), (720, 30), (480, 30)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub ok: bool,
    pub height: u32,
    pub fps: u32,
    pub video_kbps: u32,
    pub size_mb: f64,
}

/// Resolution drops before frame rate: gaming clips want 60 fps.
pub fn plan(duration: f64, limit_mb: f64, codec: Codec, src_w: u32, src_h: u32, src_fps: f64, audio_kbps: u32) -> Plan {
    let dur = duration.max(0.1);
    let total_k = limit_mb * 8.0 * 1000.0 * 0.96 / dur;
    let video_kbps = (total_k - audio_kbps as f64).round();
    let aspect = if src_w > 0 && src_h > 0 { src_w as f64 / src_h as f64 } else { 16.0 / 9.0 };
    let src_h = if src_h > 0 { src_h } else { 1080 };
    let src_fps = if src_fps > 0.0 { src_fps.round() as u32 } else { 60 };
    let mut rungs: Vec<(u32, u32)> = LADDER.iter().filter(|(h, _)| *h <= src_h).map(|(h, f)| (*h, (*f).min(src_fps))).collect();
    if rungs.first().map(|r| r.0 < src_h).unwrap_or(true) {
        rungs.insert(0, (src_h, 60.min(src_fps)));
    }
    let pick = rungs.into_iter().find(|(h, f)| {
        let w = ((*h as f64 * aspect) / 2.0).round() * 2.0;
        video_kbps * 1000.0 / (w * *h as f64 * *f as f64) >= bpp(codec)
    });
    Plan {
        ok: pick.is_some() && video_kbps > 300.0,
        height: pick.map(|p| p.0).unwrap_or(480),
        fps: pick.map(|p| p.1).unwrap_or(30),
        video_kbps: video_kbps.max(100.0) as u32,
        size_mb: limit_mb * 0.95,
    }
}

/// Bitrate (kbps) used for the quality slider when a bitrate-based rate control is chosen.
pub fn quality_kbps(quality: f64, height: u32) -> u32 {
    ((4.0 + quality * 0.32) * 1000.0 * (height.max(1) as f64 / 1080.0).powf(1.4)).round() as u32
}

/// Rough size (MB) for constant-quality mode.
pub fn estimate_quality_mb(duration: f64, quality: f64, codec: Codec, height: u32) -> f64 {
    let factor = match codec {
        Codec::H264 => 1.0,
        Codec::Hevc => 0.7,
        Codec::Av1 => 0.6,
    };
    duration * quality_kbps(quality, height) as f64 / 1000.0 * factor / 8.0
}

pub fn cq_for(q: f64) -> i32 {
    (34.0 - q * 0.18).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thirty_seconds_at_20mb_is_1080p60_h264() {
        let p = plan(30.0, 20.0, Codec::H264, 2560, 1440, 144.0, 128);
        assert!(p.ok);
        assert_eq!((p.height, p.fps), (1080, 60));
    }

    #[test]
    fn long_clip_drops_resolution_before_fps() {
        // 45 s: 1080p60 is too starved, so resolution drops first and 60 fps is kept.
        let p = plan(45.0, 20.0, Codec::H264, 2560, 1440, 144.0, 128);
        assert_eq!((p.height, p.fps), (720, 60));
        // 90 s: even 720p60 is starved, so the ladder falls back to 720p30.
        let p = plan(90.0, 20.0, Codec::H264, 2560, 1440, 144.0, 128);
        assert_eq!((p.height, p.fps), (720, 30));
    }

    #[test]
    fn hopeless_clip_is_rejected() {
        assert!(!plan(900.0, 10.0, Codec::H264, 1920, 1080, 60.0, 128).ok);
    }
}
