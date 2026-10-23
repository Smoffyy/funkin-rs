use std::rc::Rc;

use super::anim::AnimController;
use super::atlas::{Frame, Frames};
use super::gfx::{DrawList, Effect, Quad, Vertex, GAME_HEIGHT, GAME_WIDTH};
use super::math::Point;
use super::tween::{Prop, Tween, TweenSet};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Axes {
    X,
    Y,
    XY,
}

fn compute_velocity(velocity: f64, acceleration: f64, drag: f64, max: f64, elapsed: f64) -> f64 {
    let mut v = velocity;
    if acceleration != 0.0 {
        v += acceleration * elapsed;
    } else if drag != 0.0 {
        let d = drag * elapsed;
        if v - d > 0.0 {
            v -= d;
        } else if v + d < 0.0 {
            v += d;
        } else {
            v = 0.0;
        }
    }
    if v != 0.0 && max != 0.0 {
        v = v.clamp(-max, max);
    }
    v
}

#[derive(Clone)]
pub struct Sprite {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub offset: Point,
    pub origin: Point,
    pub scale: Point,
    pub scroll: Point,
    pub velocity: Point,
    pub acceleration: Point,
    pub drag: Point,
    pub max_velocity: Point,
    pub angle: f64,
    pub angular_velocity: f64,
    alpha: f64,
    pub color: u32,
    pub color_offset: [f64; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub visible: bool,
    pub active: bool,
    pub exists: bool,
    pub moves: bool,
    pub antialiasing: bool,
    pub effect: Effect,
    pub id: i32,
    pub frames: Option<Rc<Frames>>,
    pub anim: AnimController,
    pub tweens: TweenSet,
    clip: Option<[f64; 4]>,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite::new(0.0, 0.0)
    }
}

impl Sprite {
    pub fn new(x: f64, y: f64) -> Sprite {
        Sprite {
            x,
            y,
            width: 0.0,
            height: 0.0,
            offset: Point::default(),
            origin: Point::default(),
            scale: Point::new(1.0, 1.0),
            scroll: Point::new(1.0, 1.0),
            velocity: Point::default(),
            acceleration: Point::default(),
            drag: Point::default(),
            max_velocity: Point::new(10000.0, 10000.0),
            angle: 0.0,
            angular_velocity: 0.0,
            alpha: 1.0,
            color: 0xFFFFFF,
            color_offset: [0.0; 4],
            flip_x: false,
            flip_y: false,
            visible: true,
            active: true,
            exists: true,
            moves: true,
            antialiasing: false,
            effect: Effect::None,
            id: 0,
            frames: None,
            anim: AnimController::default(),
            tweens: TweenSet::default(),
            clip: None,
        }
    }

    pub fn with_frames(x: f64, y: f64, frames: Rc<Frames>) -> Sprite {
        let mut s = Sprite::new(x, y);
        s.set_frames(frames);
        s
    }

