use crate::engine::app::Ctx;
use crate::engine::audio::Sound;
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::sprite::{Axes, Sprite};
use crate::engine::text::Text;
use crate::engine::timer::Timer;

use super::library;
use super::stage::PIXEL_ZOOM;

const TYPE_DELAY: f64 = 0.04;

struct TypeText {
    text: Text,
    full: Vec<char>,
    length: usize,
    timer: f64,
    typing: bool,
}

impl TypeText {
    fn reset(&mut self, line: &str) {
        self.full = line.chars().collect();
        self.length = 0;
        self.timer = 0.0;
        self.typing = true;
        self.text.set_text("");
    }

    fn update(&mut self, ctx: &mut Ctx, sound: &mut Sound) -> bool {
        if !self.typing {
            return false;
        }
        if self.length < self.full.len() {
            self.timer += ctx.elapsed;
        }
        if self.timer >= TYPE_DELAY {
            self.length = (self.length + (self.timer / TYPE_DELAY) as usize).min(self.full.len());
            self.timer %= TYPE_DELAY;
            sound.play(&mut ctx.audio, true, 0.0);
        }
        let shown: String = self.full[..self.length].iter().collect();
        if shown != self.text.text() {
            self.text.set_text(&shown);
            if self.length >= self.full.len() {
                self.typing = false;
                return true;
            }
        }
        false
    }
}

pub struct DialogueBox {
    song: String,
    lines: Vec<String>,
    bg_fade: Sprite,
    fade_timer: Timer,
    portrait_left: Sprite,
    portrait_right: Sprite,
    face: Option<Sprite>,
    box_sprite: Sprite,
    hand: Sprite,
    drop_text: Text,
    swag: TypeText,
    type_sound: Sound,
    cur_character: String,
    opened: bool,
    started: bool,
    ended: bool,
    is_ending: bool,
    ending_timer: Timer,
    finish_timer: Timer,
    pub has_dialog: bool,
    pub finished: bool,
    pub alive: bool,
}

fn pixel_sized(mut s: Sprite, factor: f64) -> Sprite {
    let w = s.width;
    s.set_graphic_size((w * factor) as i64, 0);
    s.update_hitbox();
    s.scroll.set(0.0, 0.0);
    s
}

impl DialogueBox {
    pub fn new(ctx: &mut Ctx, song: &str, lines: Vec<String>) -> DialogueBox {
        let song = song.to_lowercase();
        if song == "senpai" || song == "thorns" {
            let name = if song == "senpai" { "Lunchbox" } else { "LunchboxScary" };
            library::play_music(ctx, name, 0.0, true);
            ctx.audio.fade_in_music(1.0, 0.0, 0.8);
        }
        let mut bg_fade = Sprite::with_frames(-200.0, -200.0, ctx.solid(GAME_WIDTH * 1.3, GAME_HEIGHT * 1.3, 0xFFB3DFD8));
        bg_fade.scroll.set(0.0, 0.0);
        bg_fade.set_alpha(0.0);

        let mut left = Sprite::with_frames(-20.0, 40.0, ctx.atlas("weeb/senpaiPortrait"));
        left.add_by_prefix("enter", "Senpai Portrait Enter", 24.0, false);
        let mut left = pixel_sized(left, PIXEL_ZOOM * 0.9);
        left.visible = false;

        let mut right = Sprite::with_frames(0.0, 40.0, ctx.atlas("weeb/bfPortrait"));
        right.add_by_prefix("enter", "Boyfriend portrait enter", 24.0, false);
        let mut right = pixel_sized(right, PIXEL_ZOOM * 0.9);
        right.visible = false;

        let mut face = None;
        let (key, prefix, normal_frame) = match song.as_str() {
            "senpai" => ("weeb/pixelUI/dialogueBox-pixel", "Text Box Appear", 4),
            "roses" => {
                library::play_sound(ctx, "ANGRY_TEXT_BOX", 1.0);
                ("weeb/pixelUI/dialogueBox-senpaiMad", "SENPAI ANGRY IMPACT SPEECH", 4)
            }
            "thorns" => {
                let mut f = Sprite::with_frames(320.0, 170.0, ctx.image("weeb/spiritFaceForward"));
                let w = f.width;
                f.set_graphic_size((w * 6.0) as i64, 0);
                f.scroll.set(0.0, 0.0);
                face = Some(f);
                ("weeb/pixelUI/dialogueBox-evil", "Spirit Textbox spawn", 11)
            }
            _ => ("", "", 0),
        };
        let has_dialog = !key.is_empty();

        let mut box_sprite = Sprite::new(-20.0, 45.0);
        let mut hand = Sprite::new(1042.0, 590.0);
        if has_dialog {
            box_sprite.set_frames(ctx.atlas(key));
            box_sprite.add_by_prefix("normalOpen", prefix, 24.0, false);
            box_sprite.add_by_indices("normal", prefix, &[normal_frame], 24.0, true);
            box_sprite.play("normalOpen", false);
            box_sprite = pixel_sized(box_sprite, PIXEL_ZOOM * 0.9);
            box_sprite.screen_center(Axes::X);
            left.screen_center(Axes::X);
            hand = pixel_sized(Sprite::with_frames(1042.0, 590.0, ctx.image("weeb/pixelUI/hand_textbox")), PIXEL_ZOOM * 0.9);
        }
        hand.visible = false;

        let font = ctx.data.fonts.ui.clone();
        let mut drop_text = Text::new(242.0, 502.0, (GAME_WIDTH * 0.6).trunc(), "", 40.0, font.clone());
        drop_text.set_color(0xFFD89494);
        drop_text.sprite.scroll.set(0.0, 0.0);
        let mut swag_text = Text::new(240.0, 500.0, (GAME_WIDTH * 0.6).trunc(), "", 40.0, font);
        swag_text.set_color(0xFF3F2021);
        swag_text.sprite.scroll.set(0.0, 0.0);
        let data = library::sound(ctx, "pixelText");

        let mut fade_timer = Timer::new();
        fade_timer.start(0.83, 5);
        DialogueBox {
            song,
            lines,
            bg_fade,
            fade_timer,
            portrait_left: left,
            portrait_right: right,
            face,
            box_sprite,
            hand,
            drop_text,
            swag: TypeText { text: swag_text, full: Vec::new(), length: 0, timer: 0.0, typing: false },
            type_sound: Sound::new(data, 0.6, false),
            cur_character: String::new(),
            opened: false,
            started: false,
            ended: false,
            is_ending: false,
            ending_timer: Timer::new(),
            finish_timer: Timer::new(),
            has_dialog,
            finished: false,
            alive: true,
        }
    }

