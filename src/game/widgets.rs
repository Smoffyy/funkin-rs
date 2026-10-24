use crate::engine::app::Ctx;
use crate::engine::gfx::DrawList;
use crate::engine::sprite::Sprite;

use super::character::icon_frames;

pub struct HealthIcon {
    pub sprite: Sprite,
    icon: String,
    is_player: bool,
    is_old: bool,
    pub size: f64,
}

impl HealthIcon {
    pub fn new(ctx: &mut Ctx, character: &str, is_player: bool) -> HealthIcon {
        let mut icon = HealthIcon { sprite: Sprite::with_frames(0.0, 0.0, ctx.grid("iconGrid", 150, 150)), icon: String::new(), is_player, is_old: false, size: 150.0 };
        icon.change_icon(ctx, character);
        icon.sprite.antialiasing = true;
        icon.sprite.scroll.set(0.0, 0.0);
        icon
    }

    pub fn change_icon(&mut self, ctx: &mut Ctx, character: &str) {
        if character == self.icon {
            return;
        }
        if !self.sprite.anim.exists(character) {
            let frames = icon_frames(ctx, character);
            self.sprite.add_anim(character, &frames, 0.0, false, self.is_player);
        }
        self.sprite.play(character, false);
        self.icon = character.to_string();
    }

    pub fn swap_old_icon(&mut self, ctx: &mut Ctx, player: &str) {
        self.is_old = !self.is_old;
        if self.is_old {
            self.change_icon(ctx, "bf-old");
        } else {
            self.change_icon(ctx, player);
        }
    }

    pub fn set_size(&mut self, size: f64) {
        self.size = size;
        let scale = size / self.sprite.frame_width();
        self.sprite.scale.set(scale, scale);
        self.sprite.update_hitbox();
    }

    pub fn set_losing(&mut self, losing: bool) {
        self.sprite.anim.set_cur_frame(if losing { 1 } else { 0 });
    }
}

pub struct HealthBar {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    value: f64,
    pub min: f64,
    pub max: f64,
    pub empty_color: u32,
    pub fill_color: u32,
}

impl HealthBar {
    pub fn new(x: f64, y: f64, width: f64, height: f64, min: f64, max: f64) -> HealthBar {
        HealthBar { x, y, width, height, value: min, min, max, empty_color: 0xFFFF0000, fill_color: 0xFF66FF33 }
    }

    pub fn update(&mut self, value: f64) {
        self.value = value.clamp(self.min, self.max);
    }

    pub fn percent(&self) -> f64 {
        if self.value > self.max {
            return 100.0;
        }
        (((self.value - self.min) / (self.max - self.min)) * 100.0).floor()
    }

    pub fn draw(&self, ctx: &mut Ctx, dl: &mut DrawList, camera: usize) {
        let mut empty = Sprite::with_frames(self.x, self.y, ctx.solid(self.width, self.height, self.empty_color));
        empty.scroll.set(0.0, 0.0);
        empty.draw(dl, camera);
        let fraction = (self.value - self.min) / (self.max - self.min);
        let interval = self.width / 100.0;
        let fill = ((fraction * self.width / interval).trunc() * interval).round().trunc();
        if fill > 0.0 {
            let mut filled = Sprite::with_frames(self.x + self.width - fill, self.y, ctx.solid(fill, self.height, self.fill_color));
            filled.scroll.set(0.0, 0.0);
            filled.draw(dl, camera);
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Flicker {
    duration: f64,
    interval: f64,
    time: f64,
    tick: f64,
    end_visibility: bool,
    pub active: bool,
}

impl Flicker {
    pub fn start(duration: f64, interval: f64, end_visibility: bool) -> Flicker {
        Flicker { duration, interval, time: 0.0, tick: 0.0, end_visibility, active: true }
    }

    pub fn update(&mut self, elapsed: f64, visible: &mut bool) -> bool {
        if !self.active {
            return false;
        }
        self.time += elapsed;
        self.tick += elapsed;
        while self.tick >= self.interval {
            self.tick -= self.interval;
            *visible = !*visible;
        }
        if self.time >= self.duration {
            self.active = false;
            *visible = self.end_visibility;
            return true;
        }
        false
    }
}