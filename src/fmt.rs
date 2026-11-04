//! Small formatting helpers shared by the UI.

use crate::backend::media::Thumb;

/// 1:23 / 1:02:03
pub fn short(t: f64) -> String {
    let t = t.max(0.0) as u64;
    let (h, m, s) = (t / 3600, (t % 3600) / 60, t % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// 01:23.45 (hours prefixed when needed)
pub fn long(t: f64) -> String {
    let t = t.max(0.0);
    let h = (t / 3600.0).floor() as u64;
    let m = ((t % 3600.0) / 60.0).floor() as u64;
    let s = t % 60.0;
    if h > 0 { format!("{h}:{m:02}:{s:05.2}") } else { format!("{m:02}:{s:05.2}") }
}

/// One decimal below 100 MB, truncated so 19.96 never reads as "20.0" against a 20 MB limit.
pub fn mb(v: f64) -> String {
    if v < 100.0 { format!("{:.1}", (v * 10.0).floor() / 10.0) } else { format!("{}", v.round()) }
}

pub fn basename(p: &str) -> String {
    p.rsplit(['\\', '/']).next().unwrap_or(p).to_string()
}

pub fn stem(p: &str) -> String {
    let b = basename(p);
    match b.rfind('.') {
        Some(i) if i > 0 => b[..i].to_string(),
        _ => b,
    }
}

fn local_now() -> (u16, u16, u16, u16, u16, u16) {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    (t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

/// Local date, YYYY-MM-DD.
pub fn date(_unix: u64) -> String {
    let (y, m, d, ..) = local_now();
    format!("{y:04}-{m:02}-{d:02}")
}

/// Local time, HH-MM-SS (file-name safe).
pub fn time_of_day(_unix: u64) -> String {
    let (.., h, mi, s) = local_now();
    format!("{h:02}-{mi:02}-{s:02}")
}

pub fn nearest(thumbs: &[Thumb], t: f64) -> Option<&Thumb> {
    if thumbs.is_empty() {
        return None;
    }
    let i = thumbs.partition_point(|x| x.t <= t);
    let cand = [i.checked_sub(1), (i < thumbs.len()).then_some(i)];
    cand.into_iter().flatten().map(|k| &thumbs[k]).min_by(|a, b| (a.t - t).abs().partial_cmp(&(b.t - t).abs()).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(short(83.4), "1:23");
        assert_eq!(short(3723.0), "1:02:03");
        assert_eq!(long(83.456), "01:23.46");
        assert_eq!(mb(19.96), "19.9");
        assert_eq!(mb(5.04), "5.0");
        assert_eq!(stem(r"C:\a\b\Ranked Match.mkv"), "Ranked Match");
    }
}
