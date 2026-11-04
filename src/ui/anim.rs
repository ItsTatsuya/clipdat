//! Tiny easing tweens for panel transitions (clip dock, export sheet, controller).
//!
//! A `Tween` follows a bool: flip it and the value glides from wherever it is now
//! toward 0 or 1, so reversing mid-animation never jumps.

use std::time::{Duration, Instant};

pub struct Tween {
    on: bool,
    from: f32,
    start: Option<Instant>,
    dur: Duration,
}

impl Tween {
    pub fn new(ms: u64) -> Self {
        Self { on: false, from: 0.0, start: None, dur: Duration::from_millis(ms) }
    }

    fn progress(&self) -> f32 {
        match self.start {
            Some(s) => (s.elapsed().as_secs_f32() / self.dur.as_secs_f32()).min(1.0),
            None => 1.0,
        }
    }

    /// Current eased value in 0..=1.
    pub fn value(&self) -> f32 {
        let to = if self.on { 1.0 } else { 0.0 };
        self.from + (to - self.from) * ease_out_cubic(self.progress())
    }

    /// Retarget (no-op if unchanged) and return the current value.
    pub fn follow(&mut self, on: bool) -> f32 {
        if on != self.on {
            self.from = self.value();
            self.on = on;
            self.start = Some(Instant::now());
        }
        let v = self.value();
        if self.progress() >= 1.0 {
            self.start = None;
            self.from = v;
        }
        v
    }

    pub fn animating(&self) -> bool {
        self.start.is_some()
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
