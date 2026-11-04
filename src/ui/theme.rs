//! Design tokens: a strictly monochrome palette (white / greys on near-black).
//! Emphasis comes from brightness and weight, never hue.

use gpui::{rgba, FontFeatures, Hsla};
use std::sync::Arc;

fn c(hex: u32) -> Hsla {
    rgba(hex).into()
}

pub fn text() -> Hsla { c(0xf5f5f5ff) }
pub fn text2() -> Hsla { c(0xf5f5f5a8) }
pub fn text3() -> Hsla { c(0xf5f5f566) }
pub fn glass() -> Hsla { c(0x141414d0) }
pub fn glass_strong() -> Hsla { c(0x111111f2) }
pub fn panel() -> Hsla { c(0x0f0f0fff) }
pub fn stroke() -> Hsla { c(0xffffff12) }
pub fn stroke2() -> Hsla { c(0xffffff24) }
pub fn hover() -> Hsla { c(0xffffff17) }
pub fn soft() -> Hsla { c(0xffffff0d) }
pub fn field() -> Hsla { c(0x00000040) }
/// Focus rings, links, selected checks: plain white.
pub fn accent() -> Hsla { c(0xf5f5f5ff) }
/// The clip selection / primary button: solid white with black content on top.
pub fn clip() -> Hsla { c(0xffffffff) }
pub fn clip_soft() -> Hsla { c(0xffffff1f) }
pub fn on_clip() -> Hsla { c(0x0a0a0aff) }
/// "Fine" is a calm mid-grey; "over the limit / error" is full white and paired with a warn icon.
pub fn ok() -> Hsla { c(0xa3a3a3ff) }
pub fn bad() -> Hsla { c(0xffffffff) }
pub fn discord() -> Hsla { c(0xf5f5f5ff) }
pub fn discord_soft() -> Hsla { c(0xffffff14) }
pub fn dim() -> Hsla { c(0x0a0a0aa8) }
pub fn black() -> Hsla { c(0x000000ff) }
pub fn code_bg() -> Hsla { c(0x0a0a0aff) }
pub fn menu_bg() -> Hsla { c(0x1a1a1afa) }

pub const FONT: &str = "Inter";

/// Tabular digits so timecodes and sizes don't jitter.
pub fn tnum() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}
