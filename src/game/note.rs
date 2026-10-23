use std::rc::Rc;

use crate::engine::app::Ctx;
use crate::engine::atlas::Frames;
use crate::engine::sprite::Sprite;

use super::conductor::Conductor;
use super::stage::PIXEL_ZOOM;

pub const SWAG_WIDTH: f64 = 160.0 * 0.7;
const COLORS: [&str; 4] = ["purple", "blue", "green", "red"];

pub struct Note {
    pub sprite: Sprite,
    pub strum_time: f64,
    pub must_press: bool,
    pub data: usize,
    pub can_be_hit: bool,
    pub too_late: bool,
    pub was_good_hit: bool,
    pub prev: usize,
    will_miss: bool,
    pub alt: bool,
    pub sustain_length: f64,
    pub is_sustain: bool,
}

fn add_note_anims(sprite: &mut Sprite, frames: &Rc<Frames>) {
    let a = &mut sprite.anim;
    a.add_by_prefix(frames, "greenScroll", "green0", 30.0, true, false);
    a.add_by_prefix(frames, "redScroll", "red0", 30.0, true, false);
    a.add_by_prefix(frames, "blueScroll", "blue0", 30.0, true, false);
    a.add_by_prefix(frames, "purpleScroll", "purple0", 30.0, true, false);
    a.add_by_prefix(frames, "purpleholdend", "pruple end hold", 30.0, true, false);
    a.add_by_prefix(frames, "greenholdend", "green hold end", 30.0, true, false);
    a.add_by_prefix(frames, "redholdend", "red hold end", 30.0, true, false);
    a.add_by_prefix(frames, "blueholdend", "blue hold end", 30.0, true, false);
    a.add_by_prefix(frames, "purplehold", "purple hold piece", 30.0, true, false);
    a.add_by_prefix(frames, "greenhold", "green hold piece", 30.0, true, false);
    a.add_by_prefix(frames, "redhold", "red hold piece", 30.0, true, false);
    a.add_by_prefix(frames, "bluehold", "blue hold piece", 30.0, true, false);
}

fn add_pixel_anims(sprite: &mut Sprite, sustain: bool) {
    if sustain {
        for (name, index) in [("purpleholdend", 4), ("greenholdend", 6), ("redholdend", 7), ("blueholdend", 5), ("purplehold", 0), ("greenhold", 2), ("redhold", 3), ("bluehold", 1)] {
            sprite.add_anim(name, &[index], 30.0, true, false);
        }
    } else {
        for (name, index) in [("greenScroll", 6), ("redScroll", 7), ("blueScroll", 5), ("purpleScroll", 4)] {
            sprite.add_anim(name, &[index], 30.0, true, false);
        }
    }
}

impl Note {
    pub fn create(ctx: &mut Ctx, notes: &mut Vec<Note>, strum_time: f64, data: usize, prev: Option<usize>, sustain: bool, speed: f64, pixel: bool) -> usize {
        let index = notes.len();
        let mut sprite = if pixel {
            let frames = if sustain { ctx.grid("weeb/pixelUI/arrowEnds", 7, 6) } else { ctx.grid("weeb/pixelUI/arrows-pixels", 17, 17) };
            let mut s = Sprite::with_frames(0.0, 0.0, frames);
            add_pixel_anims(&mut s, sustain);
            let w = s.width;
            s.set_graphic_size((w * PIXEL_ZOOM) as i64, 0);
            s.update_hitbox();
            s
        } else {
            let frames = ctx.atlas("NOTE_assets");
            let mut s = Sprite::with_frames(0.0, 0.0, frames.clone());
            add_note_anims(&mut s, &frames);
            let w = s.width;
            s.set_graphic_size((w * 0.7) as i64, 0);
            s.update_hitbox();
            s.antialiasing = true;
            s
        };
        sprite.x += 50.0;
        sprite.y -= 2000.0;
        sprite.x += SWAG_WIDTH * data as f64;
        sprite.play(&format!("{}Scroll", COLORS[data]), false);
        let mut note = Note {
            sprite,
            strum_time,
            must_press: false,
            data,
            can_be_hit: false,
            too_late: false,
            was_good_hit: false,
            prev: prev.unwrap_or(index),
            will_miss: false,
            alt: false,
            sustain_length: 0.0,
            is_sustain: sustain,
        };
        if sustain && prev.is_some() {
            note.sprite.set_alpha(0.6);
            if ctx.data.prefs.downscroll {
                note.sprite.angle = 180.0;
            }
            note.sprite.x += note.sprite.width / 2.0;
            note.sprite.play(&format!("{}holdend", COLORS[data]), false);
            note.sprite.update_hitbox();
            note.sprite.x -= note.sprite.width / 2.0;
            if pixel {
                note.sprite.x += 30.0;
            }
            let p = prev.unwrap();
            if notes[p].is_sustain {
                let step = ctx.data.conductor.step_crochet;
                let pn = &mut notes[p];
                pn.sprite.play(&format!("{}hold", COLORS[pn.data]), false);
                pn.sprite.scale.y *= step / 100.0 * 1.5 * speed;
                pn.sprite.update_hitbox();
            }
        }
        notes.push(note);
        index
    }

