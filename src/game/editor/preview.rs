use crate::engine::app::Ctx;
use crate::engine::camera::Camera;
use crate::engine::gfx::DrawList;

use super::super::character::Character;
use super::ui::Rect;

const DIRECTIONS: [&str; 4] = ["LEFT", "DOWN", "UP", "RIGHT"];

pub struct Preview {
    character: Character,
    name: String,
    is_player: bool,
    camera: usize,
    pub area: Rect,
}

impl Preview {
    pub fn new(ctx: &mut Ctx, name: &str, is_player: bool, camera: usize, area: Rect) -> Preview {
        let mut cam = Camera::transparent();
        cam.x = area.x;
        cam.y = area.y;
        cam.width = area.w;
        cam.height = area.h;
        while ctx.cameras.len() <= camera {
            ctx.cameras.push(Camera::transparent());
        }
        ctx.cameras[camera] = cam;
        let mut preview = Preview { character: Character::new(ctx, 0.0, 0.0, name, is_player), name: name.to_string(), is_player, camera, area };
        preview.frame(ctx);
        preview
    }

    fn frame(&mut self, ctx: &mut Ctx) {
        let sprite = &self.character.sprite;
        let zoom = (self.area.w / sprite.width.max(1.0)).min(self.area.h / sprite.height.max(1.0)) * 0.9;
        let mid = sprite.midpoint();
        let cam = ctx.camera(self.camera);
        cam.zoom = zoom;
        cam.focus_on(mid.x, mid.y);
    }

    pub fn sync(&mut self, ctx: &mut Ctx, name: &str) {
        if self.name != name {
            self.character = Character::new(ctx, 0.0, 0.0, name, self.is_player);
            self.name = name.to_string();
            self.frame(ctx);
        }
    }

    pub fn sing(&mut self, lane: usize, alt: bool) {
        let base = format!("sing{}", DIRECTIONS[lane]);
        let alt_name = format!("{base}-alt");
        let name = if alt && self.character.has_anim(&alt_name) { alt_name } else { base };
        self.character.play_anim(&name, true, false, 0);
        self.character.hold_timer = 0.0;
    }

    pub fn play(&mut self, animation: &str) {
        if self.character.has_anim(animation) {
            self.character.play_anim(animation, true, false, 0);
            self.character.hold_timer = 0.0;
        }
    }

    pub fn beat(&mut self, beat: i64) {
        let singing = self.character.anim_name().starts_with("sing");
        if singing {
            return;
        }
        if beat % 2 == 0 {
            if self.is_player {
                self.character.play_anim("idle", false, false, 0);
            } else {
                self.character.dance();
            }
        } else if self.name == "spooky" {
            self.character.dance();
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx) {
        self.character.update(ctx);
        let limit = ctx.data.conductor.step_crochet * 4.0 * 0.001;
        let name = self.character.anim_name();
        if self.is_player && name.starts_with("sing") && !name.ends_with("miss") && self.character.hold_timer > limit {
            self.character.play_anim("idle", false, false, 0);
        }
    }

    pub fn draw(&self, dl: &mut DrawList) {
        self.character.draw(dl, self.camera);
    }
}