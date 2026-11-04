//! Bold, rounded icon set (from the approved design), served as `icons/<name>.svg`,
//! with gpui-component's bundled icons as a fallback.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

fn body(name: &str) -> Option<&'static str> {
    Some(match name {
        "play" => r#"<path d="M7.5 4.6c0-1.1 1.2-1.8 2.2-1.2l10.3 6.9c.9.6.9 1.9 0 2.5L9.7 19.6c-1 .6-2.2-.1-2.2-1.2z" fill="white" stroke="none"/>"#,
        "pause" => r#"<rect x="5.5" y="4" width="4.6" height="16" rx="1.6" fill="white" stroke="none"/><rect x="13.9" y="4" width="4.6" height="16" rx="1.6" fill="white" stroke="none"/>"#,
        "rew" => r#"<path d="M11 6.2v11.6c0 .9-1 1.4-1.7.8L2.6 13c-.6-.5-.6-1.5 0-2l6.7-5.6c.7-.6 1.7-.1 1.7.8zM21.5 6.2v11.6c0 .9-1 1.4-1.7.8L13.1 13c-.6-.5-.6-1.5 0-2l6.7-5.6c.7-.6 1.7-.1 1.7.8z" fill="white" stroke="none"/>"#,
        "ff" => r#"<path d="M13 6.2v11.6c0 .9 1 1.4 1.7.8l6.7-5.6c.6-.5.6-1.5 0-2l-6.7-5.6c-.7-.6-1.7-.1-1.7.8zM2.5 6.2v11.6c0 .9 1 1.4 1.7.8l6.7-5.6c.6-.5.6-1.5 0-2L4.2 5.4c-.7-.6-1.7-.1-1.7.8z" fill="white" stroke="none"/>"#,
        "stepb" => r#"<path d="M19 6.3v11.4c0 1-1.1 1.6-1.9 1L9.4 13.1c-.7-.5-.7-1.6 0-2.1l7.7-5.7c.8-.6 1.9 0 1.9 1z" fill="white" stroke="none"/><path d="M5.5 5.5v13"/>"#,
        "stepf" => r#"<path d="M5 6.3v11.4c0 1 1.1 1.6 1.9 1l7.7-5.7c.7-.5.7-1.6 0-2.1L6.9 5.3C6.1 4.7 5 5.3 5 6.3z" fill="white" stroke="none"/><path d="M18.5 5.5v13"/>"#,
        "vol" => r#"<path d="M3 9.5v5c0 .6.4 1 1 1h3l4.4 3.9c.6.5 1.6.1 1.6-.8V5.4c0-.8-1-1.3-1.6-.7L7 8.5H4c-.6 0-1 .4-1 1z" fill="white" stroke="none"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19.5 5.5a9.5 9.5 0 0 1 0 13"/>"#,
        "mute" => r#"<path d="M3 9.5v5c0 .6.4 1 1 1h3l4.4 3.9c.6.5 1.6.1 1.6-.8V5.4c0-.8-1-1.3-1.6-.7L7 8.5H4c-.6 0-1 .4-1 1z" fill="white" stroke="none"/><path d="M16.5 9.5l5 5M21.5 9.5l-5 5"/>"#,
        "scissors" => r#"<circle cx="6" cy="6.5" r="3"/><circle cx="6" cy="17.5" r="3"/><path d="M20 4.5 8.4 15.2M14.6 13.4 20 19.5M8.4 8.8 11.6 11.6"/>"#,
        "gear" => r#"<path d="M12 2.8l2 1.5 2.5-.4 1.1 2.3 2.3 1.1-.4 2.5 1.5 2-1.5 2 .4 2.5-2.3 1.1-1.1 2.3-2.5-.4-2 1.5-2-1.5-2.5.4-1.1-2.3-2.3-1.1.4-2.5-1.5-2 1.5-2-.4-2.5 2.3-1.1 1.1-2.3 2.5.4z"/><circle cx="12" cy="12" r="3"/>"#,
        "full" => r#"<path d="M4 9V5.5C4 4.7 4.7 4 5.5 4H9M15 4h3.5c.8 0 1.5.7 1.5 1.5V9M20 15v3.5c0 .8-.7 1.5-1.5 1.5H15M9 20H5.5C4.7 20 4 19.3 4 18.5V15"/>"#,
        "unfull" => r#"<path d="M9 4v3.5C9 8.3 8.3 9 7.5 9H4M20 9h-3.5C15.7 9 15 8.3 15 7.5V4M15 20v-3.5c0-.8.7-1.5 1.5-1.5H20M4 15h3.5c.8 0 1.5.7 1.5 1.5V20"/>"#,
        "x" => r#"<path d="M17.5 6.5l-11 11M6.5 6.5l11 11"/>"#,
        "export" => r#"<path d="M12 15V3.8M7.5 8.2 12 3.8l4.5 4.4"/><path d="M4 14.5v3.8c0 1 .8 1.7 1.7 1.7h12.6c1 0 1.7-.8 1.7-1.7v-3.8"/>"#,
        "bolt" => r#"<path d="M13.4 2.6 4.6 13.2c-.4.5 0 1.3.6 1.3H11l-1 6.9c-.1.7.8 1.1 1.3.5l8.8-10.6c.4-.5 0-1.3-.6-1.3H13.6l1.1-6.9c.1-.7-.8-1.1-1.3-.5z" fill="white" stroke="none"/>"#,
        "check" => r#"<path d="M19.5 6.5 9.5 17l-5-5"/>"#,
        "in" => r#"<path d="M9 4H6.5C5.7 4 5 4.7 5 5.5v13c0 .8.7 1.5 1.5 1.5H9"/><path d="M12 12h8M16.5 8.5 20 12l-3.5 3.5"/>"#,
        "out" => r#"<path d="M15 4h2.5c.8 0 1.5.7 1.5 1.5v13c0 .8-.7 1.5-1.5 1.5H15"/><path d="M4 12h8M8.5 8.5 12 12l-3.5 3.5"/>"#,
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "info" => r#"<path d="M12 16.5v-5M12 7.8v.1"/><circle cx="12" cy="12" r="9.5" stroke-width="2"/>"#,
        "warn" => r#"<path d="M10.3 3.9 2.6 17.5c-.8 1.3.2 3 1.7 3h15.4c1.5 0 2.5-1.7 1.7-3L13.7 3.9c-.8-1.3-2.6-1.3-3.4 0z" stroke-width="2"/><path d="M12 9.5v4M12 17v.1"/>"#,
        "kbd" => r#"<rect x="2.5" y="5.5" width="19" height="13" rx="3"/><path d="M7 10h.01M11 10h.01M15 10h.01M17.5 10h.01M8 14.5h8"/>"#,
        "copy" => r#"<rect x="8.5" y="8.5" width="12" height="12" rx="2.5"/><path d="M15.5 8.5V6c0-1.4-1.1-2.5-2.5-2.5H6C4.6 3.5 3.5 4.6 3.5 6v7c0 1.4 1.1 2.5 2.5 2.5h2.5"/>"#,
        "folder" => r#"<path d="M3 7c0-1.1.9-2 2-2h4l2 2.5h8c1.1 0 2 .9 2 2V17c0 1.1-.9 2-2 2H5c-1.1 0-2-.9-2-2z"/>"#,
        "chev" => r#"<path d="M6.5 9.5 12 15l5.5-5.5"/>"#,
        "tune" => r#"<path d="M4 7h9M17 7h3M4 17h3M11 17h9"/><circle cx="15" cy="7" r="2.2"/><circle cx="9" cy="17" r="2.2"/>"#,
        "film" => r#"<rect x="3" y="3.5" width="18" height="17" rx="3"/><path d="M7.5 3.5v17M16.5 3.5v17M3 8.5h4.5M3 15.5h4.5M16.5 8.5H21M16.5 15.5H21"/>"#,
        "stop" => r#"<rect x="6" y="6" width="12" height="12" rx="2.5" fill="white" stroke="none"/>"#,
        "min" => r#"<path d="M5 12h14" stroke-width="1.4"/>"#,
        "max" => r#"<rect x="5.5" y="5.5" width="13" height="13" rx="2" stroke-width="1.4"/>"#,
        "close" => r#"<path d="M6 6l12 12M18 6L6 18" stroke-width="1.4"/>"#,
        // Brand mark (assets/brand/mark.svg): a "C" ring opening into a play button.
        "logo" => r#"<path d="M17.75 7.18A7.5 7.5 0 1 0 17.75 16.82" stroke-width="3"/><path d="M10.6 9.3v5.4L15 12z" fill="white" stroke-width="1.2"/>"#,
        _ => return None,
    })
}

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(name) = path.strip_prefix("clipdat/").and_then(|p| p.strip_suffix(".svg")) {
            if let Some(b) = body(name) {
                let svg = format!(
                    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round">{b}</svg>"##
                );
                return Ok(Some(Cow::Owned(svg.into_bytes())));
            }
            return Ok(None);
        }
        gpui_kit_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit_assets::Assets.list(path)
    }
}

/// Asset path for one of our icons.
pub fn icon(name: &str) -> SharedString {
    format!("clipdat/{name}.svg").into()
}
