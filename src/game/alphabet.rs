use std::rc::Rc;

use crate::engine::app::Ctx;
use crate::engine::atlas::Frames;
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::math::{frame_lerp, lerp, remap, Point};
use crate::engine::sprite::Sprite;

#[derive(Clone, Copy, PartialEq)]
pub enum Font {
    Bold,
    Default,
}

struct Letter {
    sprite: Sprite,
    x: f64,
    y: f64,
}

pub struct Alphabet {
    pub x: f64,
    pub y: f64,
    pub scroll: Point,
    pub visible: bool,
    pub target_y: f64,
    pub is_menu_item: bool,
    alpha: f64,
    letters: Vec<Letter>,
    frames: Rc<Frames>,
    font: Font,
    max_height: f64,
    text: String,
}

fn symbol_prefix(ch: char) -> Option<&'static str> {
    Some(match ch {
        '.' => "period",
        ',' => "comma",
        '\'' => "apostraphie",
        '?' => "question mark",
        '!' => "exclamation point",
        '/' => "forward slash",
        '*' => "multiply x",
        '#' => "hashtag",
        '$' => "dollarsign",
        '&' => "amp",
        '(' => "start parentheses",
        ')' => "end parentheses",
        _ => return None,
    })
}

fn glyph_prefix(ch: char, font: Font) -> String {
    if ch.is_ascii_alphabetic() {
        return match font {
            Font::Bold => format!("{} bold", ch.to_ascii_uppercase()),
            Font::Default if ch.is_ascii_uppercase() => format!("{ch} capital"),
            Font::Default => format!("{ch} lowercase"),
        };
    }
    symbol_prefix(ch).map(str::to_string).unwrap_or_else(|| ch.to_string())
}

fn glyph(frames: &Rc<Frames>, prefix: &str, x: f64, y: f64) -> Option<Sprite> {
    let mut s = Sprite::with_frames(x, y, frames.clone());
    s.add_by_prefix("anim", prefix, 24.0, true);
    if !s.anim.exists("anim") {
        return None;
    }
    s.play("anim", false);
    s.update_hitbox();
    s.antialiasing = true;
    Some(s)
}

impl Alphabet {
    fn empty(ctx: &mut Ctx, x: f64, y: f64, font: Font) -> Alphabet {
        let frames = ctx.atlas("alphabet");
        let is_letter = |name: &str| match font {
            Font::Bold => name.contains(" bold"),
            Font::Default => name.contains(" capital") || name.contains(" lowercase"),
        };
        let max_height = frames.frames.iter().filter(|f| is_letter(&f.name)).map(|f| f.source_height).fold(0.0, f64::max);
        Alphabet {
            x,
            y,
            scroll: Point::new(1.0, 1.0),
            visible: true,
            target_y: 0.0,
            is_menu_item: false,
            alpha: 1.0,
            letters: Vec::new(),
            frames,
            font,
            max_height,
            text: String::new(),
        }
    }

    pub fn new(ctx: &mut Ctx, x: f64, y: f64, text: &str, bold: bool) -> Alphabet {
        let font = if bold { Font::Bold } else { Font::Default };
        let mut a = Alphabet::empty(ctx, x, y, font);
        a.text = text.to_string();
        a.add_text();
        a
    }

    pub fn atlas(ctx: &mut Ctx, x: f64, y: f64, text: &str, font: Font) -> Alphabet {
        let mut a = Alphabet::empty(ctx, x, y, font);
        a.set_text(text);
        a
    }

    fn add_text(&mut self) {
        let mut x_pos = 0.0;
        let mut last_was_space = false;
        let chars: Vec<char> = self.text.chars().collect();
        for ch in chars {
            if ch == ' ' || ch == '-' {
                last_was_space = true;
            }
            if !ch.is_ascii_alphabetic() {
                continue;
            }
            if let Some(last) = self.letters.last() {
                x_pos = last.x + last.sprite.width;
            }
            if last_was_space {
                x_pos += 40.0;
                last_was_space = false;
            }
            let Some(sprite) = glyph(&self.frames, &glyph_prefix(ch, self.font), 0.0, 0.0) else { continue };
            let y = if self.font == Font::Bold { 0.0 } else { 110.0 - sprite.height };
            self.letters.push(Letter { sprite, x: x_pos, y });
        }
    }

    pub fn set_text(&mut self, text: &str) {
        let text = if self.font == Font::Bold { text.to_uppercase() } else { text.to_string() };
        if text == self.text && !self.letters.is_empty() {
            return;
        }
        self.text = text;
        self.letters.clear();
        let mut x_pos = 0.0;
        let mut y_pos = 0.0;
        let chars: Vec<char> = self.text.chars().collect();
        for ch in chars {
            match ch {
                ' ' => x_pos += 40.0,
                '\n' => {
                    x_pos = 0.0;
                    y_pos += self.max_height;
                }
                _ => {
                    let Some(sprite) = glyph(&self.frames, &glyph_prefix(ch, self.font), 0.0, 0.0) else { continue };
                    let y = y_pos + self.max_height - sprite.height;
                    let width = sprite.width;
                    self.letters.push(Letter { sprite, x: x_pos, y });
                    x_pos += width;
                }
            }
        }
    }

    pub fn set_alpha(&mut self, alpha: f64) {
        self.alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn width(&self) -> f64 {
        let min = self.letters.iter().map(|l| l.x).fold(f64::INFINITY, f64::min);
        let max = self.letters.iter().map(|l| l.x + l.sprite.width).fold(f64::NEG_INFINITY, f64::max);
        if self.letters.is_empty() { 0.0 } else { max - min }
    }

    pub fn height(&self) -> f64 {
        let min = self.letters.iter().map(|l| l.y).fold(f64::INFINITY, f64::min);
        let max = self.letters.iter().map(|l| l.y + l.sprite.height).fold(f64::NEG_INFINITY, f64::max);
        if self.letters.is_empty() { 0.0 } else { max - min }
    }

    pub fn screen_center_x(&mut self) {
        self.x = (GAME_WIDTH - self.width()) / 2.0;
    }

    pub fn update(&mut self, elapsed: f64) {
        if self.is_menu_item {
            let scaled = remap(self.target_y, 0.0, 1.0, 0.0, 1.3);
            self.y = lerp(self.y, scaled * 120.0 + GAME_HEIGHT * 0.48, frame_lerp(elapsed, 0.16));
            self.x = lerp(self.x, self.target_y * 20.0 + 90.0, frame_lerp(elapsed, 0.16));
        }
        for letter in self.letters.iter_mut() {
            letter.sprite.update(elapsed);
        }
    }

    pub fn draw(&mut self, dl: &mut DrawList, camera: usize) {
        if !self.visible {
            return;
        }
        for letter in self.letters.iter_mut() {
            letter.sprite.set_position(self.x + letter.x, self.y + letter.y);
            letter.sprite.scroll = self.scroll;
            letter.sprite.set_alpha(self.alpha);
            letter.sprite.draw(dl, camera);
        }
    }
}