    fn start_dialogue(&mut self) {
        if let Some(first) = self.lines.first_mut() {
            let mut parts = first.splitn(3, ':');
            parts.next();
            self.cur_character = parts.next().unwrap_or("").to_string();
            let rest = parts.next().unwrap_or("").trim().to_string();
            *first = rest;
        }
        let line = self.lines.first().cloned().unwrap_or_default();
        self.swag.reset(&line);
        self.hand.visible = false;
        self.ended = false;
        match self.cur_character.as_str() {
            "dad" => {
                self.portrait_right.visible = false;
                if !self.portrait_left.visible {
                    self.portrait_left.visible = true;
                    self.portrait_left.play("enter", false);
                }
            }
            "bf" => {
                self.portrait_left.visible = false;
                if !self.portrait_right.visible {
                    self.portrait_right.visible = true;
                    self.portrait_right.play("enter", false);
                }
            }
            _ => {}
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx) {
        if !self.alive {
            return;
        }
        let elapsed = ctx.elapsed;
        if self.song == "roses" {
            self.portrait_left.visible = false;
        }
        if self.song == "thorns" {
            self.portrait_left.color = 0x000000;
            self.swag.text.set_color(0xFFFFFFFF);
            self.drop_text.set_color(0xFF000000);
        }
        let shown = self.swag.text.text().to_string();
        self.drop_text.set_text(&shown);

        if self.fade_timer.update(elapsed) {
            let alpha = (self.bg_fade.alpha() + 0.2 * 0.7).min(0.7);
            self.bg_fade.set_alpha(alpha);
        }
        if self.box_sprite.anim.name() == "normalOpen" && self.box_sprite.anim.finished() {
            self.box_sprite.play("normal", false);
            self.opened = true;
        }
        if self.opened && !self.started {
            self.start_dialogue();
            self.started = true;
        }
        let any = ctx.keys.any_just_pressed;
        if any && self.ended {
            library::play_sound(ctx, "clickText", 0.8);
            if self.lines.len() <= 1 {
                if !self.is_ending {
                    self.is_ending = true;
                    if self.song == "senpai" || self.song == "thorns" {
                        ctx.audio.music.fade_out(2.2, 0.0);
                    }
                    self.ending_timer.start(0.2, 5);
                    self.finish_timer.start(1.2, 1);
                }
            } else {
                self.lines.remove(0);
                self.start_dialogue();
            }
        } else if any && self.started && self.swag.typing {
            self.swag.length = self.swag.full.len();
        }
        if self.ending_timer.update(elapsed) {
            let step = 1.0 / 5.0;
            let box_alpha = self.box_sprite.alpha() - step;
            self.box_sprite.set_alpha(box_alpha);
            let fade_alpha = self.bg_fade.alpha() - step * 0.7;
            self.bg_fade.set_alpha(fade_alpha);
            self.portrait_left.visible = false;
            self.portrait_right.visible = false;
            let text_alpha = self.swag.text.sprite.alpha() - step;
            self.swag.text.sprite.set_alpha(text_alpha);
            let hand_alpha = self.hand.alpha() - step;
            self.hand.set_alpha(hand_alpha);
            let text_alpha = self.swag.text.sprite.alpha();
            self.drop_text.sprite.set_alpha(text_alpha);
        }
        if self.finish_timer.update(elapsed) {
            self.finished = true;
            self.alive = false;
        }

        self.bg_fade.update(elapsed);
        self.portrait_left.update(elapsed);
        self.portrait_right.update(elapsed);
        if let Some(face) = self.face.as_mut() {
            face.update(elapsed);
        }
        self.box_sprite.update(elapsed);
        self.hand.update(elapsed);
        if self.swag.update(ctx, &mut self.type_sound) {
            self.hand.visible = true;
            self.ended = true;
        }
    }

    pub fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList, camera: usize) {
        if !self.alive {
            return;
        }
        self.bg_fade.draw(dl, camera);
        self.portrait_left.draw(dl, camera);
        self.portrait_right.draw(dl, camera);
        if let Some(face) = &self.face {
            face.draw(dl, camera);
        }
        self.box_sprite.draw(dl, camera);
        self.hand.draw(dl, camera);
        self.drop_text.draw(&mut ctx.gfx, dl, camera);
        self.swag.text.draw(&mut ctx.gfx, dl, camera);
    }
}