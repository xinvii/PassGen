//! One short-lived clock keeps the native window and its contents in sync.
use crate::AppWindow;
use slint::ComponentHandle;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
struct Track {
    from: f32,
    to: f32,
    started: Instant,
    duration: Duration,
}
impl Track {
    fn new(value: f32, ms: u64) -> Self {
        Self {
            from: value,
            to: value,
            started: Instant::now(),
            duration: Duration::from_millis(ms),
        }
    }
    fn value(self, now: Instant) -> f32 {
        let time = (now.saturating_duration_since(self.started).as_secs_f32()
            / self.duration.as_secs_f32())
        .clamp(0.0, 1.0);
        self.from + (self.to - self.from) * ease(time)
    }
    fn target(&mut self, value: f32, now: Instant) {
        if value != self.to {
            self.from = self.value(now);
            self.to = value;
            self.started = now;
        }
    }
    fn active(self, now: Instant) -> bool {
        self.from != self.to && now.saturating_duration_since(self.started) < self.duration
    }
}

// Invert cubic-bezier(0.2, 0, 0, 1)'s x coordinate, then evaluate y.
fn ease(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..20 {
        let t = (lo + hi) * 0.5;
        let curve_x = 0.6 * (1.0 - t) * (1.0 - t) * t + t * t * t;
        if curve_x < x {
            lo = t;
        } else {
            hi = t;
        }
    }
    let t = (lo + hi) * 0.5;
    3.0 * (1.0 - t) * t * t + t * t * t
}

struct Motion {
    mode: Track,
    about: Track,
    language: Track,
    result: Track,
    epoch: u64,
}
pub fn connect(ui: &AppWindow) {
    let mode = if ui.get_pin() { 1.0 } else { 0.0 };
    let language = if ui.get_chinese() { 1.0 } else { 0.0 };
    let result = ui.get_target_result_extra();
    ui.set_pin_progress(mode);
    ui.set_language_progress(language);
    ui.set_result_extra(result);
    let state = Rc::new(RefCell::new(Motion {
        mode: Track::new(mode, 280),
        about: Track::new(0.0, 300),
        language: Track::new(language, 220),
        result: Track::new(result, 280),
        epoch: 0,
    }));
    let weak = ui.as_weak();
    ui.on_animate_window_size(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut motion = state.borrow_mut();
        let now = Instant::now();
        motion
            .mode
            .target(if ui.get_pin() { 1.0 } else { 0.0 }, now);
        motion
            .about
            .target(if ui.get_about() { 1.0 } else { 0.0 }, now);
        motion
            .language
            .target(if ui.get_chinese() { 1.0 } else { 0.0 }, now);
        motion.result.target(ui.get_target_result_extra(), now);
        motion.epoch = motion.epoch.wrapping_add(1);
        let epoch = motion.epoch;
        drop(motion);
        frame(weak.clone(), state.clone(), epoch);
    });
}

fn frame(weak: slint::Weak<AppWindow>, state: Rc<RefCell<Motion>>, epoch: u64) {
    let Some(ui) = weak.upgrade() else {
        return;
    };
    let motion = state.borrow();
    if motion.epoch != epoch {
        return;
    }
    let now = Instant::now();
    let values = [
        motion.mode.value(now),
        motion.about.value(now),
        motion.language.value(now),
        motion.result.value(now),
    ];
    let active = motion.mode.active(now)
        || motion.about.active(now)
        || motion.language.active(now)
        || motion.result.active(now);
    drop(motion);
    ui.set_pin_progress(values[0]);
    ui.set_about_progress(values[1]);
    ui.set_language_progress(values[2]);
    ui.set_result_extra(values[3]);
    if !ui.window().is_maximized() {
        // Query the OS width: Slint's cached size may lag a user resize event.
        let width = crate::platform::window::logical_width(ui.window());
        ui.window().set_size(slint::LogicalSize::new(
            width,
            ui.get_page_height().max(150.0) + 2.0 * crate::chrome::PADDING,
        ));
    }
    if active {
        slint::Timer::single_shot(Duration::from_millis(16), move || frame(weak, state, epoch));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easing_is_monotonic_and_has_exact_endpoints() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        for step in 1..=100 {
            assert!(ease(step as f32 / 100.0) >= ease((step - 1) as f32 / 100.0));
        }
    }
    #[test]
    fn interrupted_motion_preserves_the_current_position() {
        let mut track = Track::new(0.0, 280);
        let start = Instant::now();
        track.target(1.0, start);
        let middle = start + Duration::from_millis(100);
        let before = track.value(middle);
        track.target(0.0, middle);
        assert!((track.value(middle) - before).abs() < f32::EPSILON);
        assert_eq!(track.value(middle + Duration::from_millis(280)), 0.0);
    }
}