    pub fn update(&mut self, elapsed: f64, conductor: &Conductor) {
        self.sprite.update(elapsed);
        if self.must_press {
            if self.will_miss && !self.was_good_hit {
                self.too_late = true;
                self.can_be_hit = false;
            } else if self.strum_time > conductor.song_position - conductor.safe_zone_offset {
                if self.strum_time < conductor.song_position + conductor.safe_zone_offset * 0.5 {
                    self.can_be_hit = true;
                }
            } else {
                self.can_be_hit = true;
                self.will_miss = true;
            }
        } else {
            self.can_be_hit = false;
            if self.strum_time <= conductor.song_position {
                self.was_good_hit = true;
            }
        }
        if self.too_late && self.sprite.alpha() > 0.3 {
            self.sprite.set_alpha(0.3);
        }
    }

    pub fn is_end(&self) -> bool {
        self.sprite.anim.name().ends_with("end")
    }
}

pub fn strum(ctx: &mut Ctx, index: usize, player: usize, y: f64, pixel: bool) -> Sprite {
    let mut s;
    if pixel {
        s = Sprite::with_frames(0.0, y, ctx.grid("weeb/pixelUI/arrows-pixels", 17, 17));
        for (name, frame) in [("green", 6), ("red", 7), ("blue", 5), ("purplel", 4)] {
            s.add_anim(name, &[frame], 30.0, true, false);
        }
        let w = s.width;
        s.set_graphic_size((w * PIXEL_ZOOM) as i64, 0);
        s.update_hitbox();
        s.x += SWAG_WIDTH * index as f64;
        let confirm_fps = if index == 2 { 12.0 } else { 24.0 };
        s.add_anim("static", &[index], 30.0, true, false);
        s.add_anim("pressed", &[index + 4, index + 8], 12.0, false, false);
        s.add_anim("confirm", &[index + 12, index + 16], confirm_fps, false, false);
    } else {
        let frames = ctx.atlas("NOTE_assets");
        s = Sprite::with_frames(0.0, y, frames.clone());
        let dirs = ["left", "down", "up", "right"];
        let statics = ["arrowLEFT", "arrowDOWN", "arrowUP", "arrowRIGHT"];
        s.anim.add_by_prefix(&frames, "green", "arrowUP", 30.0, true, false);
        s.anim.add_by_prefix(&frames, "blue", "arrowDOWN", 30.0, true, false);
        s.anim.add_by_prefix(&frames, "purple", "arrowLEFT", 30.0, true, false);
        s.anim.add_by_prefix(&frames, "red", "arrowRIGHT", 30.0, true, false);
        s.antialiasing = true;
        let w = s.width;
        s.set_graphic_size((w * 0.7) as i64, 0);
        s.x += SWAG_WIDTH * index as f64;
        s.anim.add_by_prefix(&frames, "static", statics[index], 30.0, true, false);
        s.anim.add_by_prefix(&frames, "pressed", &format!("{} press", dirs[index]), 24.0, false, false);
        s.anim.add_by_prefix(&frames, "confirm", &format!("{} confirm", dirs[index]), 24.0, false, false);
    }
    s.update_hitbox();
    s.scroll.set(0.0, 0.0);
    s.id = index as i32;
    s.play("static", false);
    s.x += 50.0;
    s.x += (crate::engine::gfx::GAME_WIDTH / 2.0) * player as f64;
    s
}