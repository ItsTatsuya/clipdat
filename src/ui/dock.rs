//! Clip mode: toolbar, thumbnail timeline with in/out handles, segment chips.

use std::path::PathBuf;

use gpui::{canvas, div, fill, img, point, prelude::*, px, relative, size, Bounds, Context, FontWeight, MouseButton, MouseDownEvent, MouseMoveEvent, ObjectFit};

use super::{
    app::{Clipdat, Drag, DOCK_H},
    theme::*,
    widgets::*,
};
use crate::{backend::settings::Mode, fmt};

const STRIP_H: f32 = 54.0;
const TILE_W: f32 = STRIP_H * 16.0 / 9.0;

impl Clipdat {
    pub fn dock(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let est = self.estimate_mb();
        let over = est > self.mb;
        let mb_label = format!("{} MB", self.mb);
        div()
            .h(px(DOCK_H))
            .flex_none()
            .bg(panel())
            .border_t_1()
            .border_color(stroke())
            .px(px(16.))
            .pt(px(10.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .gap(px(8.))
            // toolbar
            .child(
                div()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(self.ib("pf", "stepb", 18., 30., cx, |this, _, _| this.frame_step(false)))
                    .child(self.ib("dpp", if self.paused { "play" } else { "pause" }, 18., 30., cx, |this, _, _| this.toggle_pause()))
                    .child(self.ib("nf", "stepf", 18., 30., cx, |this, _, _| this.frame_step(true)))
                    .child(
                        div()
                            .mx(px(8.))
                            .flex()
                            .items_end()
                            .gap(px(6.))
                            .whitespace_nowrap()
                            .font_features(tnum())
                            .child(div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).child(fmt::long(self.time)))
                            .child(div().pb(px(1.)).text_size(px(12.)).text_color(text3()).child(format!("/ {}", fmt::long(self.duration)))),
                    )
                    .child(div().flex_1())
                    .child(self.tbtn("in", "in", "In", cx, |this| this.set_in()))
                    .child(self.tbtn("out", "out", "Out", cx, |this| this.set_out()))
                    .child(self.tbtn("seg", "plus", "Segment", cx, |this| this.new_segment()))
                    .child(div().w(px(1.)).h(px(20.)).mx(px(4.)).bg(stroke2()))
                    .child(self.seg(
                        "dock-mode",
                        self.mode,
                        vec![(Mode::Size, mb_label.into(), false), (Mode::Quality, "Quality".into(), false), (Mode::Lossless, "Lossless".into(), false)],
                        false,
                        cx,
                        |this, m| this.mode = m,
                    ))
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(11.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .rounded(px(9.))
                            .bg(soft())
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .whitespace_nowrap()
                            .font_features(tnum())
                            .child(
                                div()
                                    .w(px(46.))
                                    .h(px(5.))
                                    .rounded(px(3.))
                                    .bg(rgba_white(0x1f))
                                    .child(div().h_full().rounded(px(3.)).bg(if over { bad() } else { ok() }).w(relative((est / self.mb).min(1.0) as f32))),
                            )
                            .child(div().text_color(if over { bad() } else { text() }).child(format!("{} MB", fmt::mb(est))))
                            .child(div().text_color(text3()).font_weight(FontWeight::MEDIUM).child(format!("/ {}", self.mb))),
                    )
                    .child(self.button("dexport", Some("export"), "Export", true, cx, |this, _, _| this.sheet_open = true))
                    .child(self.ib("dclose", "x", 18., 30., cx, |this, _, _| this.set_clip_mode(false))),
            )
            .child(self.timeline(cx))
            // footer: segment chips + status
            .child(
                div()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex().gap(px(6.)).overflow_hidden().children(self.segs.iter().enumerate().map(|(i, g)| {
                        let on = i == self.act;
                        let a = g.a;
                        let many = self.segs.len() > 1;
                        div()
                            .id(("chip", i))
                            .h(px(22.))
                            .pl(px(9.))
                            .pr(px(if many { 4. } else { 9. }))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .rounded(px(6.))
                            .bg(if on { clip_soft() } else { soft() })
                            .text_color(if on { clip() } else { text2() })
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .whitespace_nowrap()
                            .font_features(tnum())
                            .cursor_pointer()
                            .child(format!("{}  {} – {}", i + 1, fmt::long(g.a), fmt::long(g.b)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.act = i;
                                this.seek(a, true);
                                cx.notify();
                            }))
                            .when(many, |d| {
                                d.child(
                                    div()
                                        .id(("chip-x", i))
                                        .size(px(16.))
                                        .rounded(px(4.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .opacity(0.6)
                                        .hover(|s| s.bg(rgba_white(0x1f)).opacity(1.))
                                        .child(icon("x", 11., text()))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.delete_segment(i);
                                            cx.stop_propagation();
                                            cx.notify();
                                        })),
                                )
                            })
                    })))
                    .child(div().flex_1())
                    .child(self.dock_note()),
            )
    }

    fn tbtn(&self, id: &'static str, icon_name: &'static str, label: &'static str, cx: &mut Context<Self>, on: impl Fn(&mut Clipdat) + 'static) -> impl IntoElement {
        div()
            .id(id)
            .h(px(32.))
            .px(px(10.))
            .flex()
            .items_center()
            .gap(px(7.))
            .rounded(px(9.))
            .text_size(px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .opacity(0.86)
            .cursor_pointer()
            .hover(|s| s.bg(hover()).opacity(1.))
            .child(icon(icon_name, 18., text()))
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                on(this);
                cx.notify();
            }))
    }

    fn dock_note(&self) -> impl IntoElement {
        match self.mode {
            Mode::Lossless => match self.segs.get(self.act) {
                Some(g) if !self.keyframes.is_empty() => {
                    let k = self.keyframe_before(g.a);
                    let off = g.a - k;
                    if off > 0.01 {
                        note("warn", clip(), format!("Start snaps to keyframe {} (−{:.2} s)", fmt::long(k), off))
                    } else {
                        note("check", text3(), "In point is on a keyframe")
                    }
                }
                _ => note("check", text3(), "Indexing keyframes…"),
            },
            Mode::Size => {
                let p = self.plan();
                if p.ok {
                    note("bolt", text3(), format!("NVENC {} · {}p{} · {:.1} Mbps", self.codec.label(), p.height, p.fps, p.video_kbps as f64 / 1000.0))
                } else {
                    note("warn", clip(), format!("Too long for {} MB. Trim it or pick a bigger limit", self.mb))
                }
            }
            Mode::Quality => note("bolt", text3(), format!("NVENC {} · frame-accurate", self.codec.label())),
        }
        .font_features(tnum())
    }

    fn timeline(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let d = self.duration.max(0.001);
        let bounds = self.probe("timeline");
        let width = bounds.map(|b| f32::from(b.size.width)).unwrap_or(0.0);
        let frac = |t: f64| (t / d).clamp(0.0, 1.0) as f32;

        // Thumbnail tiles.
        let n = if width > 0.0 { (width / TILE_W).ceil() as usize } else { 0 };
        let tiles: Vec<Option<PathBuf>> = (0..n)
            .map(|i| {
                let t = ((i as f32 + 0.5) * TILE_W / width) as f64 * d;
                fmt::nearest(&self.thumbs, t).map(|th| PathBuf::from(&th.path))
            })
            .collect();
        let loading = self.thumbs.is_empty();

        // Dimmed regions outside the selections.
        let mut sorted = self.segs.clone();
        sorted.sort_by(|a, b| a.a.partial_cmp(&b.a).unwrap());
        let mut dims = Vec::new();
        let mut prev = 0.0;
        for g in &sorted {
            if g.a > prev {
                dims.push((prev, g.a));
            }
            prev = f64::max(prev, g.b);
        }
        if prev < d {
            dims.push((prev, d));
        }

        // Axis labels.
        let steps = [1.0, 2.0, 5.0, 10.0, 15.0, 20.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0];
        let step = steps.iter().copied().find(|s| width > 0.0 && (*s / d) as f32 * width >= 90.0).unwrap_or(3600.0);
        let mut labels = Vec::new();
        let mut t = 0.0;
        while t <= d && width > 0.0 {
            labels.push(t);
            t += step;
        }

        let kf: Vec<f64> = if self.mode == Mode::Lossless && (self.keyframes.len() as f32) < width * 2.0 { self.keyframes.clone() } else { Vec::new() };
        let snap = if self.mode == Mode::Lossless { self.segs.get(self.act).map(|g| (self.keyframe_before(g.a), g.a)) } else { None };
        let hover = self.hover_tl.zip(bounds).map(|(x, b)| x - f32::from(b.origin.x));

        div()
            .id("timeline")
            .relative()
            .flex_1()
            .min_h(px(0.))
            .cursor_pointer()
            .child(self.probe_el("timeline"))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(px(STRIP_H))
                    .rounded(px(10.))
                    .overflow_hidden()
                    .bg(black())
                    .flex()
                    .children(tiles.into_iter().map(|p| {
                        div()
                            .w(px(TILE_W))
                            .h_full()
                            .flex_none()
                            .bg(gpui::rgb(if loading { 0x161616 } else { 0x0d0d0d }))
                            .when_some(p, |d, p| d.child(img(p).size_full().object_fit(ObjectFit::Cover)))
                    })),
            )
            .children(dims.into_iter().map(|(a, b)| {
                let first = a <= 0.0;
                let last = b >= d;
                div()
                    .absolute()
                    .top_0()
                    .h(px(STRIP_H))
                    .left(relative(frac(a)))
                    .w(relative(frac(b - a)))
                    .bg(dim())
                    .when(first, |d| d.rounded_l(px(10.)))
                    .when(last, |d| d.rounded_r(px(10.)))
            }))
            .when_some(snap, |el, (k, a)| el.child(div().absolute().top_0().h(px(STRIP_H)).left(relative(frac(k))).w(relative(frac(a - k))).bg(clip().opacity(0.45))))
            .children(self.segs.iter().enumerate().map(|(i, g)| {
                let active = i == self.act;
                let edge = if active { clip() } else { rgba_white(0x73) };
                let handle = |end: bool| {
                    div()
                        .id(("edge", i * 2 + end as usize))
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .w(px(12.))
                        .when(!end, |d| d.left_0().rounded_l(px(10.)))
                        .when(end, |d| d.right_0().rounded_r(px(10.)))
                        .bg(edge)
                        .cursor(gpui::CursorStyle::ResizeLeftRight)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().w(px(2.)).h(px(18.)).rounded(px(1.)).bg(black().opacity(0.5)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                this.begin_drag(Drag::Edge { seg: i, end }, ev.position.x, cx);
                                cx.stop_propagation();
                            }),
                        )
                };
                let len = g.b - g.a;
                div()
                    .absolute()
                    .top_0()
                    .h(px(STRIP_H))
                    .left(relative(frac(g.a)))
                    .w(relative(frac(len)))
                    .min_w(px(24.))
                    .rounded(px(10.))
                    .border_2()
                    .border_color(edge)
                    .child(handle(false))
                    .child(handle(true))
                    .when((len / d) as f32 * width > 70.0, |el| {
                        el.child(
                            div()
                                .absolute()
                                .left(px(16.))
                                .bottom(px(4.))
                                .px(px(7.))
                                .py(px(1.))
                                .rounded(px(6.))
                                .bg(black().opacity(0.65))
                                .text_size(px(11.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .font_features(tnum())
                                .child(format!("{len:.1} s")),
                        )
                    })
            }))
            .when_some(hover, |el, x| el.child(div().absolute().top_0().h(px(STRIP_H)).left(px(x)).w(px(1.)).bg(rgba_white(0x8c))))
            // playhead
            .child(
                div()
                    .absolute()
                    .top(px(-6.))
                    .h(px(STRIP_H + 12.))
                    .left(relative(frac(self.time)))
                    .ml(px(-1.))
                    .w(px(2.))
                    .rounded(px(1.))
                    .bg(text())
                    .child(div().absolute().top(px(-3.)).left(px(-4.)).size(px(10.)).rounded(px(5.)).bg(text())),
            )
            // keyframe dots (lossless only)
            .child(
                canvas(
                    |_, _, _| {},
                    move |b, _, window, _| {
                        let w = f32::from(b.size.width);
                        for t in &kf {
                            let x = f32::from(b.origin.x) + (*t / d) as f32 * w;
                            window.paint_quad(fill(Bounds::new(point(px(x - 1.), b.origin.y + px(1.)), size(px(2.5), px(2.5))), clip().opacity(0.6)));
                        }
                    },
                )
                .absolute()
                .top(px(STRIP_H + 3.))
                .left_0()
                .right_0()
                .h(px(4.)),
            )
            .children(labels.into_iter().map(|t| {
                let x = frac(t) * width;
                div()
                    .absolute()
                    .top(px(STRIP_H + 7.))
                    .left(px(if t == 0.0 { 0.0 } else { x - 18.0 }))
                    .w(px(36.))
                    .when(t > 0.0, |d| d.flex().justify_center())
                    .text_size(px(10.5))
                    .text_color(text3())
                    .font_features(tnum())
                    .child(fmt::short(t))
            }))
            .when_some(hover, |el, x| {
                let t = (x / width.max(1.)) as f64 * d;
                el.child(self.preview(x, width, t, STRIP_H + 18.))
            })
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                this.hover_tl = Some(f32::from(ev.position.x));
                cx.notify();
            }))
            .on_hover(cx.listener(|this, h: &bool, _, cx| {
                if !*h {
                    this.hover_tl = None;
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    this.begin_drag(Drag::Timeline, ev.position.x, cx);
                }),
            )
    }
}
