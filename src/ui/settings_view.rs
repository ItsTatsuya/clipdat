//! Settings modal: General, Export, GPU, Shortcuts.

use gpui::{div, prelude::*, px, Context, Div, FontWeight, MouseButton};
use gpui_component::input::Input;

use super::{
    app::{Clipdat, Opt, SettingsTab},
    theme::*,
    widgets::*,
};
use crate::backend::settings::{Codec, Mode, SegmentsPref};

fn group() -> Div {
    div().rounded(px(12.)).border_1().border_color(stroke()).bg(rgba_white(0x06)).mb(px(14.)).flex().flex_col()
}

fn row(first: bool) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .px(px(14.))
        .py(px(10.))
        .text_size(px(12.5))
        .font_weight(FontWeight::SEMIBOLD)
        .when(!first, |d| d.border_t_1().border_color(stroke()))
}

fn label(title: &'static str, hint: Option<String>) -> Div {
    div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .child(title)
        .when_some(hint, |d, h| d.child(div().text_size(px(11.5)).font_weight(FontWeight::MEDIUM).text_color(text3()).child(h)))
}

impl Clipdat {
    pub fn settings_modal(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = [
            (SettingsTab::General, "General", "gear"),
            (SettingsTab::Export, "Export", "export"),
            (SettingsTab::Gpu, "GPU", "bolt"),
            (SettingsTab::Keys, "Shortcuts", "kbd"),
        ];
        div()
            .id("scrim")
            .absolute()
            .inset_0()
            .occlude()
            .bg(black().opacity(0.55))
            .flex()
            .items_center()
            .justify_center()
            .p(px(20.))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                this.settings_open = false;
                cx.notify();
            }))
            .child(
                div()
                    .id("modal")
                    .occlude()
                    .w(px(760.))
                    .h(px(520.))
                    .max_w_full()
                    .max_h_full()
                    .rounded(px(18.))
                    .overflow_hidden()
                    .bg(glass_strong())
                    .border_1()
                    .border_color(stroke())
                    .shadow_lg()
                    .flex()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .w(px(190.))
                            .flex_none()
                            .bg(black().opacity(0.22))
                            .border_r_1()
                            .border_color(stroke())
                            .px(px(8.))
                            .py(px(16.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(div().px(px(10.)).pb(px(14.)).text_size(px(16.)).font_weight(FontWeight::BOLD).child("Settings"))
                            .children(tabs.into_iter().map(|(t, l, ic)| {
                                let on = self.settings_tab == t;
                                div()
                                    .id(l)
                                    .h(px(34.))
                                    .px(px(10.))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .rounded(px(9.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if on { text() } else { text2() })
                                    .when(on, |d| d.bg(rgba_white(0x1a)))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgba_white(0x0d)))
                                    .child(icon(ic, 16., if on { text() } else { text2() }))
                                    .child(l)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.settings_tab = t;
                                        cx.notify();
                                    }))
                            })),
                    )
                    .child(
                        div()
                            .id("pane")
                            .relative()
                            .flex_1()
                            .overflow_y_scroll()
                            .px(px(22.))
                            .py(px(18.))
                            .child(div().absolute().top(px(12.)).right(px(12.)).child(self.ib("settings-x", "x", 18., 30., cx, |this, _, _| this.settings_open = false)))
                            .child(match self.settings_tab {
                                SettingsTab::General => self.tab_general(cx),
                                SettingsTab::Export => self.tab_export(cx),
                                SettingsTab::Gpu => self.tab_gpu(),
                                SettingsTab::Keys => tab_keys(),
                            }),
                    ),
            )
    }

    fn tab_general(&mut self, cx: &mut Context<Self>) -> Div {
        let s = &self.settings;
        div()
            .child(div().text_size(px(17.)).font_weight(FontWeight::BOLD).mb(px(14.)).child("General"))
            .child(
                group()
                    .child(
                        row(true)
                            .child(label("Default video player", Some("Registers Clipdat for video files, then opens Windows Default apps".into())))
                            .child(self.button("make-default", None, "Make default…", false, cx, |this, _, cx| this.register_default(cx))),
                    )
                    .child(row(false).child(label("Hardware decoding", None)).child(div().w(px(210.)).child(self.dd(
                        "hwdec",
                        vec![
                            ("auto-safe".to_string(), Opt::new("Auto").desc("D3D11 / NVDEC when safe")),
                            ("nvdec".to_string(), Opt::new("NVDEC")),
                            ("d3d11va".to_string(), Opt::new("D3D11VA")),
                            ("no".to_string(), Opt::new("Off")),
                        ],
                        s.hwdec.clone(),
                        cx,
                        |this, v| {
                            this.mpv_set("hwdec", serde_json::json!(v));
                            this.settings.hwdec = v;
                        },
                    ))))
                    .child(row(false).child(label("Resume where I left off", None)).child(self.switch("resume", s.resume, cx, |t, v| t.settings.resume = v)))
                    .child(row(false).child(label("Hide controls after", None)).child(div().w(px(210.)).child(self.dd(
                        "autohide",
                        vec![(1500u64, Opt::new("1.5 seconds")), (2000, Opt::new("2 seconds")), (4000, Opt::new("4 seconds")), (0, Opt::new("Never"))],
                        s.auto_hide_ms,
                        cx,
                        |t, v| t.settings.auto_hide_ms = v,
                    )))),
            )
    }

    fn tab_export(&mut self, cx: &mut Context<Self>) -> Div {
        let s = self.settings.clone();
        let mut limits = vec![(20.0, Opt::new("20 MB · Discord")), (10.0, Opt::new("10 MB · Discord")), (50.0, Opt::new("50 MB · Nitro Basic")), (500.0, Opt::new("500 MB · Nitro"))];
        if ![10.0, 20.0, 50.0, 500.0].contains(&s.custom_limit) {
            limits.push((s.custom_limit, Opt::new(format!("{} MB · Custom", s.custom_limit))));
        }
        div()
            .child(div().text_size(px(17.)).font_weight(FontWeight::BOLD).mb(px(14.)).child("Export"))
            .child(
                group()
                    .child(row(true).child(label("Default mode", None)).child(self.seg(
                        "def-mode",
                        s.default_mode,
                        vec![(Mode::Size, "Fit to size".into(), false), (Mode::Quality, "Quality".into(), false), (Mode::Lossless, "Lossless".into(), false)],
                        false,
                        cx,
                        |t, v| t.settings.default_mode = v,
                    )))
                    .child(row(false).child(label("Default size limit", Some("Shown on the clip toolbar".into()))).child(div().w(px(210.)).child(self.dd("def-limit", limits, s.default_limit, cx, |t, v| t.settings.default_limit = v))))
                    .child(row(false).child(label("Custom size limit (MB)", Some("An extra option in the export sheet".into()))).child(div().w(px(120.)).child(Input::new(&self.limit_input))))
                    .child(row(false).child(label("Default codec", None)).child(self.seg(
                        "def-codec",
                        s.default_codec,
                        vec![(Codec::H264, "H.264".into(), false), (Codec::Hevc, "HEVC".into(), false), (Codec::Av1, "AV1".into(), false)],
                        false,
                        cx,
                        |t, v| t.settings.default_codec = v,
                    )))
                    .child(row(false).child(label("Multiple segments", None)).child(div().w(px(210.)).child(self.dd(
                        "def-segs",
                        vec![
                            (SegmentsPref::Ask, Opt::new("Merge, change per export").desc("Choose in the export sheet")),
                            (SegmentsPref::Merge, Opt::new("Merge into one file")),
                            (SegmentsPref::Separate, Opt::new("Separate files")),
                        ],
                        s.segments,
                        cx,
                        |t, v| t.settings.segments = v,
                    )))),
            )
            .child(
                group()
                    .child(
                        row(true)
                            .child(label("Output folder", Some(s.output_dir.clone())))
                            .child(self.button("outdir", None, "Change…", false, cx, |this, w, cx| this.choose_output_dir(w, cx))),
                    )
                    .child(row(false).child(label("File name", Some("{name} {n} {date} {time}".into()))).child(div().w(px(220.)).child(Input::new(&self.template_input))))
                    .child(row(false).child(label("Copy file to clipboard after export", None)).child(self.switch("set-copy", s.copy_clipboard, cx, |t, v| t.settings.copy_clipboard = v)))
                    .child(row(false).child(label("Show advanced options by default", Some("Expands Advanced in the export sheet".into()))).child(self.switch("set-adv", s.adv_open, cx, |t, v| t.settings.adv_open = v)))
                    .child(row(false).child(label("Show FFmpeg command", Some("Shows the exact command in the Advanced section".into()))).child(self.switch("set-cmd", s.show_command, cx, |t, v| t.settings.show_command = v))),
            )
    }

    fn tab_gpu(&self) -> Div {
        let mut g = group();
        match &self.gpu {
            None => g = g.child(row(true).child(label("Detecting hardware…", None))),
            Some(info) => {
                for (i, name) in info.adapters.iter().enumerate() {
                    let lower = name.to_lowercase();
                    let vendor = if lower.contains("nvidia") { "nvenc" } else if lower.contains("amd") || lower.contains("radeon") { "amf" } else if lower.contains("intel") { "qsv" } else { "-" };
                    let encs: Vec<String> = info.encoders.iter().filter(|e| e.ends_with(vendor)).map(|e| e.split('_').next().unwrap_or("").to_uppercase()).collect();
                    let ready = !encs.is_empty();
                    g = g.child(
                        row(i == 0)
                            .child(label("", None).child(name.clone()).child(div().text_size(px(11.5)).font_weight(FontWeight::MEDIUM).text_color(text3()).child(if ready { encs.join(" · ") } else { "No hardware encoder found".into() })))
                            .child(note(if ready { "check" } else { "x" }, if ready { ok() } else { text3() }, if ready { "Ready" } else { "Unavailable" })),
                    );
                }
            }
        }
        div()
            .child(div().text_size(px(17.)).font_weight(FontWeight::BOLD).mb(px(14.)).child("GPU"))
            .child(g.child(row(false).child(label("Encoding uses NVENC", Some("Other GPU vendors are detected but not used for export yet".into())))))
    }
}

