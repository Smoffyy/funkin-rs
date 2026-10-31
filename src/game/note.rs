use std::rc::Rc;

use crate::engine::app::Ctx;
use crate::engine::atlas::{Frame, Frames};
use crate::engine::math::round_decimal;
use crate::engine::sprite::Sprite;

use super::conductor::Conductor;
use super::data::Prefs;
use super::stage::PIXEL_ZOOM;

pub const SWAG_WIDTH: f64 = 160.0 * 0.7;
const LANE_LEFT: f64 = 50.0;
const COLORS: [&str; 4] = ["purple", "blue", "green", "red"];
const NOTE_CORES: [(f64, f64); 4] = [(76.5, 78.5), (78.0, 77.0), (78.0, 77.0), (76.5, 78.5)];

struct ArrowCores {
    stat: (f64, f64),
    press: [(f64, f64); 4],
    confirm: [(f64, f64); 4],
}

const ARROW_CORES: [ArrowCores; 4] = [
    ArrowCores {
        stat: (76.0, 78.0),
        press: [(73.0, 74.0), (73.0, 74.0), (72.5, 74.5), (72.5, 74.5)],
        confirm: [(112.0, 114.5), (112.0, 114.5), (112.5, 113.5), (112.5, 113.5)],
    },
    ArrowCores {
        stat: (78.0, 76.0),
        press: [(74.5, 72.5), (74.5, 72.5), (74.0, 73.0), (74.0, 73.0)],
        confirm: [(117.0, 115.0), (117.0, 115.0), (113.5, 118.0), (113.5, 118.0)],
    },
    ArrowCores {
        stat: (78.0, 76.0),
        press: [(77.0, 74.5), (77.0, 74.5), (76.0, 75.0), (76.0, 75.0)],
        confirm: [(116.0, 113.5), (116.5, 114.0), (116.0, 114.0), (116.0, 114.0)],
    },
    ArrowCores {
        stat: (76.5, 78.0),
        press: [(71.5, 77.5), (71.5, 77.5), (73.5, 75.5), (73.5, 75.5)],
        confirm: [(110.0, 116.0), (110.0, 116.0), (110.5, 115.5), (110.5, 115.5)],
    },
];

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
    pub shift: f64,
    pub head: usize,
}

fn trimmed_body(frame: &Frame) -> Frame {
    let mut out = frame.clone();
    let row = frame.height.min(frame.source_height).max(0.0);
    let dv = if row > 0.0 { (frame.uv[3] - frame.uv[1]) / row } else { 0.0 };
    out.uv[1] = frame.uv[1] + dv;
    out.uv[3] = frame.uv[3] - dv;
    out.height = frame.height - 2.0;
    out.source_height = frame.source_height - 2.0;
    out.offset_y = frame.offset_y + 1.0;
    out
}

fn body_frames(ctx: &mut Ctx) -> Rc<Frames> {
    if let Some(frames) = ctx.assets.registered("NOTE_assets_body") {
        return frames;
    }
    let base = ctx.atlas("NOTE_assets");
    let frames: Vec<Frame> = base
        .frames
        .iter()
        .map(|f| if f.name.contains("hold piece") { trimmed_body(f) } else { f.clone() })
        .collect();
    ctx.assets.register("NOTE_assets_body", Frames { texture: base.texture, frames })
}

fn note_frames(ctx: &mut Ctx, sustain: bool) -> Rc<Frames> {
    let base = if sustain { body_frames(ctx) } else { ctx.atlas("NOTE_assets") };
    if !ctx.data.prefs.pixel_perfect_arrows_active() {
        return base;
    }
    let key = if sustain { "NOTE_assets_body_pixel" } else { "NOTE_assets_pixel" };
    if let Some(frames) = ctx.assets.registered(key) {
        return frames;
    }
    let tw = base.texture.width as f64;
    let du = 0.5 / tw;
    let dv = 0.5 / base.texture.height as f64;
    let frames: Vec<Frame> = base
        .frames
        .iter()
        .map(|f| {
            let mut out = f.clone();
            out.uv = [f.uv[0] + du, f.uv[1] + dv, f.uv[2] - du, f.uv[3] - dv];
            if f.name.starts_with("arrowDOWN") || f.name.starts_with("down ") {
                out.width -= 1.0;
                out.uv[2] -= 1.0 / tw;
            }
            out
        })
        .collect();
    ctx.assets.register(key, Frames { texture: base.texture, frames })
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
        let head = match prev {
            Some(p) if sustain => {
                if notes[p].is_sustain {
                    notes[p].head
                } else {
                    p
                }
            }
            _ => index,
        };
        let mut sprite = if pixel {
            let frames = if sustain { ctx.grid("weeb/pixelUI/arrowEnds", 7, 6) } else { ctx.grid("weeb/pixelUI/arrows-pixels", 17, 17) };
            let mut s = Sprite::with_frames(0.0, 0.0, frames);
            add_pixel_anims(&mut s, sustain);
            let w = s.width;
            s.set_graphic_size((w * PIXEL_ZOOM) as i64, 0);
            s.update_hitbox();
            s
        } else {
            let frames = note_frames(ctx, sustain);
            let mut s = Sprite::with_frames(0.0, 0.0, frames.clone());
            add_note_anims(&mut s, &frames);
            let w = s.width;
            s.set_graphic_size((w * 0.7) as i64, 0);
            s.update_hitbox();
            s.antialiasing = true;
            s
        };
        sprite.x += LANE_LEFT + center_shift(&ctx.data.prefs);
        sprite.y -= 2000.0;
        sprite.x += SWAG_WIDTH * data as f64;
        sprite.play(&format!("{}Scroll", COLORS[data]), false);
        let fixed = !pixel && ctx.data.prefs.pixel_perfect_arrows_active();
        if fixed && !sustain {
            anchor_core(&mut sprite, NOTE_CORES[data]);
        }
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
            shift: 0.0,
            head,
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
            note.shift = if notes[p].is_sustain { notes[p].shift } else { notes[p].sprite.height / 2.0 };
            if notes[p].is_sustain {
                let span = strum_time - notes[p].strum_time;
                let pn = &mut notes[p];
                pn.sprite.play(&format!("{}hold", COLORS[pn.data]), false);
                pn.sprite.scale.y = span * 0.45 * round_decimal(speed, 2) / pn.sprite.frame_height();
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

fn center_shift(prefs: &Prefs) -> f64 {
    if prefs.center_arrows_active() {
        (crate::engine::gfx::GAME_WIDTH - (crate::engine::gfx::GAME_WIDTH / 2.0 + 4.0 * SWAG_WIDTH)) / 2.0 - LANE_LEFT
    } else {
        0.0
    }
}

pub fn strum_core(index: usize, anim: &str, frame: usize) -> (f64, f64) {
    let cores = &ARROW_CORES[index];
    match anim {
        "pressed" => cores.press[frame],
        "confirm" => cores.confirm[frame],
        _ => cores.stat,
    }
}

pub fn anchor_core(sprite: &mut Sprite, core: (f64, f64)) {
    sprite.offset.x = sprite.origin.x + sprite.scale.x * (core.0 - sprite.origin.x) - sprite.width / 2.0;
    sprite.offset.y = sprite.origin.y + sprite.scale.y * (core.1 - sprite.origin.y) - sprite.height / 2.0;
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
        let frames = note_frames(ctx, false);
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
    s.x += LANE_LEFT + center_shift(&ctx.data.prefs);
    s.x += (crate::engine::gfx::GAME_WIDTH / 2.0) * player as f64;
    s
}