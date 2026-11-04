//! Window frame, player chrome, overlays, jobs, menus.

use std::path::PathBuf;

use gpui::{
    anchored, canvas, deferred, div, img, linear_color_stop, linear_gradient, point, prelude::*, px, relative, AnyElement,
    Context, ExternalPaths, FontWeight, MouseButton, DispatchPhase, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, ScrollEvent,
    Window, WindowControlArea,
};

use super::{
    app::{Clipdat, Drag, DOCK_H, TITLEBAR_H},
    theme::*,
    widgets::*,
};
use crate::fmt;

impl Render for Clipdat {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has = self.path.is_some();
        self.clip_e = self.clip_tw.follow(has && self.clip_on);
        self.sheet_e = self.sheet_tw.follow(has && self.clip_on && self.sheet_open);
        if self.clip_tw.animating() || self.sheet_tw.animating() {
            window.request_animation_frame();
        }
        self.sync_margins(window);
        div()
            .id("root")
            .size_full()
            .flex()
            .flex_col()
            .font_family(FONT)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(text())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev, window, cx| this.on_key(ev, window, cx)))
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, window, cx| {
                this.wake(cx);
                if this.drag.is_none() {
                    this.video_move(ev.position, ev.pressed_button == Some(MouseButton::Left), window);
                }
            }))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                this.dropping = false;
                let list = paths.paths();
                if let Some(p) = list.iter().find(|p| Clipdat::video_ext_ok(p)).or(list.first()) {
                    this.open_file(p.to_string_lossy().into_owned(), window, cx);
                }
            }))
            .when(self.drag.is_some(), |d| d.child(self.drag_tracker()))
            .child(self.stage(window, cx))
            // The dock rises like a drawer: its slot grows from 0 to DOCK_H and the stage shrinks with it.
            .when(has && self.clip_e > 0.001, |d| {
                let e = self.clip_e;
                d.child(div().flex_none().h(px(DOCK_H * e)).overflow_hidden().opacity((e * 1.6).min(1.0)).child(self.dock(cx)))
            })
            .when(self.settings_open, |d| d.child(self.settings_modal(cx)))
            .when(self.menu.is_some(), |d| d.child(self.menu_popup(cx)))
    }
}

const CAPTION_W: f32 = 46.;

