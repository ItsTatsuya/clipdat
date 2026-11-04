//! Export side sheet: mode, size limit, encoder, the Advanced section, after-export actions.

use gpui::{div, prelude::*, px, relative, AnyElement, Context, Div, FontWeight, MouseButton, MouseDownEvent, SharedString, Window};
use gpui_component::input::Input;

use super::{
    app::{Cliper, Drag, Opt, SHEET_W},
    theme::*,
    widgets::*,
};
use crate::{
    backend::{
        export::{self, Adv},
        settings::{Codec, Mode},
    },
    fmt, planner,
};

impl Cliper {
    fn has_encoder(&self, c: Codec) -> bool {
        self.gpu.as_ref().map(|g| g.encoders.iter().any(|e| e == &format!("{}_nvenc", c.id()))).unwrap_or(true)
    }

    pub fn gpu_name(&self) -> Option<String> {
        self.gpu.as_ref()?.adapters.iter().find(|a| a.to_lowercase().contains("nvidia")).map(|n| {
            n.trim_start_matches("NVIDIA ").trim_start_matches("GeForce ").to_string()
        })
    }

    pub fn sheet(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let est = self.estimate_mb();
        let over = est > self.mb;
        let plan = self.plan();
        let total = self.total_duration();
        let n = self.segs.len();
        let _ = window;
        div()
            .id("sheet")
            .occlude()
            .absolute()
            .top(px(10.))
            .right(px(10.))
            .bottom(px(10.))
            .w(px(SHEET_W))
            .rounded(px(16.))
            .bg(glass_strong())
            .border_1()
            .border_color(stroke())
            .shadow_lg()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .pl(px(18.))
                    .pr(px(12.))
                    .pt(px(16.))
                    .pb(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .child(div().text_size(px(16.)).font_weight(FontWeight::BOLD).child("Export"))
                            .child(div().text_size(px(12.)).text_color(text3()).font_features(tnum()).child(format!(
                                "{n} segment{} · {total:.1} s",
                                if n > 1 { "s" } else { "" }
                            ))),
                    )
                    .child(self.ib("sheet-x", "x", 18., 30., cx, |this, _, _| this.sheet_open = false)),
            )
            .child(
                div()
                    .id("sheet-body")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .px(px(18.))
                    .pt(px(6.))
                    .pb(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    // file name
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(field_label("File name"))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .child(div().flex_1().child(Input::new(&self.name_input)))
                                    .child(div().px(px(12.)).flex().items_center().rounded(px(9.)).bg(soft()).text_color(text2()).font_weight(FontWeight::SEMIBOLD).child(".mp4")),
                            )
                            .child(div().text_size(px(11.)).text_color(text3()).child(format!("Saves as {}.mp4 in {}", self.default_name(cx), self.settings.output_dir))),
                    )
                    // mode
                    .child(div().child(section_title("Mode")).child(self.seg(
                        "sheet-mode",
                        self.mode,
                        vec![(Mode::Size, "Fit to size".into(), false), (Mode::Quality, "Best quality".into(), false), (Mode::Lossless, "Lossless".into(), false)],
                        true,
                        cx,
                        |this, m| this.mode = m,
                    )))
                    .when(self.mode == Mode::Size, |d| d.child(self.size_section(plan, total, cx)))
                    .when(self.mode == Mode::Quality, |d| d.child(self.quality_section(cx)))
                    .when(self.mode == Mode::Lossless, |d| {
                        let rate = self.info.as_ref().map(|i| i.bitrate as f64 / 1e6).unwrap_or(0.0);
                        d.child(
                            div()
                                .p(px(12.))
                                .rounded(px(10.))
                                .border_1()
                                .border_color(if over { bad().opacity(0.35) } else { stroke() })
                                .bg(if over { bad().opacity(0.07) } else { soft() })
                                .flex()
                                .gap(px(10.))
                                .text_size(px(12.))
                                .text_color(text2())
                                .child(icon(if over { "warn" } else { "check" }, 16., if over { bad() } else { ok() }))
                                .child(div().flex_1().child(if over {
                                    format!("{} MB is over your {} MB limit. A lossless cut keeps the source bitrate ({rate:.0} Mbps). Use Fit to size for Discord.", fmt::mb(est), self.mb)
                                } else {
                                    "Stream copy, no quality loss, done in seconds. The start snaps to the keyframe before your in point.".to_string()
                                })),
                        )
                    })
                    .when(self.mode != Mode::Lossless, |d| d.child(self.encoder_section(cx)))
                    .child(self.advanced(cx))
                    .when(n > 1, |d| {
                        d.child(div().child(section_title("Segments")).child(self.seg(
                            "merge",
                            self.merge,
                            vec![(true, "Merge into one file".into(), false), (false, "Separate files".into(), false)],
                            true,
                            cx,
                            |this, v| this.merge = v,
                        )))
                    })
                    .child(
                        div()
                            .child(section_title("After export"))
                            .child(self.toggle_row("sw-copy", "Copy file to clipboard", Some("Then just press Ctrl+V in Discord"), self.settings.copy_clipboard, cx, |this, v| this.settings.copy_clipboard = v))
                            .child(self.toggle_row("sw-reveal", "Show in Explorer", None, self.settings.reveal, cx, |this, v| this.settings.reveal = v)),
                    ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(stroke())
                    .pl(px(18.))
                    .pr(px(14.))
                    .pt(px(12.))
                    .pb(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .font_features(tnum())
                            .child(div().text_size(px(14.)).font_weight(FontWeight::BOLD).text_color(if over { bad() } else { text() }).child(format!("≈ {} MB", fmt::mb(est))))
                            .child(div().text_size(px(11.5)).text_color(text3()).child(if self.mode == Mode::Lossless { "stream copy".to_string() } else { format!("NVENC {}", self.codec.label()) })),
                    )
                    .child(
                        self.button("go", Some("export"), "Export", true, cx, |this, w, cx| this.start_export(w, cx))
                            .h(px(38.))
                            .px(px(18.))
                            .when(self.mode == Mode::Size && !plan.ok, |d| d.opacity(0.4)),
                    ),
            )
    }

    fn size_section(&self, plan: planner::Plan, total: f64, cx: &mut Context<Self>) -> Div {
        let presets: [(f64, &str); 4] = [(10.0, "Discord"), (20.0, "Discord"), (50.0, "Nitro Basic"), (500.0, "Nitro")];
        let custom = self.settings.custom_limit;
        let has_custom = !presets.iter().any(|(m, _)| *m == custom);
        div()
            .child(section_title("Size limit"))
            .child(div().flex().gap(px(6.)).children(presets.iter().map(|(mb, l)| {
                let (mb, l) = (*mb, *l);
                let on = self.mb == mb;
                div()
                    .id(("preset", mb as usize))
                    .flex_1()
                    .h(px(52.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(if on { discord() } else { stroke() })
                    .bg(if on { discord_soft() } else { soft() })
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.border_color(stroke2()))
                    .child(div().text_size(px(14.)).font_weight(FontWeight::BOLD).child(format!("{mb} MB")))
                    .child(div().text_size(px(10.5)).font_weight(FontWeight::SEMIBOLD).text_color(if on { gpui::rgb(0xaab2ff).into() } else { text3() }).child(l))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.mb = mb;
                        cx.notify();
                    }))
            })))
            .when(has_custom, |d| {
                d.child(
                    div()
                        .id("custom-limit")
                        .mt(px(8.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(accent())
                        .cursor_pointer()
                        .child(format!("Use custom limit · {custom} MB{}", if self.mb == custom { " (selected)" } else { "" }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.mb = custom;
                            cx.notify();
                        })),
                )
            })
            .child(
                div()
                    .mt(px(12.))
                    .p(px(12.))
                    .rounded(px(11.))
                    .bg(soft())
                    .border_1()
                    .border_color(stroke())
                    .font_features(tnum())
                    .map(|d| {
                        if plan.ok {
                            let stat = |k: &'static str, v: String| {
                                div()
                                    .flex_1()
                                    .child(div().text_size(px(10.5)).font_weight(FontWeight::SEMIBOLD).text_color(text3()).child(k.to_uppercase()))
                                    .child(div().text_size(px(13.5)).font_weight(FontWeight::BOLD).child(v))
                            };
                            let audio = self.audio_kbps();
                            d.child(
                                div()
                                    .flex()
                                    .child(stat("Resolution", format!("{}p", plan.height)))
                                    .child(stat("Frame rate", format!("{} fps", plan.fps)))
                                    .child(stat("Bitrate", format!("{:.1} Mbps", plan.video_kbps as f64 / 1000.0))),
                            )
                            .child(div().mt(px(8.)).pt(px(8.)).border_t_1().border_color(stroke()).child(note(
                                "bolt",
                                text3(),
                                format!("Picked automatically to stay under {} MB{}", self.mb, if audio > 0 { format!(" · audio {audio}k") } else { " · no audio".into() }),
                            )))
                        } else {
                            d.child(note("warn", bad(), format!("{total:.0} s is too long to look good at {} MB. Trim the clip or pick a bigger limit.", self.mb)))
                        }
                    }),
            )
    }

    fn quality_section(&self, cx: &mut Context<Self>) -> Div {
        let q = self.quality;
        let i = self.info.as_ref();
        let h = i.map(|i| i.height).unwrap_or(1080);
        let f = i.map(|i| i.fps.round() as u32).unwrap_or(60);
        let mut res = vec![(0u32, Opt::new(format!("Source · {h}p")).desc(i.map(|i| format!("{} × {}", i.width, i.height)).unwrap_or_default()))];
        res.extend([2160u32, 1440, 1080, 720, 480].into_iter().filter(|x| *x < h).map(|x| (x, Opt::new(format!("{x}p")))));
        let mut fps = vec![(0u32, Opt::new(format!("Source · {f}")).desc("No frame-rate conversion"))];
        fps.extend([120u32, 60, 30].into_iter().filter(|x| *x < f).map(|x| (x, Opt::new(x.to_string()))));
        let label = format!(
            "{} · CQ {}",
            if q > 85. { "Visually lossless" } else if q > 55. { "High" } else if q > 30. { "Balanced" } else { "Small" },
            planner::cq_for(q)
        );
        div()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(field_label("Quality"))
                    .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child(label)),
            )
            .child(self.slider("quality", (q / 100.0) as f32, Drag::Quality, cx))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(px(10.5))
                    .text_color(text3())
                    .child("Smaller")
                    .child("Balanced")
                    .child("Visually lossless"),
            )
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .gap(px(8.))
                    .child(div().flex_1().child(self.dd_field("Resolution", "q-res", res, self.q_res, cx, |this, v| this.q_res = v)))
                    .child(div().flex_1().child(self.dd_field("Frame rate", "q-fps", fps, self.q_fps, cx, |this, v| this.q_fps = v))),
            )
    }

    pub fn slider(&self, id: &'static str, frac: f32, drag: Drag, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(id)
            .relative()
            .h(px(22.))
            .my(px(4.))
            .flex()
            .items_center()
            .cursor_pointer()
            .child(self.probe_el(id))
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(4.))
                    .rounded(px(2.))
                    .bg(rgba_white(0x29))
                    .child(div().absolute().left_0().top_0().bottom_0().rounded(px(2.)).bg(accent()).w(relative(frac))),
            )
            .child(div().absolute().size(px(14.)).rounded(px(7.)).bg(text()).left(relative(frac)).ml(px(-7.)).shadow_md())
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.begin_drag(drag, ev.position.x, cx);
                cx.stop_propagation();
            }))
    }

    fn encoder_section(&self, cx: &mut Context<Self>) -> Div {
        let gpu = self.gpu_name();
        let (ic, text_note) = match self.codec {
            Codec::H264 => ("check", "Plays everywhere in Discord, including the inline player on desktop, web and mobile"),
            Codec::Hevc => ("info", "About 30% smaller for the same quality, but many Discord clients won't play it inline"),
            Codec::Av1 => ("info", "The best quality for the size. Inline playback works in recent Discord clients only"),
        };
        div()
            .child(section_title("Encoder"))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .mb(px(6.))
                    .child(field_label("Codec"))
                    .child(note("bolt", if gpu.is_some() { ok() } else { text3() }, match &gpu {
                        Some(g) => format!("NVENC · {g}"),
                        None if self.gpu.is_none() => "Detecting GPU…".into(),
                        None => "NVENC not found".into(),
                    })),
            )
            .child(self.seg(
                "codec",
                self.codec,
                vec![
                    (Codec::H264, "H.264".into(), !self.has_encoder(Codec::H264)),
                    (Codec::Hevc, "HEVC".into(), !self.has_encoder(Codec::Hevc)),
                    (Codec::Av1, "AV1".into(), !self.has_encoder(Codec::Av1)),
                ],
                true,
                cx,
                |this, c| {
                    this.codec = c;
                    // Keep advanced options valid for the codec.
                    let a = &mut this.settings.adv;
                    a.profile = match c {
                        Codec::H264 if !["high", "main", "baseline"].contains(&a.profile.as_str()) => "high".into(),
                        Codec::Hevc if !["main", "main10"].contains(&a.profile.as_str()) => "main".into(),
                        Codec::Av1 => "main".into(),
                        _ => a.profile.clone(),
                    };
                    if c == Codec::H264 {
                        a.depth = 8;
                    }
                },
            ))
            .child(div().mt(px(8.)).child(note(ic, text3(), text_note)))
    }

    fn advanced(&mut self, cx: &mut Context<Self>) -> Div {
        let defaults = Adv::default();
        let a = self.settings.adv.clone();
        let changed = adv_diff(&a, &defaults);
        let open = self.settings.adv_open;
        let header = div()
            .id("adv-h")
            .h(px(40.))
            .px(px(12.))
            .flex()
            .items_center()
            .gap(px(9.))
            .rounded(px(10.))
            .bg(soft())
            .border_1()
            .border_color(stroke())
            .text_size(px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|s| s.bg(rgba_white(0x12)))
            .child(icon("tune", 16., text()))
            .child("Advanced")
            .child(div().ml_auto().text_size(px(11.)).text_color(if changed > 0 { clip() } else { text3() }).child(if changed > 0 { format!("{changed} changed") } else { "Defaults".into() }))
            .child(icon("chev", 16., text3()))
            .on_click(cx.listener(|this, _, _, cx| {
                this.settings.adv_open = !this.settings.adv_open;
                this.changed(cx);
            }));
        let mut body = div();
        if open {
            body = self.advanced_body(a, cx);
        }
        div().child(header).when(open, |d| d.child(body))
    }

    fn advanced_body(&mut self, a: Adv, cx: &mut Context<Self>) -> Div {
        let size_mode = self.mode == Mode::Size;
        let h264 = self.codec == Codec::H264;
        let tracks = self.info.as_ref().map(|i| i.audio.clone()).unwrap_or_default();

        // Saved presets.
        let builtin: Vec<(&str, &str, Adv)> = vec![
            ("Discord · balanced", "Fit to size, P6, full-res multipass", Adv::default()),
            ("Discord · max quality", "P7, 32-frame lookahead, slower", Adv { preset: "p7".into(), lookahead: 32, ..Adv::default() }),
            ("Fast", "P3, no multipass, quickest exports", Adv { preset: "p3".into(), multipass: "disabled".into(), lookahead: 8, ..Adv::default() }),
        ];
        let mut preset_items: Vec<(String, Opt)> = builtin.iter().map(|(n, d, _)| (format!("b:{n}"), Opt::new(*n).desc(*d).group("Built-in"))).collect();
        preset_items.extend(self.settings.presets.iter().map(|p| (format!("u:{}", p.name), Opt::new(p.name.clone()).group("Saved"))));
        let current = builtin
            .iter()
            .find(|(_, _, adv)| adv_eq_ignoring_extra(adv, &a))
            .map(|(n, _, _)| format!("b:{n}"))
            .or_else(|| self.settings.presets.iter().find(|p| adv_eq_ignoring_extra(&p.adv, &a)).map(|p| format!("u:{}", p.name)))
            .unwrap_or_default();
        if current.is_empty() {
            preset_items.push((String::new(), Opt::new("Custom").desc("Your current settings").disabled_if(true, "Your current settings").group(if self.settings.presets.is_empty() { "Built-in" } else { "Saved" })));
        }
        let builtin_advs: Vec<(String, Adv)> = builtin.into_iter().map(|(n, _, adv)| (format!("b:{n}"), adv)).collect();

        let num = |v: &[(f64, &str, Option<&str>)]| -> Vec<(f64, Opt)> {
            v.iter().map(|(x, l, d)| (*x, d.map_or(Opt::new(*l), |d| Opt::new(*l).desc(d)))).collect()
        };
        let s = |v: &[(&str, &str, Option<&str>)]| -> Vec<(String, Opt)> {
            v.iter().map(|(x, l, d)| (x.to_string(), d.map_or(Opt::new(*l), |d| Opt::new(*l).desc(d)))).collect()
        };
        let u = |v: &[(u32, &str, Option<&str>)]| -> Vec<(u32, Opt)> {
            v.iter().map(|(x, l, d)| (*x, d.map_or(Opt::new(*l), |d| Opt::new(*l).desc(d)))).collect()
        };

        let rc_items = vec![
            ("vbr".to_string(), Opt::new("VBR").desc("Target bitrate with capped peaks. Best for size limits")),
            ("cbr".to_string(), Opt::new("CBR").desc("Flat bitrate. Predictable, less efficient")),
            ("cq".to_string(), Opt::new("Constant quality").desc("Size varies").disabled_if(size_mode, "Not available in Fit to size")),
            ("qp".to_string(), Opt::new("Constant QP").desc("Fixed quantizer per frame type").disabled_if(size_mode, "Not available in Fit to size")),
        ];
        let profile_items: Vec<(String, Opt)> = match self.codec {
            Codec::H264 => s(&[("high", "High", Some("Recommended")), ("main", "Main", None), ("baseline", "Baseline", Some("Old devices"))]),
            Codec::Hevc => s(&[("main", "Main", Some("8-bit")), ("main10", "Main 10", Some("10-bit"))]),
            Codec::Av1 => s(&[("main", "Main", Some("8 or 10-bit"))]),
        };
        let depth_items = vec![
            (8u32, Opt::new("8-bit").desc("yuv420p · plays everywhere")),
            (10u32, Opt::new("10-bit").desc("p010 · less banding").disabled_if(h264, "Not supported by H.264 NVENC")),
        ];
        let vol_items = |vals: &[f64]| -> Vec<(f64, Opt)> {
            vals.iter().map(|v| (*v, Opt::new(if *v == 0.0 { "Unchanged".to_string() } else { format!("{}{} dB", if *v > 0.0 { "+" } else { "−" }, v.abs()) }))).collect()
        };
        let cpu_note = a.crop != "src" || a.hdr == "tonemap" || a.sharpen > 0.0;

        let grid = |children: Vec<Div>| div().grid().grid_cols(2).gap(px(8.)).children(children);

        let mut body = div()
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .gap(px(8.))
                    .map(|d| {
                        if self.naming_preset {
                            d.child(div().flex_1().child(Input::new(&self.preset_input)))
                        } else {
                            d.child(div().flex_1().min_w(px(0.)).child(self.dd("preset", preset_items, current, cx, move |this, v: String| {
                                if let Some((_, adv)) = builtin_advs.iter().find(|(k, _)| *k == v) {
                                    this.settings.adv = Adv { extra: this.settings.adv.extra.clone(), ..adv.clone() };
                                } else if let Some(p) = this.settings.presets.iter().find(|p| format!("u:{}", p.name) == v) {
                                    this.settings.adv = p.adv.clone();
                                }
                            })))
                            .child(self.button("save-preset", None, "Save preset", false, cx, |this, w, cx| {
                                this.naming_preset = true;
                                this.preset_input.update(cx, |s, cx| {
                                    s.set_value("", w, cx);
                                    s.focus(w, cx);
                                });
                            }))
                        }
                    }),
            );

        if self.mode != Mode::Lossless {
            body = body
                .child(sub_header("Rate control"))
                .child(grid(vec![
                    self.dd_field("Mode", "a-rc", rc_items, a.rc.clone(), cx, |t, v| t.settings.adv.rc = v),
                    self.dd_field("Peak bitrate", "a-peak", num(&[(1.15, "Target +15%", Some("Recommended")), (1.3, "Target +30%", None), (1.5, "Target +50%", None), (1.0, "Same as target", None)]), a.peak, cx, |t, v| t.settings.adv.peak = v),
                    self.dd_field("Multipass", "a-mp", s(&[("disabled", "Off", Some("Fastest, least accurate")), ("qres", "Quarter res", Some("First pass at ¼ resolution")), ("fullres", "Full res", Some("Best bitrate distribution"))]), a.multipass.clone(), cx, |t, v| t.settings.adv.multipass = v),
                    self.dd_field("Lookahead", "a-la", u(&[(0, "Off", None), (8, "8 frames", None), (20, "20 frames", Some("Recommended")), (32, "32 frames", Some("Slower, better scene cuts"))]), a.lookahead, cx, |t, v| t.settings.adv.lookahead = v),
                ]))
                .child(sub_header("Encoder"))
                .child(grid(vec![
                    self.dd_field("Preset", "a-preset", s(&[("p1", "P1 · Fastest", Some("Fastest, lowest quality")), ("p2", "P2 · Faster", None), ("p3", "P3 · Fast", None), ("p4", "P4 · Medium", Some("NVENC default")), ("p5", "P5 · Slow", None), ("p6", "P6 · Slower", Some("Recommended for clips")), ("p7", "P7 · Slowest", Some("Best quality, ~2× slower than P4"))]), a.preset.clone(), cx, |t, v| t.settings.adv.preset = v),
                    self.dd_field("Tune", "a-tune", s(&[("hq", "High quality", Some("For recordings and uploads")), ("ll", "Low latency", Some("For streaming")), ("ull", "Ultra-low latency", None), ("lossless", "Lossless", Some("Huge files"))]), a.tune.clone(), cx, |t, v| t.settings.adv.tune = v),
                    self.dd_field("Profile", "a-prof", profile_items, a.profile.clone(), cx, |t, v| t.settings.adv.profile = v),
                    self.dd_field("Bit depth", "a-depth", depth_items, a.depth, cx, |t, v| t.settings.adv.depth = v),
                    self.dd_field("B-frames", "a-bf", u(&[(0, "None", None), (1, "1", None), (2, "2", None), (3, "3", Some("Recommended")), (4, "4", None)]), a.bframes, cx, |t, v| t.settings.adv.bframes = v),
                    self.dd_field("B-frame refs", "a-bref", s(&[("middle", "Middle", Some("Recommended")), ("each", "Each", None), ("disabled", "Off", None)]), a.bref.clone(), cx, |t, v| t.settings.adv.bref = v),
                    self.dd_field("Keyframe every", "a-gop", num(&[(1.0, "1 second", None), (2.0, "2 seconds", Some("Good seeking in Discord")), (5.0, "5 seconds", None), (10.0, "10 seconds", Some("Smallest files"))]), a.gop_sec, cx, |t, v| t.settings.adv.gop_sec = v),
                    self.dd_field("Ref frames", "a-refs", s(&[("auto", "Auto", None), ("2", "2", None), ("4", "4", None), ("6", "6", None)]), a.refs.clone(), cx, |t, v| t.settings.adv.refs = v),
                ]))
                .child(div().h(px(6.)))
                .child(self.toggle_row("a-saq", "Spatial AQ", Some("Gives more bits to flat areas like skies and walls"), a.spatial_aq, cx, |t, v| t.settings.adv.spatial_aq = v))
                .when(a.spatial_aq, |d| {
                    d.child(
                        div()
                            .flex()
                            .justify_between()
                            .child(field_label("AQ strength"))
                            .child(div().text_size(px(12.)).font_weight(FontWeight::SEMIBOLD).child(a.aq_strength.to_string())),
                    )
                    .child(self.slider("aq", (a.aq_strength.saturating_sub(1)) as f32 / 14.0, Drag::AqStrength, cx))
                })
                .child(self.toggle_row("a-taq", "Temporal AQ", Some("Sharper static HUD and UI elements"), a.temporal_aq, cx, |t, v| t.settings.adv.temporal_aq = v))
                .child(self.toggle_row("a-wp", "Weighted prediction", Some("Helps with fades and flashes. Turns off B-frames"), a.weighted_pred, cx, |t, v| t.settings.adv.weighted_pred = v))
                .child(sub_header("Picture"))
                .child(grid(vec![
                    self.dd_field("Scaling", "a-scale", s(&[("lanczos", "Lanczos", Some("Sharpest")), ("bicubic", "Bicubic", None), ("bilinear", "Bilinear", None), ("nearest", "Nearest", Some("Pixel art"))]), a.scale.clone(), cx, |t, v| t.settings.adv.scale = v),
                    self.dd_field("Crop", "a-crop", s(&[("src", "Source", None), ("916", "9:16 vertical", Some("Centered, for phones")), ("11", "1:1 square", None), ("43", "4:3", None), ("219", "21:9", Some("Ultrawide"))]), a.crop.clone(), cx, |t, v| t.settings.adv.crop = v),
                    self.dd_field("HDR", "a-hdr", s(&[("keep", "Keep source", Some(if self.info.as_ref().is_some_and(|i| i.hdr) { "Source is HDR" } else { "Source is SDR" })), ("tonemap", "Tone map to SDR", Some("Hable, for HDR captures"))]), a.hdr.clone(), cx, |t, v| t.settings.adv.hdr = v),
                    self.dd_field("Color range", "a-range", s(&[("auto", "Auto", None), ("tv", "Limited", Some("16–235, standard")), ("pc", "Full", Some("0–255"))]), a.range.clone(), cx, |t, v| t.settings.adv.range = v),
                    self.dd_field("Frame timing", "a-vsync", s(&[("cfr", "Constant", Some("Smooth playback in Discord")), ("vfr", "Variable", Some("Keep source timestamps"))]), a.vsync.clone(), cx, |t, v| t.settings.adv.vsync = v),
                    self.dd_field("Sharpen", "a-sharp", num(&[(0.0, "Off", None), (0.3, "Light", None), (0.6, "Medium", None), (1.0, "Strong", None)]), a.sharpen, cx, |t, v| t.settings.adv.sharpen = v),
                ]))
                .when(cpu_note, |d| d.child(div().mt(px(8.)).child(note("info", text3(), "Crop, tone mapping and sharpening run on the CPU, so this export will be slower"))));
        }

        body = body.child(sub_header("Audio"));
        if tracks.is_empty() {
            body = body.child(note("info", text3(), "This video has no audio"));
        } else {
            body = body
                .child(div().flex().flex_col().gap(px(6.)).mb(px(6.)).children(tracks.iter().map(|t| {
                    let on = a.tracks.contains(&t.index);
                    let idx = t.index;
                    let title = if t.title.is_empty() { format!("Track {}", t.index + 1) } else { format!("Track {} · {}", t.index + 1, t.title) };
                    let detail = format!("{} · {} · {} kHz", t.codec.to_uppercase(), if t.layout.is_empty() { format!("{} ch", t.channels) } else { t.layout.clone() }, (t.sample_rate as f64 / 1000.0).round());
                    div()
                        .id(("trk", idx))
                        .h(px(36.))
                        .px(px(11.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .rounded(px(9.))
                        .border_1()
                        .border_color(if on { accent().opacity(0.4) } else { stroke() })
                        .bg(if on { accent().opacity(0.08) } else { rgba_white(0x08) })
                        .cursor_pointer()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(
                            div()
                                .size(px(16.))
                                .rounded(px(5.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(on, |d| d.bg(accent()).child(icon("check", 11., text())))
                                .when(!on, |d| d.border_1().border_color(rgba_white(0x4d))),
                        )
                        .child(title)
                        .child(div().ml_auto().text_size(px(11.)).font_weight(FontWeight::MEDIUM).text_color(text3()).child(detail))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let tr = &mut this.settings.adv.tracks;
                            if let Some(p) = tr.iter().position(|x| *x == idx) {
                                tr.remove(p);
                            } else {
                                tr.push(idx);
                                tr.sort();
                            }
                            this.changed(cx);
                        }))
                })))
                .when(tracks.len() > 1, |d| d.child(self.toggle_row("a-mix", "Mix selected tracks into one", Some("Discord only plays the first audio track"), a.mix, cx, |t, v| t.settings.adv.mix = v)))
                .child(div().h(px(6.)))
                .child(grid({
                    let mut v = vec![
                        self.dd_field("Codec", "a-ac", vec![
                            ("aac".to_string(), Opt::new("AAC").desc("Plays everywhere")),
                            ("libopus".to_string(), Opt::new("Opus").desc("Better at low bitrates")),
                            ("copy".to_string(), Opt::new("Copy source").disabled_if(a.mix && a.tracks.len() > 1, "Mixing needs re-encoding")),
                            ("none".to_string(), Opt::new("No audio")),
                        ], a.acodec.clone(), cx, |t, v| t.settings.adv.acodec = v),
                        self.dd_field("Bitrate", "a-ab", [96u32, 128, 160, 192, 256, 320].iter().map(|b| (*b, if *b == 128 { Opt::new(format!("{b} kbps")).desc("Recommended") } else { Opt::new(format!("{b} kbps")) })).collect(), a.abitrate, cx, |t, v| t.settings.adv.abitrate = v),
                        self.dd_field("Channels", "a-ch", s(&[("2", "Stereo", None), ("1", "Mono", None), ("src", "Source", None)]), a.channels.clone(), cx, |t, v| t.settings.adv.channels = v),
                        self.dd_field("Sample rate", "a-sr", u(&[(48000, "48 kHz", None), (44100, "44.1 kHz", None)]), a.sample_rate, cx, |t, v| t.settings.adv.sample_rate = v),
                        self.dd_field(format!("{} volume", if tracks[0].title.is_empty() { "Track 1" } else { &tracks[0].title }), "a-gv", vol_items(&[0.0, -6.0, -3.0, 3.0, 6.0]), a.game_vol, cx, |t, v| t.settings.adv.game_vol = v),
                    ];
                    if tracks.len() > 1 {
                        v.push(self.dd_field(format!("{} volume", if tracks[1].title.is_empty() { "Track 2" } else { &tracks[1].title }), "a-mv", vol_items(&[0.0, 3.0, 6.0, 10.0, -3.0, -6.0]), a.mic_vol, cx, |t, v| t.settings.adv.mic_vol = v));
                    }
                    v
                }))
                .child(div().h(px(6.)))
                .child(self.toggle_row("a-loud", "Normalize loudness", Some("EBU R128, −14 LUFS, so clips aren't too loud"), a.loudnorm, cx, |t, v| t.settings.adv.loudnorm = v));
        }

        body = body
            .child(sub_header("Output"))
            .child(self.toggle_row("a-fast", "Fast start", Some("Starts playing before the file has fully loaded"), a.faststart, cx, |t, v| t.settings.adv.faststart = v))
            .child(self.toggle_row("a-meta", "Keep metadata", Some("Title, recording date, encoder tags"), a.keep_meta, cx, |t, v| t.settings.adv.keep_meta = v))
            .child(self.toggle_row("a-hw", "Hardware decoding", Some("Decodes on NVDEC and keeps frames on the GPU"), a.hwdec, cx, |t, v| t.settings.adv.hwdec = v));

        if self.settings.show_command {
            let cmd = export::preview(&self.build_request(cx));
            body = body
                .child(sub_header("FFmpeg"))
                .child(div().flex().flex_col().gap(px(6.)).child(field_label("Extra arguments")).child(Input::new(&self.extra_input)))
                .child(
                    div()
                        .mt(px(12.))
                        .mb(px(6.))
                        .flex()
                        .justify_between()
                        .child(field_label("Command"))
                        .child(
                            div()
                                .id("copy-cmd")
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(accent())
                                .cursor_pointer()
                                .child(icon("copy", 12., accent()))
                                .child(if self.cmd_copied.is_some() { "Copied" } else { "Copy" })
                                .on_click(cx.listener(|this, _, _, cx| this.copy_command(cx))),
                        ),
                )
                .child(command_block(&cmd));
        }

        body.child(
            div().mt(px(8.)).flex().justify_end().child(
                div()
                    .id("adv-reset")
                    .text_size(px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(accent())
                    .cursor_pointer()
                    .child("Reset to defaults")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.settings.adv = Adv::default();
                        this.changed(cx);
                    })),
            ),
        )
    }
}

