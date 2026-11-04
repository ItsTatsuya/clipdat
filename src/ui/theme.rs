//! Design tokens from the approved mockup.

use gpui::{rgba, FontFeatures, Hsla};
use std::sync::Arc;

fn c(hex: u32) -> Hsla {
    rgba(hex).into()
}

pub fn text() -> Hsla { c(0xf4f4f6ff) }
pub fn text2() -> Hsla { c(0xf4f4f6a8) }
pub fn text3() -> Hsla { c(0xf4f4f666) }
pub fn glass() -> Hsla { c(0x16161bd0) }
pub fn glass_strong() -> Hsla { c(0x121216f2) }
pub fn panel() -> Hsla { c(0x111114ff) }
pub fn stroke() -> Hsla { c(0xffffff12) }
pub fn stroke2() -> Hsla { c(0xffffff24) }
pub fn hover() -> Hsla { c(0xffffff17) }
pub fn soft() -> Hsla { c(0xffffff0d) }
pub fn field() -> Hsla { c(0x00000040) }
pub fn accent() -> Hsla { c(0x5b8cffff) }
pub fn clip() -> Hsla { c(0xffb224ff) }
pub fn clip_soft() -> Hsla { c(0xffb22424) }
pub fn on_clip() -> Hsla { c(0x1a1200ff) }
pub fn ok() -> Hsla { c(0x34d399ff) }
pub fn bad() -> Hsla { c(0xff5c5cff) }
pub fn discord() -> Hsla { c(0x5865f2ff) }
pub fn discord_soft() -> Hsla { c(0x5865f224) }
pub fn dim() -> Hsla { c(0x0a0a0ca8) }
pub fn black() -> Hsla { c(0x000000ff) }

pub const FONT: &str = "Inter";

/// Tabular digits so timecodes and sizes don't jitter.
pub fn tnum() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}
