// Clipdat: a native (GPUI) video player and clipper for Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// GPUI comes through gpui-kit; the alias keeps `gpui::` paths (and macro output) resolving.
extern crate gpui_kit as gpui;

mod backend;
mod fmt;
mod planner;
mod ui;

use std::borrow::Cow;

use gpui::{px, size, App, AppContext, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds, WindowOptions};
use gpui_kit::component::{Root, Theme, ThemeMode};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn main() {
    let file = std::env::args().skip(1).find(|a| !a.starts_with('-') && std::path::Path::new(a).is_file());

    // A second launch (e.g. double-clicking another video) hands its file to the running window.
    if backend::instance::forward_to_running(file.as_deref()) {
        return;
    }

    let (tx, rx) = smol::channel::unbounded();
    backend::instance::serve(tx.clone());

    gpui_kit::platform::application().with_assets(ui::icons::Assets).run(move |cx: &mut App| {
        gpui_kit::component::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
        retint_theme(cx);
        cx.text_system()
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Medium.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice()),
                Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice()),
            ])
            .expect("fonts");

        let bounds = Bounds::centered(None, size(px(1280.), px(720.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some("Clipdat".into()), appears_transparent: true, traffic_light_position: None }),
            window_background: WindowBackgroundAppearance::Transparent,
            window_min_size: Some(size(px(ui::app::MIN_WINDOW.0), px(ui::app::MIN_WINDOW.1))),
            app_id: Some("Clipdat".into()),
            ..Default::default()
        };
        let window = cx
            .open_window(options, |window, cx| {
                let hwnd = match window.window_handle().map(|h| h.as_raw()) {
                    Ok(RawWindowHandle::Win32(h)) => h.hwnd.get(),
                    _ => 0,
                };
                let view = cx.new(|cx| ui::app::Clipdat::new(window, cx, hwnd, tx, rx, file));
                view.update(cx, |v, cx| v.focus.focus(window, cx));
                let v2 = view.clone();
                window.on_window_should_close(cx, move |_, cx| {
                    v2.update(cx, |v, _| v.on_quit());
                    true
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("open window");
        let _ = window;
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.activate(true);
    });
}

/// Pull gpui-component's dark theme (used by text inputs) toward the Clipdat palette.
fn retint_theme(cx: &mut App) {
    use ui::theme::*;
    let t = Theme::global_mut(cx);
    t.font_family = FONT.into();
    t.font_size = px(13.);
    t.radius = px(9.);
    t.background = gpui::transparent_black();
    t.tokens.background = gpui::transparent_black().into();
    t.foreground = text();
    t.input = stroke2();
    t.ring = accent();
    t.border = stroke2();
    t.muted_foreground = text3();
    t.selection = accent().opacity(0.35);
    t.caret = text();
}