fn adv_eq_ignoring_extra(a: &Adv, b: &Adv) -> bool {
    Adv { extra: String::new(), ..a.clone() } == Adv { extra: String::new(), ..b.clone() }
}

/// Number of advanced fields that differ from the defaults.
fn adv_diff(a: &Adv, d: &Adv) -> usize {
    let (va, vd) = (serde_json::to_value(a).unwrap_or_default(), serde_json::to_value(d).unwrap_or_default());
    match (va.as_object(), vd.as_object()) {
        (Some(x), Some(y)) => x.iter().filter(|(k, v)| y.get(*k) != Some(*v)).count(),
        _ => 0,
    }
}

/// Monospace command with flags and quoted values highlighted.
fn command_block(cmd: &str) -> AnyElement {
    let mut lines: Vec<Vec<(String, gpui::Hsla)>> = vec![vec![]];
    let breaks = ["-i", "-vf", "-c:v", "-rc", "-multipass", "-filter_complex", "-map", "-c:a", "-movflags", "-map_metadata", "-ss"];
    let mut in_quote = false;
    let mut word = String::new();
    let mut tokens = Vec::new();
    for ch in cmd.chars() {
        if ch == '"' {
            in_quote = !in_quote;
        }
        if ch == ' ' && !in_quote {
            tokens.push(std::mem::take(&mut word));
        } else {
            word.push(ch);
        }
    }
    tokens.push(word);
    for (i, t) in tokens.into_iter().enumerate() {
        if i > 0 && breaks.contains(&t.as_str()) {
            lines.push(vec![]);
        }
        let color = if i == 0 {
            text()
        } else if t.starts_with('"') {
            gpui::rgb(0xffcf7a).into()
        } else if t.starts_with('-') && t.len() > 1 && !t[1..2].chars().all(|c| c.is_ascii_digit()) {
            gpui::rgb(0x8fb3ff).into()
        } else {
            text2()
        };
        lines.last_mut().unwrap().push((t, color));
    }
    div()
        .p(px(10.))
        .rounded(px(10.))
        .bg(gpui::rgb(0x0a0a0d))
        .border_1()
        .border_color(stroke())
        .font_family("Cascadia Mono")
        .text_size(px(10.5))
        .line_height(px(17.))
        .flex()
        .flex_col()
        .children(lines.into_iter().enumerate().map(|(li, words)| {
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(6.))
                .when(li > 0, |d| d.pl(px(12.)))
                .children(words.into_iter().map(|(w, c)| div().text_color(c).child(SharedString::from(w))))
        }))
        .into_any_element()
}