impl Clipdat {
    /// While a slider / timeline drag is live, follow the mouse at the *window* level.
    ///
    /// Element listeners (`on_mouse_move`, `on_mouse_up`) only fire while their element is hovered,
    /// and the controller, dock and sheet occlude the root. Moves over them were dropped and a
    /// release over them never ended the drag, so the playhead kept following the cursor.
    /// `window.on_mouse_event` sees every event regardless of what is under the mouse.
    fn drag_tracker(&self) -> impl IntoElement {
        let me = self.me.clone();
        canvas(
            |_, _, _| {},
            move |_, _, window, _| {
                let m = me.clone();
                window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let held = ev.pressed_button == Some(MouseButton::Left);
                    let x = ev.position.x;
                    let _ = m.update(cx, |this, cx| if held { this.drag_to(x, cx) } else { this.end_drag(cx) });
                });
                let m = me.clone();
                window.on_mouse_event(move |ev: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble && ev.button == MouseButton::Left {
                        let _ = m.update(cx, |this, cx| this.end_drag(cx));
                    }
                });
            },
        )
        .absolute()
        .size_0()
    }

    fn stage(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has = self.path.is_some();
        div()
            .relative()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
            // Click / double-click / drag-to-move on the video itself.
            .child(
                div()
                    .id("hit")
                    .absolute()
                    .inset_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                            this.focus.focus(window, cx);
                            this.video_down(ev.position, ev.click_count, window, cx)
                        }),
                    )
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.video_up(cx)))
                    .on_scroll(cx.listener(|this, ev: &ScrollEvent, _, cx| {
                        let dy = f32::from(ev.delta.pixel_delta(px(16.)).y);
                        if dy.abs() > 0.5 && this.path.is_some() {
                            this.set_volume(this.volume + if dy > 0.0 { 5.0 } else { -5.0 }, cx);
                            this.osd(format!("Volume {}%", this.volume));
                        }
                    })),
            )
            .when(!has, |d| d.child(self.empty(cx)))
            .child(self.titlebar(window))
            .when_some(self.osd.clone(), |d, (text, _)| {
                d.child(
                    div()
                        .absolute()
                        .top(px(50.))
                        .left(px(18.))
                        .px(px(12.))
                        .py(px(7.))
                        .rounded(px(10.))
                        .bg(glass_strong())
                        .border_1()
                        .border_color(stroke())
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .font_features(tnum())
                        .child(div().size(px(7.)).rounded(px(4.)).bg(clip()))
                        .child(text),
                )
            })
            .when_some(self.flash, |d, (name, _)| {
                d.child(
                    div().absolute().inset_0().flex().items_center().justify_center().child(
                        div().size(px(72.)).rounded(px(36.)).bg(black().opacity(0.5)).flex().items_center().justify_center().child(icon(name, 28., text())),
                    ),
                )
            })
            .when(has && self.clip_e < 0.999, |d| d.child(self.osc(cx)))
            .when(has && self.sheet_e > 0.001, |d| d.child(self.sheet(window, cx)))
            .child(self.job_cards(cx))
            .child(self.toast_list())
    }

    fn titlebar(&self, window: &mut Window) -> impl IntoElement {
        let title = self.path.as_deref().map(fmt::basename).unwrap_or_default();
        let fs = window.is_fullscreen();
        let caption = |id: &'static str, name: &'static str, area: WindowControlArea, close: bool| {
            div()
                .id(id)
                .w(px(CAPTION_W))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .window_control_area(area)
                // Occlude so the video surface underneath never sees these clicks (it would toggle pause).
                .occlude()
                .text_color(text2())
                .hover(move |s| if close { s.bg(rgba_white(0x2e)).text_color(text()) } else { s.bg(hover()).text_color(text()) })
                .child(icon(name, 16., text2()))
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(TITLEBAR_H))
            .flex()
            .items_center()
            .pl(px(14.))
            .bg(linear_gradient(180., linear_color_stop(black().opacity(0.55), 0.), linear_color_stop(black().opacity(0.), 1.)))
            // Drag strip as a sibling of the caption buttons, never their parent: GPUI returns the
            // first window-control hitbox under the mouse, so a wrapping Drag area would swallow
            // min/max/close (they'd all hit-test as HTCAPTION).
            .child(div().id("drag").absolute().top_0().bottom_0().left_0().right(px(if fs { 0. } else { CAPTION_W * 3. })).window_control_area(WindowControlArea::Drag).occlude())
            .when(self.idle, |d| d.opacity(0.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text2())
                    .child(icon("logo", 16., text()))
                    .child("Clipdat"),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(div().max_w(relative(0.5)).overflow_hidden().whitespace_nowrap().text_ellipsis().child(title)),
            )
            .when(!fs, |d| {
                d.child(
                    div()
                        .ml_auto()
                        .h_full()
                        .flex()
                        .child(caption("min", "min", WindowControlArea::Min, false))
                        .child(caption("max", "max", WindowControlArea::Max, false))
                        .child(caption("close", "close", WindowControlArea::Close, true)),
                )
            })
            .when(fs, |d| d.child(div().ml_auto()))
    }

    fn empty(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .bg(code_bg())
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(14.))
                    .child(
                        div()
                            .size(px(76.))
                            .rounded(px(19.))
                            .bg(linear_gradient(180., linear_color_stop(gpui::rgb(0x262626), 0.), linear_color_stop(gpui::rgb(0x0a0a0a), 1.)))
                            .border_1()
                            .border_color(stroke2())
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon("logo", 50., text())),
                    )
                    .child(div().mt(px(4.)).text_size(px(11.)).font_weight(FontWeight::SEMIBOLD).text_color(text3()).child("CLIPDAT"))
                    .child(div().text_size(px(20.)).font_weight(FontWeight::BOLD).child("Open a video"))
                    .child(div().text_size(px(12.5)).text_color(text3()).child("Drop a recording here, or press Ctrl+O"))
                    .child(div().mt(px(6.)).child(self.button("choose", Some("film"), "Choose file", true, cx, |this, w, cx| this.open_dialog(w, cx)))),
            )
    }

    fn osc(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let d = self.duration.max(0.001);
        let p = (self.time / d).clamp(0.0, 1.0) as f32;
        let vol = self.volume;
        div()
            .absolute()
            .bottom(px(22. - 36. * self.clip_e))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .opacity(if self.idle { 0. } else { 1. - self.clip_e })
            .child(
                div()
                    .id("osc")
                    .occlude()
                    .w(px(620.))
                    .max_w(relative(0.94))
                    .px(px(14.))
                    .pt(px(10.))
                    .pb(px(12.))
                    .rounded(px(18.))
                    .bg(glass())
                    .border_1()
                    .border_color(stroke())
                    .shadow_lg()
                    .on_hover(cx.listener(|this, h: &bool, _, cx| {
                        this.over_controls = *h;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .child(self.ib("mute", if self.muted { "mute" } else { "vol" }, 20., 34., cx, |this, _, _| {
                                        this.mpv_cmd(vec![serde_json::json!("cycle"), serde_json::json!("mute")])
                                    }))
                                    .child(
                                        div()
                                            .id("volume")
                                            .relative()
                                            .w(px(76.))
                                            .h(px(18.))
                                            .ml(px(4.))
                                            .cursor_pointer()
                                            .flex()
                                            .items_center()
                                            .child(self.probe_el("volume"))
                                            .child(
                                                div()
                                                    .relative()
                                                    .w_full()
                                                    .h(px(4.))
                                                    .rounded(px(2.))
                                                    .bg(rgba_white(0x33))
                                                    .child(div().absolute().left_0().top_0().bottom_0().rounded(px(2.)).bg(if self.muted { text3() } else { text() }).w(relative((vol.min(100.) / 150.) as f32)))
                                                    .when(vol > 100.0, |d| d.child(div().absolute().top_0().bottom_0().bg(clip()).left(relative(100. / 150.)).w(relative(((vol - 100.) / 150.) as f32)))),
                                            )
                                            .on_mouse_down(MouseButton::Left, cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                                                this.begin_drag(Drag::Volume, ev.position.x, cx);
                                                cx.stop_propagation();
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(self.ib("rew", "rew", 20., 34., cx, |this, _, _| this.seek(this.time - 5.0, true)))
                                    .child(
                                        self.ib("pp", if self.paused { "play" } else { "pause" }, 22., 44., cx, |this, _, _| this.toggle_pause())
                                            .rounded(px(22.))
                                            .bg(rgba_white(0x1a))
                                            .opacity(1.),
                                    )
                                    .child(self.ib("ff", "ff", 20., 34., cx, |this, _, _| this.seek(this.time + 5.0, true))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .justify_end()
                                    .items_center()
                                    .gap(px(2.))
                                    .child(self.ib("clip", "scissors", 20., 34., cx, |this, _, _| this.set_clip_mode(true)))
                                    .child(self.ib("gear", "gear", 20., 34., cx, |this, _, _| this.settings_open = true))
                                    .child(self.ib("fs", "full", 20., 34., cx, |this, w, _| this.toggle_fullscreen(w))),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(11.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(text2())
                            .font_features(tnum())
                            .child(fmt::short(self.time))
                            .child(self.seek_bar(p, cx))
                            .child(format!("-{}", fmt::short(self.duration - self.time))),
                    ),
            )
    }

    fn seek_bar(&self, p: f32, cx: &mut Context<Self>) -> impl IntoElement {
        let d = self.duration.max(0.001);
        let hover_x = self.hover_seek;
        let bar = self.probe("seek");
        div()
            .id("seek")
            .relative()
            .flex_1()
            .h(px(18.))
            .cursor_pointer()
            .flex()
            .items_center()
            .child(self.probe_el("seek"))
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(if hover_x.is_some() { 6. } else { 4. }))
                    .rounded(px(3.))
                    .bg(rgba_white(0x29))
                    .child(div().absolute().left_0().top_0().bottom_0().rounded(px(3.)).bg(text()).w(relative(p)))
                    .children(self.segs.iter().map(|g| {
                        div().absolute().top_0().bottom_0().rounded(px(2.)).bg(clip()).left(relative((g.a / d) as f32)).w(relative(((g.b - g.a) / d) as f32))
                    })),
            )
            .when(hover_x.is_some(), |el| {
                el.child(div().absolute().size(px(13.)).rounded(px(7.)).bg(text()).left(relative(p)).ml(px(-6.5)).shadow_md())
            })
            .when_some(hover_x.zip(bar), |el, (x, b)| {
                let rel = x - f32::from(b.origin.x);
                let w = f32::from(b.size.width);
                let t = (rel / w.max(1.)).clamp(0., 1.) as f64 * d;
                el.child(self.preview(rel, w, t, 28.))
            })
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                this.hover_seek = Some(f32::from(ev.position.x));
                cx.notify();
            }))
            .on_hover(cx.listener(|this, h: &bool, _, cx| {
                if !*h {
                    this.hover_seek = None;
                    cx.notify();
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                this.begin_drag(Drag::Seek, ev.position.x, cx);
                cx.stop_propagation();
            }))
    }

    /// Thumbnail + timecode above a seek surface. `x` is relative to the surface.
    pub fn preview(&self, x: f32, width: f32, t: f64, bottom: f32) -> AnyElement {
        let th = fmt::nearest(&self.thumbs, t).map(|t| PathBuf::from(&t.path));
        let left = (x - 88.).clamp(-8., (width - 168.).max(-8.));
        div()
            .absolute()
            .bottom(px(bottom))
            .left(px(left))
            .w(px(176.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .when_some(th, |d, p| {
                d.child(
                    div()
                        .w(px(176.))
                        .h(px(99.))
                        .rounded(px(9.))
                        .overflow_hidden()
                        .bg(black())
                        .border_1()
                        .border_color(stroke2())
                        .shadow_lg()
                        .child(img(p).size_full().object_fit(ObjectFit::Cover)),
                )
            })
            .child(
                div()
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(black().opacity(0.75))
                    .text_size(px(11.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(text())
                    .font_features(tnum())
                    .child(fmt::long(t)),
            )
            .into_any_element()
    }

    fn job_cards(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .left(px(16.))
            .bottom(px(16.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .children(self.jobs.iter().map(|j| {
                let id = j.id;
                let e = &j.ev;
                let running = e.state == "running";
                let (line, color): (String, gpui::Hsla) = match e.state.as_str() {
                    "running" => {
                        let what = match e.phase.as_str() {
                            "re-encoding" => "Over the limit, re-encoding smaller".to_string(),
                            "merging" => "Merging segments".to_string(),
                            _ if j.label == "lossless" => "Copying streams".to_string(),
                            _ => j.label.clone(),
                        };
                        let fps = if e.fps > 0.0 { format!(" · {} fps", e.fps.round()) } else { String::new() };
                        (format!("{what} · {}%{fps}", (e.progress * 100.0).floor()), text2())
                    }
                    "done" => {
                        let tail = if e.message == "copied" {
                            " · copied · Ctrl+V in Discord".to_string()
                        } else if e.outputs.len() > 1 {
                            format!(" · {} files", e.outputs.len())
                        } else {
                            " · saved".to_string()
                        };
                        (format!("{} MB{tail}", fmt::mb(e.bytes as f64 / 1e6)), text2())
                    }
                    "cancelled" => ("Cancelled".into(), text2()),
                    _ => (e.message.clone(), bad()),
                };
                let bar_color = match e.state.as_str() {
                    "done" => ok(),
                    "error" => bad(),
                    _ => accent(),
                };
                let prog = if e.state == "done" { 1.0 } else { e.progress as f32 };
                div()
                    .id(("job", id as usize))
                    .occlude()
                    .w(px(376.))
                    .p(px(10.))
                    .rounded(px(14.))
                    .bg(glass_strong())
                    .border_1()
                    .border_color(stroke())
                    .shadow_lg()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(76.))
                                    .h(px(43.))
                                    .rounded(px(7.))
                                    .overflow_hidden()
                                    .bg(black())
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .flex_none()
                                    .map(|d| match &j.thumb {
                                        Some(t) => d.child(img(PathBuf::from(t)).size_full().object_fit(ObjectFit::Cover)),
                                        None => d.child(icon("film", 18., text3())),
                                    }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .child(div().text_size(px(12.5)).font_weight(FontWeight::BOLD).whitespace_nowrap().overflow_hidden().text_ellipsis().child(format!("{}.mp4", j.name)))
                                    .child(div().mt(px(2.)).text_size(px(11.5)).text_color(color).font_features(tnum()).child(line))
                                    .child(div().mt(px(7.)).h(px(4.)).rounded(px(2.)).bg(rgba_white(0x1f)).child(div().h_full().rounded(px(2.)).bg(bar_color).w(relative(prog)))),
                            )
                            .child(if running {
                                self.ib(("cancel", id as usize), "stop", 14., 30., cx, move |this, _, _| this.cancel_job(id))
                            } else {
                                self.ib(("dismiss", id as usize), "x", 16., 30., cx, move |this, _, _| this.jobs.retain(|j| j.id != id))
                            }),
                    )
                    .when(e.state == "done", |d| {
                        let out = e.outputs.first().cloned().unwrap_or_default();
                        d.child(
                            div()
                                .mt(px(10.))
                                .flex()
                                .gap(px(6.))
                                .child(self.button(("again", id as usize), Some("copy"), "Copy again", false, cx, move |this, _, cx| this.copy_outputs(id, cx)).flex_1().justify_center())
                                .child(self.button(("reveal", id as usize), Some("folder"), "Show in folder", false, cx, move |_, _, _| crate::backend::util::reveal_in_explorer(&out)).flex_1().justify_center()),
                        )
                    })
            }))
    }

    fn toast_list(&self) -> impl IntoElement {
        div()
            .absolute()
            .top(px(50.))
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .children(self.toasts.iter().map(|t| {
                div()
                    .max_w(px(520.))
                    .px(px(14.))
                    .py(px(9.))
                    .rounded(px(10.))
                    .bg(glass_strong())
                    .border_1()
                    .border_color(if t.error { bad().opacity(0.35) } else { stroke2() })
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(icon(if t.error { "warn" } else { "info" }, 16., if t.error { bad() } else { accent() }))
                    .child(t.text.clone())
            }))
    }

    fn menu_popup(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let m = self.menu.as_ref().unwrap();
        let a = m.anchor;
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut last_group: Option<&'static str> = None;
        for (i, o) in m.opts.iter().enumerate() {
            if o.group.is_some() && o.group != last_group {
                rows.push(
                    div()
                        .px(px(9.))
                        .pt(px(8.))
                        .pb(px(4.))
                        .when(i > 0, |d| d.mt(px(4.)).border_t_1().border_color(stroke()))
                        .text_size(px(10.5))
                        .font_weight(FontWeight::BOLD)
                        .text_color(text3())
                        .child(o.group.unwrap().to_uppercase())
                        .into_any_element(),
                );
                last_group = o.group;
            }
            let selected = i == m.selected;
            let disabled = o.disabled.is_some();
            let sub = o.disabled.clone().or(o.desc.clone());
            rows.push(
                div()
                    .id(("menu-item", i))
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .py(px(7.))
                    .rounded(px(8.))
                    .when(disabled, |d| d.opacity(0.35))
                    .when(!disabled, |d| {
                        d.cursor_pointer().hover(|s| s.bg(rgba_white(0x14))).on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(f) = this.menu.as_ref().map(|m| m.on_pick.clone()) {
                                f(this, i, window, cx);
                            }
                            this.close_menu();
                            this.changed(cx);
                        }))
                    })
                    .child(div().w(px(16.)).flex_none().when(selected, |d| d.child(icon("check", 14., accent()))))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .text_size(px(12.5))
                            .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .child(o.label.clone())
                            .when_some(sub, |d, s| d.child(div().mt(px(1.)).text_size(px(11.)).font_weight(FontWeight::MEDIUM).text_color(text3()).child(s))),
                    )
                    .into_any_element(),
            );
        }
        deferred(
            anchored().position(point(a.origin.x, a.origin.y + a.size.height + px(6.))).snap_to_window_with_margin(px(8.)).child(
                div()
                    .id("menu")
                    .occlude()
                    .min_w(a.size.width)
                    .max_h(px(320.))
                    .overflow_y_scroll()
                    .p(px(5.))
                    .rounded(px(12.))
                    .bg(menu_bg())
                    .border_1()
                    .border_color(stroke2())
                    .shadow_lg()
                    .font_family(FONT)
                    .text_color(text())
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.close_menu();
                        cx.notify();
                    }))
                    .children(rows),
            ),
        )
        .with_priority(10)
    }
}
