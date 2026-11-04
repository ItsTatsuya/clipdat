//! The Clipdat window: state, backend wiring and behavior. Rendering lives in `view.rs`.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::Path,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    AppContext, Bounds, Context, Entity, FocusHandle, KeyDownEvent, Pixels, Point, SharedString, Subscription,
    Task, Window,
};
use gpui_component::input::{InputEvent, InputState};
use serde_json::{json, Value};

use crate::backend::{
    self,
    export::{self, Adv, ExportReq, JobEvent, Jobs},
    gpu::GpuInfo,
    media::{self, MediaInfo, Thumb},
    mpv::Mpv,
    settings::{Codec, Mode, SegmentsPref, Settings},
    util, Event,
};
use crate::planner::{self, Plan};

pub const TITLEBAR_H: f32 = 40.0;
pub const DOCK_H: f32 = 164.0;
pub const SHEET_W: f32 = 392.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub a: f64,
    pub b: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    General,
    Export,
    Gpu,
    Keys,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Drag {
    Seek,
    Volume,
    Timeline,
    Edge { seg: usize, end: bool },
    Quality,
    AqStrength,
}

#[derive(Clone)]
pub struct Opt {
    pub label: SharedString,
    pub desc: Option<SharedString>,
    pub disabled: Option<SharedString>,
    pub group: Option<&'static str>,
}

impl Opt {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Opt { label: label.into(), desc: None, disabled: None, group: None }
    }
    pub fn desc(mut self, d: impl Into<SharedString>) -> Self {
        self.desc = Some(d.into());
        self
    }
    pub fn disabled_if(mut self, cond: bool, reason: &'static str) -> Self {
        if cond {
            self.disabled = Some(reason.into());
        }
        self
    }
    pub fn group(mut self, g: &'static str) -> Self {
        self.group = Some(g);
        self
    }
}

pub type Pick = Rc<dyn Fn(&mut Clipdat, usize, &mut Window, &mut Context<Clipdat>)>;

pub struct OpenMenu {
    pub id: &'static str,
    pub anchor: Bounds<Pixels>,
    pub opts: Vec<Opt>,
    pub selected: usize,
    pub on_pick: Pick,
}

pub struct JobView {
    pub id: u64,
    pub name: String,
    pub label: String,
    pub thumb: Option<String>,
    pub ev: JobEvent,
}

pub struct Toast {
    pub text: SharedString,
    pub error: bool,
    pub at: Instant,
}

pub struct Clipdat {
    pub me: gpui::WeakEntity<Clipdat>,
    pub focus: FocusHandle,
    pub hwnd: isize,
    pub mpv: Option<Arc<Mpv>>,
    pub tx: smol::channel::Sender<Event>,
    pub settings: Settings,
    save_task: Option<Task<()>>,

    // media
    pub path: Option<String>,
    pub info: Option<MediaInfo>,
    pub keyframes: Vec<f64>,
    pub thumbs: Vec<Thumb>,
    pub time: f64,
    pub duration: f64,
    pub paused: bool,
    pub volume: f64,
    pub muted: bool,
    resume_at: Option<f64>,
    prev_t: f64,
    scrubbing: bool,

    // clip
    pub clip_on: bool,
    /// Eased 0..1 progress of the clip dock and export sheet (see `anim.rs`).
    pub clip_tw: super::anim::Tween,
    pub sheet_tw: super::anim::Tween,
    pub clip_e: f32,
    pub sheet_e: f32,
    pub sheet_open: bool,
    pub segs: Vec<Seg>,
    pub act: usize,
    pub mode: Mode,
    pub mb: f64,
    pub codec: Codec,
    pub quality: f64,
    pub q_res: u32,
    pub q_fps: u32,
    pub merge: bool,

    // inputs
    pub name_input: Entity<InputState>,
    pub extra_input: Entity<InputState>,
    pub template_input: Entity<InputState>,
    pub limit_input: Entity<InputState>,
    pub preset_input: Entity<InputState>,
    pub naming_preset: bool,
    _subs: Vec<Subscription>,