    pub fn set_frames(&mut self, frames: Rc<Frames>) {
        self.anim.clear();
        self.frames = Some(frames);
        self.anim.frame_index = 0;
        self.width = self.frame_width();
        self.height = self.frame_height();
        self.center_origin();
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn set_alpha(&mut self, value: f64) {
        self.alpha = value.clamp(0.0, 1.0);
    }

    pub fn frame(&self) -> Option<&Frame> {
        let frames = self.frames.as_ref()?;
        frames.frames.get(self.anim.frame_index).or_else(|| frames.frames.first())
    }

    pub fn frame_width(&self) -> f64 {
        self.frame().map(|f| f.source_width.trunc()).unwrap_or(0.0)
    }

    pub fn frame_height(&self) -> f64 {
        self.frame().map(|f| f.source_height.trunc()).unwrap_or(0.0)
    }

    pub fn set_graphic_size(&mut self, width: i64, height: i64) {
        if width <= 0 && height <= 0 {
            return;
        }
        let sx = width as f64 / self.frame_width();
        let sy = height as f64 / self.frame_height();
        self.scale.set(sx, sy);
        if width <= 0 {
            self.scale.x = sy;
        } else if height <= 0 {
            self.scale.y = sx;
        }
    }

    pub fn update_hitbox(&mut self) {
        let fw = self.frame_width();
        let fh = self.frame_height();
        self.width = self.scale.x.abs() * fw;
        self.height = self.scale.y.abs() * fh;
        self.offset.set(-0.5 * (self.width - fw), -0.5 * (self.height - fh));
        self.center_origin();
    }

    pub fn center_origin(&mut self) {
        self.origin.set(self.frame_width() * 0.5, self.frame_height() * 0.5);
    }

    pub fn center_offsets(&mut self, adjust_position: bool) {
        self.offset.x = (self.frame_width() - self.width) * 0.5;
        self.offset.y = (self.frame_height() - self.height) * 0.5;
        if adjust_position {
            self.x += self.offset.x;
            self.y += self.offset.y;
        }
    }

    pub fn screen_center(&mut self, axes: Axes) {
        if axes != Axes::Y {
            self.x = (GAME_WIDTH - self.width) / 2.0;
        }
        if axes != Axes::X {
            self.y = (GAME_HEIGHT - self.height) / 2.0;
        }
    }

    pub fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    pub fn midpoint(&self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn graphic_midpoint(&self) -> Point {
        Point::new(self.x + self.frame_width() * 0.5, self.y + self.frame_height() * 0.5)
    }

    pub fn screen_position(&self, scroll_x: f64, scroll_y: f64) -> Point {
        Point::new(self.x - scroll_x * self.scroll.x, self.y - scroll_y * self.scroll.y)
    }

    pub fn set_clip_rect(&mut self, rect: Option<[f64; 4]>) {
        self.clip = rect.map(|r| [r[0].round(), r[1].round(), r[2].round(), r[3].round()]);
    }

    pub fn play(&mut self, name: &str, force: bool) {
        self.anim.play(name, force, false, 0);
    }

    pub fn add_by_prefix(&mut self, name: &str, prefix: &str, fps: f64, looped: bool) {
        if let Some(frames) = self.frames.clone() {
            self.anim.add_by_prefix(&frames, name, prefix, fps, looped, false);
        }
    }

    pub fn add_by_indices(&mut self, name: &str, prefix: &str, indices: &[i64], fps: f64, looped: bool) {
        if let Some(frames) = self.frames.clone() {
            self.anim.add_by_indices(&frames, name, prefix, indices, "", fps, looped, false);
        }
    }

    pub fn add_anim(&mut self, name: &str, indices: &[usize], fps: f64, looped: bool, flip_x: bool) {
        if let Some(frames) = self.frames.clone() {
            self.anim.add(&frames, name, indices, fps, looped, flip_x);
        }
    }

    pub fn tween(&mut self, prop: Prop, tween: Tween) {
        self.tweens.add(prop, tween, 0);
    }

    pub fn tween_tagged(&mut self, prop: Prop, tween: Tween, tag: u32) {
        self.tweens.add(prop, tween, tag);
    }

    fn prop(&self, prop: Prop) -> f64 {
        match prop {
            Prop::Y => self.y,
            Prop::Alpha => self.alpha,
        }
    }

    fn set_prop(&mut self, prop: Prop, value: f64) {
        match prop {
            Prop::Y => self.y = value,
            Prop::Alpha => self.set_alpha(value),
        }
    }

    pub fn update_tweens(&mut self, elapsed: f64) {
        if self.tweens.tweens.is_empty() {
            return;
        }
        let mut list = std::mem::take(&mut self.tweens.tweens);
        for (prop, tween, tag) in list.iter_mut() {
            let current = self.prop(*prop);
            if let Some(value) = tween.step(elapsed, current) {
                self.set_prop(*prop, value);
            }
            if tween.finished {
                self.tweens.completed.push(*tag);
            }
        }
        list.retain(|(_, t, _)| !t.finished);
        list.append(&mut self.tweens.tweens);
        self.tweens.tweens = list;
    }

    pub fn update(&mut self, elapsed: f64) {
        self.update_tweens(elapsed);
        if self.moves {
            self.update_motion(elapsed);
        }
        self.anim.update(elapsed);
    }

    pub fn update_motion(&mut self, elapsed: f64) {
        let delta = 0.5 * (compute_velocity(self.angular_velocity, 0.0, 0.0, 10000.0, elapsed) - self.angular_velocity);
        self.angular_velocity += delta;
        self.angle += self.angular_velocity * elapsed;
        self.angular_velocity += delta;

        let delta = 0.5 * (compute_velocity(self.velocity.x, self.acceleration.x, self.drag.x, self.max_velocity.x, elapsed) - self.velocity.x);
        self.velocity.x += delta;
        let dx = self.velocity.x * elapsed;
        self.velocity.x += delta;
        self.x += dx;

        let delta = 0.5 * (compute_velocity(self.velocity.y, self.acceleration.y, self.drag.y, self.max_velocity.y, elapsed) - self.velocity.y);
        self.velocity.y += delta;
        let dy = self.velocity.y * elapsed;
        self.velocity.y += delta;
        self.y += dy;
    }

    pub fn draw(&self, dl: &mut DrawList, camera: usize) {
        if !self.exists || !self.visible || self.alpha == 0.0 {
            return;
        }
        let Some(frames) = self.frames.as_ref() else { return };
        let Some(base) = self.frame() else { return };
        let Some(layer) = dl.layer(camera) else { return };
        let clipped;
        let frame = match self.clip {
            Some(clip) => {
                clipped = base.clip_to(clip);
                &clipped
            }
            None => base,
        };
        if frame.empty {
            return;
        }
        let (anim_flip_x, anim_flip_y) = self.anim.cur().map(|a| (a.flip_x, a.flip_y)).unwrap_or((false, false));
        let mut m = frame.matrix(self.flip_x != anim_flip_x, self.flip_y != anim_flip_y);
        m.translate(-self.origin.x, -self.origin.y);
        m.scale(self.scale.x, self.scale.y);
        if self.angle != 0.0 {
            let radians = self.angle.to_radians();
            m.rotate(radians.cos(), radians.sin());
        }
        let px = self.x - layer.scroll[0] * self.scroll.x - self.offset.x + self.origin.x;
        let py = self.y - layer.scroll[1] * self.scroll.y - self.offset.y + self.origin.y;
        m.translate(px, py);
        let full = m.then(&layer.transform);
        let mul = [
            ((self.color >> 16) & 0xFF) as f32 / 255.0,
            ((self.color >> 8) & 0xFF) as f32 / 255.0,
            (self.color & 0xFF) as f32 / 255.0,
            self.alpha as f32,
        ];
        let add = [
            (self.color_offset[0] / 255.0) as f32,
            (self.color_offset[1] / 255.0) as f32,
            (self.color_offset[2] / 255.0) as f32,
            (self.color_offset[3] / 255.0) as f32,
        ];
        let [u0, v0, u1, v1] = frame.uv;
        let corner = |lx: f64, ly: f64, u: f64, v: f64| {
            let (wx, wy) = full.apply(lx, ly);
            Vertex { pos: [wx as f32, wy as f32], uv: [u as f32, v as f32], mul, add }
        };
        let vertices = [
            corner(0.0, 0.0, u0, v0),
            corner(frame.width, 0.0, u1, v0),
            corner(0.0, frame.height, u0, v1),
            corner(frame.width, frame.height, u1, v1),
        ];
        dl.quad(camera, Quad { texture: frames.texture.id, linear: self.antialiasing, effect: self.effect, vertices });
    }
}