fn tab_keys() -> Div {
    let keys: &[(&str, &[&str])] = &[
        ("Play / pause", &["Space"]),
        ("Open file", &["Ctrl", "O"]),
        ("Clip mode", &["C"]),
        ("Set in / out", &["I", "O"]),
        ("Previous / next frame", &[",", "."]),
        ("Seek ±5 s (Shift: ±1 s)", &["←", "→"]),
        ("Volume", &["↑", "↓"]),
        ("Mute", &["M"]),
        ("Fullscreen", &["F"]),
        ("New segment", &["N"]),
        ("Delete segment", &["Del"]),
        ("Export", &["E"]),
        ("Quick export with last settings", &["Ctrl", "E"]),
        ("Settings", &["Ctrl", ","]),
        ("Close panel / exit clip mode", &["Esc"]),
    ];
    div()
        .child(div().text_size(px(17.)).font_weight(FontWeight::BOLD).mb(px(14.)).child("Shortcuts"))
        .children(keys.iter().map(|(l, ks)| {
            div()
                .flex()
                .justify_between()
                .py(px(5.))
                .text_size(px(12.5))
                .text_color(text2())
                .child(*l)
                .child(div().flex().gap(px(4.)).children(ks.iter().map(|k| {
                    div().px(px(6.)).py(px(1.)).rounded(px(4.)).bg(rgba_white(0x14)).text_size(px(10.5)).font_weight(FontWeight::SEMIBOLD).child(*k)
                })))
        }))
}
