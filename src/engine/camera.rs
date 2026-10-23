use super::gfx::{Affine, DrawList, GAME_HEIGHT, GAME_WIDTH};
use super::math::{color_rgba, Rng};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FollowStyle {
    LockOn,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CameraEvent {
    FlashDone,
    FadeDone,
    ShakeDone,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Target {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub bg_color: u32,
    pub visible: bool,
    pub alpha: f64,
    pub target: Option<Target>,
    pub follow_lerp: f64,
    pub deadzone: Option<[f64; 4]>,
    pub min_scroll_x: Option<f64>,
    pub max_scroll_x: Option<f64>,
    pub min_scroll_y: Option<f64>,
    pub max_scroll_y: Option<f64>,
    scroll_target: (f64, f64),
    flash_color: u32,
    flash_duration: f64,
    flash_alpha: f64,
    fade_color: u32,
    fade_duration: f64,
    fade_alpha: f64,
    fade_in: bool,
    fade_completed: bool,
    shake_intensity: f64,
    shake_duration: f64,
    shake_offset: (f64, f64),
    events: Vec<CameraEvent>,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new()
    }
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            x: 0.0,
            y: 0.0,
            width: GAME_WIDTH,
            height: GAME_HEIGHT,
            zoom: 1.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
            bg_color: 0xFF000000,
            visible: true,
            alpha: 1.0,
            target: None,
            follow_lerp: 1.0,
            deadzone: None,
            min_scroll_x: None,
            max_scroll_x: None,
            min_scroll_y: None,
            max_scroll_y: None,
            scroll_target: (0.0, 0.0),
            flash_color: 0xFFFFFFFF,
            flash_duration: 0.0,
            flash_alpha: 0.0,
            fade_color: 0xFF000000,
            fade_duration: 0.0,
            fade_alpha: 0.0,
            fade_in: false,
            fade_completed: true,
            shake_intensity: 0.0,
            shake_duration: 0.0,
            shake_offset: (0.0, 0.0),
            events: Vec::new(),
        }
    }

    pub fn transparent() -> Camera {
        let mut cam = Camera::new();
        cam.bg_color = 0;
        cam
    }

    pub fn follow(&mut self, target: Target, style: FollowStyle, lerp: f64) {
        self.target = Some(target);
        self.follow_lerp = lerp;
        self.deadzone = match style {
            FollowStyle::LockOn => Some([
                (self.width - target.width) / 2.0,
                (self.height - target.height) / 2.0 - target.height * 0.25,
                target.width,
                target.height,
            ]),
        };
    }

    pub fn set_target_position(&mut self, x: f64, y: f64) {
        if let Some(t) = self.target.as_mut() {
            t.x = x;
            t.y = y;
        }
    }

    pub fn focus_on(&mut self, x: f64, y: f64) {
        self.scroll_x = x - self.width * 0.5;
        self.scroll_y = y - self.height * 0.5;
    }

    pub fn update(&mut self, elapsed: f64, rng: &mut Rng) {
        if let Some(t) = self.target {
            self.update_follow(t, elapsed);
        }
        self.update_scroll();
        if self.flash_alpha > 0.0 {
            self.flash_alpha -= elapsed / self.flash_duration;
            if self.flash_alpha <= 0.0 {
                self.events.push(CameraEvent::FlashDone);
            }
        }
        if !self.fade_completed {
            if self.fade_in {
                self.fade_alpha -= elapsed / self.fade_duration;
                if self.fade_alpha <= 0.0 {
                    self.fade_alpha = 0.0;
                    self.fade_completed = true;
                    self.events.push(CameraEvent::FadeDone);
                }
            } else {
                self.fade_alpha += elapsed / self.fade_duration;
                if self.fade_alpha >= 1.0 {
                    self.fade_alpha = 1.0;
                    self.fade_completed = true;
                    self.events.push(CameraEvent::FadeDone);
                }
            }
        }
        self.shake_offset = (0.0, 0.0);
        if self.shake_duration > 0.0 {
            self.shake_duration -= elapsed;
            if self.shake_duration <= 0.0 {
                self.events.push(CameraEvent::ShakeDone);
            } else {
                let ix = self.shake_intensity * self.width;
                let iy = self.shake_intensity * self.height;
                self.shake_offset = (rng.float(-ix, ix) * self.zoom, rng.float(-iy, iy) * self.zoom);
            }
        }
    }

    fn update_follow(&mut self, t: Target, elapsed: f64) {
        let Some(dz) = self.deadzone else {
            self.focus_on(t.x + t.width * 0.5, t.y + t.height * 0.5);
            return;
        };
        let st = &mut self.scroll_target;
        let edge = t.x - dz[0];
        if st.0 > edge {
            st.0 = edge;
        }
        let edge = t.x + t.width - dz[0] - dz[2];
        if st.0 < edge {
            st.0 = edge;
        }
        let edge = t.y - dz[1];
        if st.1 > edge {
            st.1 = edge;
        }
        let edge = t.y + t.height - dz[1] - dz[3];
        if st.1 < edge {
            st.1 = edge;
        }
        if self.follow_lerp >= 1.0 {
            self.scroll_x = st.0;
            self.scroll_y = st.1;
        } else {
            let lerp = self.follow_lerp * (elapsed / (1.0 / 60.0));
            self.scroll_x += (st.0 - self.scroll_x) * lerp;
            self.scroll_y += (st.1 - self.scroll_y) * lerp;
        }
    }

    fn update_scroll(&mut self) {
        let zoom = self.zoom;
        let min_x = self.min_scroll_x.map(|v| v - (zoom - 1.0) * self.width / (2.0 * zoom));
        let max_x = self.max_scroll_x.map(|v| v + (zoom - 1.0) * self.width / (2.0 * zoom) - self.width);
        let min_y = self.min_scroll_y.map(|v| v - (zoom - 1.0) * self.height / (2.0 * zoom));
        let max_y = self.max_scroll_y.map(|v| v + (zoom - 1.0) * self.height / (2.0 * zoom) - self.height);
        if let Some(v) = min_x {
            self.scroll_x = self.scroll_x.max(v);
        }
        if let Some(v) = max_x {
            self.scroll_x = self.scroll_x.min(v);
        }
        if let Some(v) = min_y {
            self.scroll_y = self.scroll_y.max(v);
        }
        if let Some(v) = max_y {
            self.scroll_y = self.scroll_y.min(v);
        }
    }

    pub fn flash(&mut self, color: u32, duration: f64, force: bool) {
        if !force && self.flash_alpha > 0.0 {
            return;
        }
        self.flash_color = color;
        self.flash_duration = if duration <= 0.0 { 0.000001 } else { duration };
        self.flash_alpha = 1.0;
    }

    pub fn fade(&mut self, color: u32, duration: f64, fade_in: bool, force: bool) {
        if !self.fade_completed && !force {
            return;
        }
        self.fade_color = color;
        self.fade_duration = if duration <= 0.0 { 0.000001 } else { duration };
        self.fade_in = fade_in;
        self.fade_alpha = if fade_in { 0.999999 } else { 0.000001 };
        self.fade_completed = false;
    }

    pub fn take_events(&mut self) -> Vec<CameraEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn push_layer(&self, slot: usize, dl: &mut DrawList) {
        if !self.visible || self.alpha <= 0.0 {
            return;
        }
        let s = dl.view.scale;
        let (sx, sy) = self.shake_offset;
        let hw = self.width * 0.5;
        let hh = self.height * 0.5;
        let transform = Affine {
            a: self.zoom * s,
            b: 0.0,
            c: 0.0,
            d: self.zoom * s,
            tx: dl.view.offset_x + s * (self.x + sx + hw - hw * self.zoom),
            ty: dl.view.offset_y + s * (self.y + sy + hh - hh * self.zoom),
        };
        let rect = [self.x + sx, self.y + sy, self.width, self.height];
        let bg = color_rgba(self.bg_color);
        let alpha = self.alpha as f32;
        dl.push_layer(slot, transform, [self.scroll_x, self.scroll_y], rect, Some(bg), alpha);
        let fx = |color: u32, amount: f64| {
            let c = color_rgba(color);
            let base = if c[3] <= 0.0 { 1.0 } else { c[3] };
            [c[0], c[1], c[2], base * amount as f32]
        };
        if self.flash_alpha > 0.0 {
            dl.overlay(slot, fx(self.flash_color, self.flash_alpha));
        }
        if self.fade_alpha > 0.0 {
            dl.overlay(slot, fx(self.fade_color, self.fade_alpha));
        }
    }
}