    // ui
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    pub menu: Option<OpenMenu>,
    menu_closed: Option<(&'static str, Instant)>,
    pub drag: Option<Drag>,
    pub hover_seek: Option<f32>,
    pub hover_tl: Option<f32>,
    pub probes: Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>,
    pub osd: Option<(SharedString, Instant)>,
    pub flash: Option<(&'static str, Instant)>,
    pub toasts: Vec<Toast>,
    pub jobs: Vec<JobView>,
    pub gpu: Option<GpuInfo>,
    pub dropping: bool,
    pub idle: bool,
    pub over_controls: bool,
    last_activity: Instant,
    press: Option<Point<Pixels>>,
    /// Set when we post a window-move; GPUI echoes it back as one more mouse-down, which we skip.
    win_drag: bool,
    pending_click: Option<Task<()>>,
    last_margins: (f32, f32),
    pub cmd_copied: Option<Instant>,
}

static JOBS: std::sync::LazyLock<Jobs> = std::sync::LazyLock::new(Jobs::default);

impl Clipdat {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        hwnd: isize,
        tx: smol::channel::Sender<Event>,
        rx: smol::channel::Receiver<Event>,
        file: Option<String>,
    ) -> Self {
        let settings = Settings::load();
        let mut startup_error = None;
        let mpv = match backend::rt().block_on(Mpv::start(tx.clone(), hwnd)) {
            Ok(m) => Some(m),
            Err(e) => {
                startup_error = Some(format!("The video engine failed to start: {e}"));
                None
            }
        };

        let mk = |window: &mut Window, cx: &mut Context<Self>, placeholder: &str, value: &str| {
            let (p, v) = (placeholder.to_string(), value.to_string());
            cx.new(|cx| InputState::new(window, cx).placeholder(p).default_value(v))
        };
        let name_input = mk(window, cx, "clip name", "");
        let extra_input = mk(window, cx, "e.g. -highbitdepth 1", &settings.adv.extra);
        let template_input = mk(window, cx, "{name}_clip{n}", &settings.name_template);
        let limit_input = mk(window, cx, "25", &format!("{}", settings.custom_limit));
        let preset_input = mk(window, cx, "Preset name", "");

        let subs = vec![
            cx.subscribe_in(&extra_input, window, |this, s, ev: &InputEvent, _w, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.settings.adv.extra = s.read(cx).value().to_string();
                    this.changed(cx);
                }
            }),
            cx.subscribe_in(&template_input, window, |this, s, ev: &InputEvent, _w, cx| {
                if matches!(ev, InputEvent::Change) {
                    let v = s.read(cx).value().to_string();
                    this.settings.name_template = if v.trim().is_empty() { "{name}_clip{n}".into() } else { v };
                    this.changed(cx);
                }
            }),
            cx.subscribe_in(&limit_input, window, |this, s, ev: &InputEvent, _w, cx| {
                if matches!(ev, InputEvent::Change) {
                    if let Ok(v) = s.read(cx).value().trim().parse::<f64>() {
                        if (1.0..=4000.0).contains(&v) {
                            this.settings.custom_limit = v;
                            this.changed(cx);
                        }
                    }
                }
            }),
            cx.subscribe_in(&preset_input, window, |this, s, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => {
                    let name = s.read(cx).value().trim().to_string();
                    if !name.is_empty() {
                        this.settings.presets.retain(|p| p.name != name);
                        this.settings.presets.push(backend::settings::Preset { name, adv: this.settings.adv.clone() });
                        this.naming_preset = false;
                        this.focus.focus(window, cx);
                        this.changed(cx);
                    }
                }
                InputEvent::Blur => {
                    this.naming_preset = false;
                    cx.notify();
                }
                _ => {}
            }),
        ];

        let mut this = Clipdat {
            me: cx.entity().downgrade(),
            focus: cx.focus_handle(),
            hwnd,
            mpv,
            tx,
            mode: settings.default_mode,
            mb: settings.default_limit,
            codec: settings.default_codec,
            merge: settings.segments != SegmentsPref::Separate,
            volume: settings.volume,
            settings,
            save_task: None,
            path: None,
            info: None,
            keyframes: Vec::new(),
            thumbs: Vec::new(),
            time: 0.0,
            duration: 0.0,
            paused: true,
            muted: false,
            resume_at: None,
            prev_t: 0.0,
            scrubbing: false,
            clip_on: false,
            clip_tw: super::anim::Tween::new(260),
            sheet_tw: super::anim::Tween::new(240),
            clip_e: 0.0,
            sheet_e: 0.0,
            sheet_open: false,
            segs: Vec::new(),
            act: 0,
            quality: 68.0,
            q_res: 0,
            q_fps: 0,
            name_input,
            extra_input,
            template_input,
            limit_input,
            preset_input,
            naming_preset: false,
            _subs: subs,
            settings_open: false,
            settings_tab: SettingsTab::General,
            menu: None,
            menu_closed: None,
            drag: None,
            hover_seek: None,
            hover_tl: None,
            probes: Rc::new(RefCell::new(HashMap::new())),
            osd: None,
            flash: None,
            toasts: Vec::new(),
            jobs: Vec::new(),
            gpu: None,
            dropping: false,
            idle: false,
            over_controls: false,
            last_activity: Instant::now(),
            press: None,
            win_drag: false,
            pending_click: None,
            last_margins: (-1.0, -1.0),
            cmd_copied: None,
        };

        if let Some(e) = startup_error {
            this.toasts.push(Toast { text: e.into(), error: true, at: Instant::now() + Duration::from_secs(3600) });
        }
        this.mpv_set("volume", json!(this.volume));
        this.mpv_set("hwdec", json!(this.settings.hwdec));

        // Backend events → UI.
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(ev) = rx.recv().await {
                if this.update_in(cx, |this, window, cx| this.on_event(ev, window, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        // Housekeeping tick: auto-hide, OSD/toast expiry.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(150)).await;
            if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                break;
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            let info = backend::run(backend::gpu::detect()).await;
            let _ = this.update(cx, |this, cx| {
                this.gpu = Some(info);
                cx.notify();
            });
        })
        .detach();

        if let Some(f) = file {
            this.open_file(f, window, cx);
        }
        this
    }

    // ---------------------------------------------------------------- helpers

    pub fn mpv_cmd(&self, args: Vec<Value>) {
        if let Some(m) = &self.mpv {
            m.command_nowait(args);
        }
    }
    pub fn mpv_set(&self, prop: &str, v: Value) {
        self.mpv_cmd(vec![json!("set_property"), json!(prop), v]);
    }

    /// Persist settings shortly after the last change.
    pub fn changed(&mut self, cx: &mut Context<Self>) {
        let s = self.settings.clone();
        self.save_task = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(Duration::from_millis(400)).await;
            cx.background_executor().spawn(async move { s.save() }).await;
        }));
        cx.notify();
    }

    pub fn osd(&mut self, text: impl Into<SharedString>) {
        self.osd = Some((text.into(), Instant::now()));
    }

    pub fn toast(&mut self, text: impl Into<SharedString>, error: bool) {
        self.toasts.push(Toast { text: text.into(), error, at: Instant::now() });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub fn wake(&mut self, cx: &mut Context<Self>) {
        self.last_activity = Instant::now();
        if self.idle {
            self.idle = false;
            cx.notify();
        }
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let mut dirty = false;
        let idle = self.settings.auto_hide_ms > 0
            && self.path.is_some()
            && !self.paused
            && !self.clip_on
            && !self.settings_open
            && !self.over_controls
            && self.menu.is_none()
            && self.last_activity.elapsed() > Duration::from_millis(self.settings.auto_hide_ms);
        if idle != self.idle {
            self.idle = idle;
            dirty = true;
        }
        if self.osd.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_millis(1300)) {
            self.osd = None;
            dirty = true;
        }
        if self.flash.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_millis(450)) {
            self.flash = None;
            dirty = true;
        }
        let before = self.toasts.len();
        self.toasts.retain(|t| t.at.elapsed() < Duration::from_millis(if t.error { 6000 } else { 2500 }));
        dirty |= before != self.toasts.len();
        if self.cmd_copied.is_some_and(|t| t.elapsed() > Duration::from_millis(1200)) {
            self.cmd_copied = None;
            dirty = true;
        }
        if dirty {
            cx.notify();
        }
    }

    // ---------------------------------------------------------------- backend events

    fn on_event(&mut self, ev: Event, window: &mut Window, cx: &mut Context<Self>) {
        match ev {
            Event::Prop(name, data) => {
                match name.as_str() {
                    "time-pos" => {
                        if let Some(t) = data.as_f64() {
                            if !self.scrubbing {
                                self.time = t;
                                self.loop_segment();
                            }
                        }
                    }
                    "duration" => {
                        if let Some(d) = data.as_f64() {
                            self.duration = d;
                        }
                    }
                    "pause" => self.paused = data.as_bool().unwrap_or(true),
                    "volume" => {
                        if let Some(v) = data.as_f64() {
                            self.volume = v.round();
                        }
                    }
                    "mute" => self.muted = data.as_bool().unwrap_or(false),
                    _ => return,
                }
                cx.notify();
            }
            Event::Mpv(v) => match v["event"].as_str() {
                Some("file-loaded") => {
                    if let Some(t) = self.resume_at.take() {
                        self.mpv_cmd(vec![json!("seek"), json!(t), json!("absolute+exact")]);
                        self.toast(format!("Resumed at {}", crate::fmt::short(t)), false);
                        cx.notify();
                    }
                }
                Some("mpv-exited") => {
                    self.toast("The video engine stopped. Restart Clipdat.", true);
                    cx.notify();
                }
                _ => {}
            },
            Event::Job(e) => self.on_job(e, cx),
            Event::OpenFile(p) => {
                window.activate_window();
                if !p.is_empty() && Path::new(&p).is_file() {
                    self.open_file(p, window, cx);
                }
            }
        }
    }

    fn on_job(&mut self, e: JobEvent, cx: &mut Context<Self>) {
        let Some(j) = self.jobs.iter_mut().find(|j| j.id == e.id) else { return };
        let state = e.state.clone();
        let msg = e.message.clone();
        let mut merged = e;
        if merged.outputs.is_empty() {
            merged.outputs = j.ev.outputs.clone();
        }
        if merged.fps == 0.0 {
            merged.fps = j.ev.fps;
        }
        j.ev = merged;
        if state == "error" {
            self.toast(msg, true);
        }
        cx.notify();
    }

    // ---------------------------------------------------------------- player

    pub fn open_file(&mut self, p: String, window: &mut Window, cx: &mut Context<Self>) {
        self.remember_position();
        self.path = Some(p.clone());
        self.info = None;
        self.keyframes.clear();
        self.thumbs.clear();
        self.time = 0.0;
        self.duration = 0.0;
        self.clip_on = false;
        self.sheet_open = false;
        self.segs.clear();
        self.act = 0;
        self.mode = self.settings.default_mode;
        self.mb = self.settings.default_limit;
        self.codec = self.settings.default_codec;
        self.merge = self.settings.segments != SegmentsPref::Separate;
        self.name_input.update(cx, |s, cx| s.set_value("", window, cx));
        self.resume_at = if self.settings.resume { self.settings.positions.get(&p).copied() } else { None };
        window.set_window_title(&format!("{} — Clipdat", crate::fmt::basename(&p)));
        self.mpv_cmd(vec![json!("loadfile"), json!(p)]);
        self.mpv_set("pause", json!(false));

        let path = p.clone();
        cx.spawn(async move |this, cx| {
            let pp = path.clone();
            let info = backend::run(async move { media::probe(&pp).await }).await;
            let Ok(info) = info else {
                let _ = this.update(cx, |this, cx| {
                    this.toast(format!("Couldn't read {}", crate::fmt::basename(&path)), true);
                    cx.notify();
                });
                return;
            };
            let start = info.start;
            let dur = info.duration;
            let ok = this
                .update(cx, |this, cx| {
                    if this.path.as_deref() != Some(&path) {
                        return false;
                    }
                    if this.duration == 0.0 {
                        this.duration = info.duration;
                    }
                    this.info = Some(info);
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !ok {
                return;
            }
            let pk = path.clone();
            let this2 = this.clone();
            cx.spawn(async move |cx| {
                if let Ok(k) = backend::run(async move { media::keyframes(&pk).await }).await {
                    let _ = this2.update(cx, |this, cx| {
                        this.keyframes = k.into_iter().map(|t| t - start).collect();
                        cx.notify();
                    });
                }
            })
            .detach();
            let count = ((dur / 2.0).round() as u32).clamp(40, 240);
            let pt = path.clone();
            let cache = backend::settings::cache_dir();
            if let Ok(th) = backend::run(async move { media::thumbnails(&cache, &pt, dur, count).await }).await {
                let _ = this.update(cx, |this, cx| {
                    if this.path.as_deref() == Some(&path) {
                        this.thumbs = th.into_iter().map(|mut t| { t.t -= start; t }).collect();
                        cx.notify();
                    }
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn remember_position(&mut self) {
        let Some(p) = self.path.clone() else { return };
        if !self.settings.resume {
            return;
        }
        if self.time > 5.0 && self.time < self.duration - 5.0 {
            self.settings.positions.insert(p, (self.time * 10.0).round() / 10.0);
        } else {
            self.settings.positions.remove(&p);
        }
        while self.settings.positions.len() > 300 {
            let k = self.settings.positions.keys().next().cloned().unwrap();
            self.settings.positions.remove(&k);
        }
        self.settings.save();
    }

    pub fn on_quit(&mut self) {
        self.remember_position();
        self.settings.save();
    }

    pub fn seek(&mut self, t: f64, exact: bool) {
        let t = t.clamp(0.0, if self.duration > 0.0 { self.duration } else { t.max(0.0) });
        self.time = t;
        self.mpv_cmd(vec![json!("seek"), json!(t), json!(if exact { "absolute+exact" } else { "absolute+keyframes" })]);
    }

    pub fn toggle_pause(&mut self) {
        if self.path.is_none() {
            return;
        }
        self.flash = Some((if self.paused { "play" } else { "pause" }, Instant::now()));
        self.mpv_cmd(vec![json!("cycle"), json!("pause")]);
    }

    pub fn frame_step(&mut self, fwd: bool) {
        self.mpv_cmd(vec![json!(if fwd { "frame-step" } else { "frame-back-step" })]);
    }

    pub fn set_volume(&mut self, v: f64, cx: &mut Context<Self>) {
        self.volume = v.clamp(0.0, 150.0).round();
        self.settings.volume = self.volume;
        self.mpv_set("volume", json!(self.volume));
        self.changed(cx);
    }

    pub fn toggle_fullscreen(&mut self, window: &mut Window) {
        window.toggle_fullscreen();
    }

    /// Loop the active segment while playing in clip mode.
    fn loop_segment(&mut self) {
        let t = self.time;
        if self.clip_on && !self.paused {
            if let Some(g) = self.segs.get(self.act).copied() {
                if self.prev_t < g.b && t >= g.b && t - self.prev_t < 1.0 {
                    self.seek(g.a, true);
                }
            }
        }
        self.prev_t = self.time;
    }

    /// Keep the picture clear of the dock and export sheet.
    pub fn sync_margins(&mut self, window: &Window) {
        let size = window.viewport_size();
        let (w, h) = (f32::from(size.width).max(1.0), f32::from(size.height).max(1.0));
        let bottom = (DOCK_H * self.clip_e / h).min(0.6);
        let right = ((SHEET_W + 20.0) * self.sheet_e / w).min(0.6);
        if (bottom, right) != self.last_margins {
            self.last_margins = (bottom, right);
            self.mpv_set("video-margin-ratio-bottom", json!(bottom));
            self.mpv_set("video-margin-ratio-right", json!(right));
        }
    }

    // ---------------------------------------------------------------- mouse on the video

    pub fn video_down(&mut self, pos: Point<Pixels>, clicks: usize, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.win_drag) {
            return;
        }
        // Title-bar clicks (min/max/close, drag strip) reach us too, as GPUI forwards non-client
        // presses as ordinary mouse-downs. They must not count as "click the video to pause".
        if f32::from(pos.y) < TITLEBAR_H && !window.is_fullscreen() {
            return;
        }
        self.close_menu();
        if clicks >= 2 {
            self.pending_click = None;
            self.press = None;
            self.toggle_fullscreen(window);
        } else {
            self.press = Some(pos);
        }
        cx.notify();
    }

    pub fn video_move(&mut self, pos: Point<Pixels>, left_down: bool, window: &Window) {
        // A press that ended inside a native move loop never sends us its mouse-up: drop it.
        if !left_down {
            self.press = None;
            self.win_drag = false;
            return;
        }
        if let Some(p) = self.press {
            let d = (f32::from(pos.x) - f32::from(p.x)).abs() + (f32::from(pos.y) - f32::from(p.y)).abs();
            if d > 5.0 {
                self.press = None;
                if !window.is_fullscreen() {
                    self.win_drag = true;
                    util::begin_window_drag(self.hwnd);
                }
            }
        }
    }

    pub fn video_up(&mut self, cx: &mut Context<Self>) {
        if self.press.take().is_none() || self.path.is_none() {
            return;
        }
        self.pending_click = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(220)).await;
            let _ = this.update(cx, |this, cx| {
                this.toggle_pause();
                cx.notify();
            });
        }));
    }

    // ---------------------------------------------------------------- clip

    fn ensure_seg(&mut self) {
        if !self.segs.is_empty() {
            return;
        }
        // Start at the playhead; near the end, take the last 30 s ("clip what just happened").
        let d = self.duration;
        let a = if self.time + 30.0 > d { (d - 30.0).max(0.0) } else { self.time };
        self.segs = vec![Seg { a, b: (a + 30.0).min(d) }];
        self.act = 0;
    }

    pub fn set_clip_mode(&mut self, on: bool) {
        if self.path.is_none() {
            return;
        }
        if on {
            self.ensure_seg();
        } else {
            self.sheet_open = false;
        }
        self.clip_on = on;
    }

    pub fn set_in(&mut self) {
        self.ensure_seg();
        let t = self.time;
        let g = &mut self.segs[self.act];
        g.a = t.min(g.b - 0.2).max(0.0);
        let a = g.a;
        self.osd(format!("In  {}", crate::fmt::long(a)));
    }

    pub fn set_out(&mut self) {
        self.ensure_seg();
        let t = self.time;
        let d = self.duration;
        let g = &mut self.segs[self.act];
        g.b = t.max(g.a + 0.2).min(d);
        let b = g.b;
        self.osd(format!("Out  {}", crate::fmt::long(b)));
    }

    pub fn new_segment(&mut self) {
        let d = self.duration;
        let a = self.time.min((d - 2.0).max(0.0));
        self.segs.push(Seg { a, b: (a + 10.0).min(d) });
        self.segs.sort_by(|x, y| x.a.partial_cmp(&y.a).unwrap());
        self.act = self.segs.iter().position(|g| g.a == a).unwrap_or(0);
        self.osd("New segment");
    }

    pub fn delete_segment(&mut self, i: usize) {
        if self.segs.len() > 1 && i < self.segs.len() {
            self.segs.remove(i);
            self.act = self.act.min(self.segs.len() - 1);
        }
    }

    pub fn move_edge(&mut self, i: usize, end: bool, t: f64) {
        let d = self.duration;
        if let Some(g) = self.segs.get_mut(i) {
            if end {
                g.b = t.max(g.a + 0.2).min(d);
            } else {
                g.a = t.min(g.b - 0.2).max(0.0);
            }
        }
    }

    pub fn total_duration(&self) -> f64 {
        self.segs.iter().map(|g| (g.b - g.a).max(0.0)).sum()
    }

    fn audio_track_count(&self) -> usize {
        self.info.as_ref().map(|i| i.audio.len()).unwrap_or(0)
    }

    pub fn audio_kbps(&self) -> u32 {
        let a = &self.settings.adv;
        let n = a.tracks.iter().filter(|t| **t < self.audio_track_count()).count();
        let n = if a.mix { n.min(1) } else { n } as u32;
        match a.acodec.as_str() {
            _ if n == 0 => 0,
            "none" => 0,
            "copy" => 192 * n,
            _ => a.abitrate * n,
        }
    }

    pub fn plan(&self) -> Plan {
        let i = self.info.as_ref();
        let dur = if self.merge || self.segs.len() < 2 {
            self.total_duration()
        } else {
            self.segs.iter().map(|g| g.b - g.a).fold(0.1, f64::max)
        };
        planner::plan(
            dur,
            self.mb,
            self.codec,
            i.map(|i| i.width).unwrap_or(1920),
            i.map(|i| i.height).unwrap_or(1080),
            i.map(|i| i.fps).unwrap_or(60.0),
            self.audio_kbps(),
        )
    }

    /// Estimated output size (MB) for the current mode.
    pub fn estimate_mb(&self) -> f64 {
        let i = self.info.as_ref();
        match self.mode {
            Mode::Size => self.plan().size_mb,
            Mode::Lossless => self.total_duration() * i.map(|i| i.bitrate as f64).unwrap_or(40e6) / 8.0 / 1e6,
            Mode::Quality => {
                let h = if self.q_res > 0 { self.q_res } else { i.map(|i| i.height).unwrap_or(1080) };
                planner::estimate_quality_mb(self.total_duration(), self.quality, self.codec, h)
                    + self.total_duration() * self.audio_kbps() as f64 / 8000.0
            }
        }
    }

    pub fn keyframe_before(&self, t: f64) -> f64 {
        let k = &self.keyframes;
        match k.partition_point(|x| *x <= t + 1e-6) {
            0 => 0.0,
            i => k[i - 1],
        }
    }

    pub fn default_name(&self, cx: &gpui::App) -> String {
        let typed = self.name_input.read(cx).value().trim().to_string();
        if !typed.is_empty() {
            return typed;
        }
        let Some(p) = &self.path else { return "clip".into() };
        let stem = crate::fmt::stem(p);
        let n = self.settings.counters.get(&stem).copied().unwrap_or(0) + 1;
        let now = std::time::SystemTime::now();
        let secs = now.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.settings
            .name_template
            .replace("{name}", &stem)
            .replace("{n}", &format!("{n:02}"))
            .replace("{date}", &crate::fmt::date(secs))
            .replace("{time}", &crate::fmt::time_of_day(secs))
    }

    pub fn build_request(&self, cx: &gpui::App) -> ExportReq {
        let i = self.info.as_ref();
        let p = self.plan();
        let (height, fps) = match self.mode {
            Mode::Size => (p.height, p.fps as f64),
            Mode::Quality => (
                if self.q_res > 0 { self.q_res } else { i.map(|i| i.height).unwrap_or(0) },
                if self.q_fps > 0 { self.q_fps as f64 } else { i.map(|i| i.fps).unwrap_or(0.0) },
            ),
            Mode::Lossless => (0, 0.0),
        };
        let mut adv: Adv = self.settings.adv.clone();
        if self.mode == Mode::Size && (adv.rc == "cq" || adv.rc == "qp") {
            adv.rc = "vbr".into();
        }
        if self.codec == Codec::H264 {
            adv.depth = 8;
        }
        ExportReq {
            input: self.path.clone().unwrap_or_default(),
            segments: self.segs.iter().map(|g| [g.a, g.b]).collect(),
            mode: match self.mode {
                Mode::Size => "size",
                Mode::Quality => "quality",
                Mode::Lossless => "lossless",
            }
            .into(),
            limit_mb: self.mb,
            codec: self.codec.id().into(),
            quality: self.quality,
            height,
            fps,
            video_kbps: if self.mode == Mode::Size { p.video_kbps } else { planner::quality_kbps(self.quality, height.max(1)) },
            merge: self.merge,
            out_dir: self.settings.output_dir.clone(),
            name: self.default_name(cx),
            copy_clipboard: self.settings.copy_clipboard,
            reveal: self.settings.reveal,
            src_width: i.map(|i| i.width).unwrap_or(0),
            src_height: i.map(|i| i.height).unwrap_or(0),
            src_fps: i.map(|i| i.fps).unwrap_or(0.0),
            audio_tracks: self.audio_track_count(),
            adv,
        }
    }

    pub fn start_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.path.is_none() || self.segs.is_empty() {
            return;
        }
        if self.mode == Mode::Size && !self.plan().ok {
            self.toast(format!("This clip is too long to look good at {} MB. Trim it or pick a bigger limit.", self.mb), true);
            cx.notify();
            return;
        }
        let req = self.build_request(cx);
        let name = req.name.clone();
        let label = if self.mode == Mode::Lossless { "lossless".to_string() } else { format!("NVENC {}", self.codec.label()) };
        let thumb = crate::fmt::nearest(&self.thumbs, self.segs[0].a + 1.0).map(|t| t.path.clone());
        let started = {
            let _g = backend::rt().enter();
            export::start(self.tx.clone(), &JOBS, req)
        };
        match started {
            Ok(id) => {
                self.jobs.retain(|j| j.ev.state == "running");
                self.jobs.push(JobView {
                    id,
                    name,
                    label,
                    thumb,
                    ev: JobEvent { id, state: "running".into(), phase: "starting".into(), ..Default::default() },
                });
                if self.name_input.read(cx).value().trim().is_empty() {
                    if let Some(p) = &self.path {
                        *self.settings.counters.entry(crate::fmt::stem(p)).or_insert(0) += 1;
                    }
                } else {
                    self.name_input.update(cx, |s, cx| s.set_value("", window, cx));
                }
                self.sheet_open = false;
                self.changed(cx);
            }
            Err(e) => self.toast(format!("Export failed to start: {e}"), true),
        }
        cx.notify();
    }

    pub fn cancel_job(&mut self, id: u64) {
        JOBS.cancel(id);
    }

    pub fn copy_outputs(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(j) = self.jobs.iter().find(|j| j.id == id) {
            match util::copy_files_to_clipboard(&j.ev.outputs) {
                Ok(()) => self.toast("Copied. Paste it in Discord", false),
                Err(e) => self.toast(e.to_string(), true),
            }
        }
        cx.notify();
    }

    pub fn open_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open video".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                if let Some(p) = paths.into_iter().next() {
                    let _ = this.update_in(cx, |this, window, cx| this.open_file(p.to_string_lossy().into_owned(), window, cx));
                }
            }
        })
        .detach();
    }

    pub fn choose_output_dir(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose output folder".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await {
                if let Some(p) = paths.into_iter().next() {
                    let _ = this.update(cx, |this, cx| {
                        this.settings.output_dir = p.to_string_lossy().into_owned();
                        this.changed(cx);
                    });
                }
            }
        })
        .detach();
    }

    // ---------------------------------------------------------------- menus

    pub fn open_menu(&mut self, id: &'static str, opts: Vec<Opt>, selected: usize, on_pick: Pick) {
        if let Some((cid, t)) = self.menu_closed {
            if cid == id && t.elapsed() < Duration::from_millis(250) {
                return; // the click that closed it shouldn't reopen it
            }
        }
        let anchor = self.probes.borrow().get(id).copied().unwrap_or_default();
        self.menu = Some(OpenMenu { id, anchor, opts, selected, on_pick });
    }

    pub fn close_menu(&mut self) {
        if let Some(m) = self.menu.take() {
            self.menu_closed = Some((m.id, Instant::now()));
        }
    }

    // ---------------------------------------------------------------- drags

    pub fn probe(&self, id: &str) -> Option<Bounds<Pixels>> {
        self.probes.borrow().get(id).copied()
    }

    fn frac(&self, id: &str, x: Pixels) -> f64 {
        let Some(b) = self.probe(id) else { return 0.0 };
        let w = f32::from(b.size.width).max(1.0);
        ((f32::from(x) - f32::from(b.origin.x)) / w).clamp(0.0, 1.0) as f64
    }

    pub fn begin_drag(&mut self, d: Drag, x: Pixels, cx: &mut Context<Self>) {
        self.close_menu();
        self.drag = Some(d);
        if matches!(d, Drag::Seek | Drag::Timeline | Drag::Edge { .. }) {
            self.scrubbing = true;
        }
        if let Drag::Edge { seg, .. } = d {
            self.act = seg;
        }
        self.drag_to(x, cx);
    }

    pub fn drag_to(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let Some(d) = self.drag else { return };
        let dur = self.duration.max(0.001);
        match d {
            Drag::Seek => {
                let t = self.frac("seek", x) * dur;
                self.seek(t, false);
            }
            Drag::Timeline => {
                let t = self.frac("timeline", x) * dur;
                self.seek(t, false);
            }
            Drag::Edge { seg, end } => {
                let t = self.frac("timeline", x) * dur;
                self.move_edge(seg, end, t);
                if let Some(g) = self.segs.get(seg).copied() {
                    self.seek(if end { g.b } else { g.a }, false);
                }
            }
            Drag::Volume => {
                let v = self.frac("volume", x) * 150.0;
                self.set_volume(v, cx);
            }
            Drag::Quality => {
                self.quality = (self.frac("quality", x) * 100.0).round();
            }
            Drag::AqStrength => {
                self.settings.adv.aq_strength = (1.0 + self.frac("aq", x) * 14.0).round() as u32;
                self.changed(cx);
            }
        }
        cx.notify();
    }

    pub fn end_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(d) = self.drag.take() {
            if matches!(d, Drag::Seek | Drag::Timeline | Drag::Edge { .. }) {
                self.scrubbing = false;
                let t = self.time;
                self.seek(t, true);
            }
            cx.notify();
        }
    }

    // ---------------------------------------------------------------- keyboard

    pub fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let k = ev.keystroke.key.as_str();
        let ctrl = ev.keystroke.modifiers.control;
        let shift = ev.keystroke.modifiers.shift;
        self.wake(cx);
        if k == "escape" {
            if self.menu.is_some() {
                self.close_menu();
            } else if self.settings_open {
                self.settings_open = false;
            } else if self.sheet_open {
                self.sheet_open = false;
            } else if self.clip_on {
                self.set_clip_mode(false);
            } else if window.is_fullscreen() {
                window.toggle_fullscreen();
            }
            self.focus.focus(window, cx);
            cx.notify();
            return;
        }
        // Text fields keep their keys.
        if !self.focus.is_focused(window) {
            return;
        }
        if ctrl && k == "o" {
            self.open_dialog(window, cx);
            return;
        }
        if ctrl && k == "," {
            self.settings_open = !self.settings_open;
            cx.notify();
            return;
        }
        if self.settings_open || self.path.is_none() {
            return;
        }
        match k {
            "space" => self.toggle_pause(),
            "c" if !ctrl => self.set_clip_mode(!self.clip_on),
            "i" if self.clip_on => self.set_in(),
            "o" if self.clip_on => self.set_out(),
            "n" if self.clip_on => self.new_segment(),
            "delete" if self.clip_on => self.delete_segment(self.act),
            "e" if ctrl => {
                self.set_clip_mode(true);
                self.start_export(window, cx);
            }
            "e" => {
                self.set_clip_mode(true);
                self.sheet_open = true;
            }
            "," => self.frame_step(false),
            "." => self.frame_step(true),
            "left" => self.seek(self.time - if shift { 1.0 } else { 5.0 }, true),
            "right" => self.seek(self.time + if shift { 1.0 } else { 5.0 }, true),
            "up" => self.set_volume(self.volume + 5.0, cx),
            "down" => self.set_volume(self.volume - 5.0, cx),
            "m" => self.mpv_cmd(vec![json!("cycle"), json!("mute")]),
            "f" => self.toggle_fullscreen(window),
            _ => return,
        }
        cx.stop_propagation();
        cx.notify();
    }

    pub fn register_default(&mut self, cx: &mut Context<Self>) {
        match util::register_file_types() {
            Ok(()) => {
                util::open_default_apps_settings();
                self.toast("Registered. Pick Clipdat for your video types in Windows Settings", false);
            }
            Err(e) => self.toast(format!("Couldn't register file types: {e}"), true),
        }
        cx.notify();
    }

    pub fn copy_command(&mut self, cx: &mut Context<Self>) {
        let cmd = export::preview(&self.build_request(cx));
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(cmd));
        self.cmd_copied = Some(Instant::now());
        cx.notify();
    }

    pub fn video_ext_ok(p: &Path) -> bool {
        p.extension()
            .and_then(|e| e.to_str())
            .map(|e| util::VIDEO_EXT.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
    }
}
