use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use fontdue::{Font, Metrics};

use super::atlas::Frames;
use super::gfx::{DrawList, Gfx, Texture};
use super::sprite::Sprite;

thread_local! {
    static GARBAGE: RefCell<Vec<Texture>> = const { RefCell::new(Vec::new()) };
}

struct Glyph {
    metrics: Metrics,
    coverage: Vec<u8>,
}

thread_local! {
    static GLYPHS: RefCell<HashMap<(usize, char, u32), Rc<Glyph>>> = RefCell::new(HashMap::new());
    static GLYPH_CACHE: Cell<bool> = const { Cell::new(false) };
}

pub fn enable_glyph_cache() {
    GLYPH_CACHE.with(|c| c.set(true));
}

fn glyph_caching() -> bool {
    GLYPH_CACHE.with(|c| c.get())
}

fn cached_glyph(font: &Rc<Font>, ch: char, px: f32) -> Rc<Glyph> {
    let caching = glyph_caching();
    let key = (Rc::as_ptr(font) as usize, ch, px.to_bits());
    if caching {
        if let Some(glyph) = GLYPHS.with(|g| g.borrow().get(&key).cloned()) {
            return glyph;
        }
    }
    let (metrics, coverage) = font.rasterize(ch, px);
    let glyph = Rc::new(Glyph { metrics, coverage });
    if caching {
        GLYPHS.with(|g| g.borrow_mut().insert(key, glyph.clone()));
    }
    glyph
}

pub fn collect_garbage(gfx: &mut Gfx) {
    let dead: Vec<Texture> = GARBAGE.with(|g| std::mem::take(&mut *g.borrow_mut()));
    for texture in dead {
        gfx.delete_texture(texture);
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Border {
    None,
    Outline,
}

const GUTTER: f64 = 2.0;

struct Line {
    glyphs: Vec<(char, f64)>,
    width: f64,
}

pub struct Text {
    pub sprite: Sprite,
    text: String,
    font: Rc<Font>,
    size: f64,
    color: u32,
    align: Align,
    border: Border,
    border_color: u32,
    border_size: f64,
    border_quality: f64,
    field_width: f64,
    dirty: bool,
    texture: Option<Texture>,
    pixels: Vec<u8>,
    bitmap: (u32, u32),
}

impl Drop for Text {
    fn drop(&mut self) {
        if let Some(t) = self.texture.take() {
            GARBAGE.with(|g| g.borrow_mut().push(t));
        }
    }
}

impl Text {
    pub fn new(x: f64, y: f64, field_width: f64, text: &str, size: f64, font: Rc<Font>) -> Text {
        let mut t = Text {
            sprite: Sprite::new(x, y),
            text: text.to_string(),
            font,
            size,
            color: 0xFFFFFFFF,
            align: Align::Left,
            border: Border::None,
            border_color: 0x00000000,
            border_size: 1.0,
            border_quality: 1.0,
            field_width,
            dirty: true,
            texture: None,
            pixels: Vec::new(),
            bitmap: (0, 0),
        };
        t.relayout();
        t
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: &str) {
        if self.text != text {
            self.text = text.to_string();
            self.relayout();
        }
    }

    pub fn set_format(&mut self, font: Rc<Font>, size: f64, color: u32, align: Align, border: Border, border_color: u32) {
        self.font = font;
        self.size = size;
        self.color = color;
        self.align = align;
        self.border = border;
        self.border_color = border_color;
        self.relayout();
    }

    pub fn set_size(&mut self, size: f64) {
        if self.size != size {
            self.size = size;
            self.relayout();
        }
    }

    pub fn set_color(&mut self, color: u32) {
        if self.color != color {
            self.color = color;
            self.dirty = true;
        }
    }

    pub fn set_align(&mut self, align: Align) {
        if self.align != align {
            self.align = align;
            self.dirty = true;
        }
    }

    pub fn width(&self) -> f64 {
        self.sprite.width
    }

    fn px(&self) -> f32 {
        self.size as f32
    }

    fn advance(&self, ch: char) -> f64 {
        let px = self.px();
        let advance = if glyph_caching() {
            cached_glyph(&self.font, ch, px).metrics.advance_width
        } else {
            self.font.metrics(ch, px).advance_width
        };
        advance as f64
    }

    fn line_height(&self) -> f64 {
        match self.font.horizontal_line_metrics(self.px()) {
            Some(m) => (m.ascent - m.descent + m.line_gap) as f64,
            None => self.size * 1.2,
        }
    }

    fn ascent(&self) -> f64 {
        self.font.horizontal_line_metrics(self.px()).map(|m| m.ascent as f64).unwrap_or(self.size)
    }

    fn layout(&self) -> Vec<Line> {
        let px = self.px();
        let wrap = if self.field_width > 0.0 { Some(self.field_width - GUTTER * 2.0) } else { None };
        let mut lines = Vec::new();
        for raw in self.text.split('\n') {
            let raw = raw.trim_end_matches('\r');
            let mut line = Line { glyphs: Vec::new(), width: 0.0 };
            let mut last_space: Option<usize> = None;
            let mut prev: Option<char> = None;
            for ch in raw.chars() {
                let kern = prev.and_then(|p| self.font.horizontal_kern(p, ch, px)).unwrap_or(0.0) as f64;
                let advance = self.advance(ch);
                let pen = line.width + kern;
                if let Some(limit) = wrap {
                    if pen + advance > limit && !line.glyphs.is_empty() && ch != ' ' {
                        if let Some(space) = last_space {
                            let rest: Vec<(char, f64)> = line.glyphs.split_off(space + 1);
                            line.glyphs.pop();
                            line.width = line.glyphs.last().map(|(c, x)| x + self.advance(*c)).unwrap_or(0.0);
                            lines.push(line);
                            let shift = rest.first().map(|(_, x)| *x).unwrap_or(0.0);
                            let mut next = Line { glyphs: rest.into_iter().map(|(c, x)| (c, x - shift)).collect(), width: 0.0 };
                            next.width = next.glyphs.last().map(|(c, x)| x + self.advance(*c)).unwrap_or(0.0);
                            line = next;
                            last_space = None;
                        } else {
                            lines.push(line);
                            line = Line { glyphs: Vec::new(), width: 0.0 };
                        }
                    }
                }
                let pen = line.width + if line.glyphs.is_empty() { 0.0 } else { kern };
                if ch == ' ' {
                    last_space = Some(line.glyphs.len());
                }
                line.glyphs.push((ch, pen));
                line.width = pen + advance;
                prev = Some(ch);
            }
            lines.push(line);
        }
        lines
    }

    fn relayout(&mut self) {
        let lines = self.layout();
        let text_width = lines.iter().map(|l| l.width).fold(0.0, f64::max);
        let text_height = lines.len() as f64 * self.line_height();
        let w = if self.field_width > 0.0 { self.field_width.ceil() } else { (text_width + GUTTER * 2.0).ceil() };
        let h = if self.text.is_empty() { (self.line_height() + GUTTER * 2.0).ceil() } else { text_height.ceil() + GUTTER * 2.0 };
        let (w, h) = (w.max(1.0) as u32, h.max(1.0) as u32);
        if (w, h) != self.bitmap {
            self.bitmap = (w, h);
            self.sprite.width = w as f64;
            self.sprite.height = h as f64;
            self.sprite.origin.set(w as f64 * 0.5, h as f64 * 0.5);
        }
        self.dirty = true;
    }

    fn rasterize(&mut self) {
        let (w, h) = self.bitmap;
        let (wi, hi) = (w as usize, h as usize);
        let mut mask = vec![0f32; wi * hi];
        let px = self.px();
        let lines = self.layout();
        let line_height = self.line_height();
        let ascent = self.ascent();
        let inner = w as f64 - GUTTER * 2.0;
        for (row, line) in lines.iter().enumerate() {
            let start = match self.align {
                Align::Left => GUTTER,
                Align::Center => GUTTER + ((inner - line.width) / 2.0).floor(),
                Align::Right => GUTTER + (inner - line.width).floor(),
            };
            let baseline = (GUTTER + ascent + row as f64 * line_height).round() as i32;
            for &(ch, pen) in &line.glyphs {
                if ch.is_whitespace() {
                    continue;
                }
                let glyph = cached_glyph(&self.font, ch, px);
                let m = &glyph.metrics;
                let coverage = &glyph.coverage;
                let gx = (start + pen).round() as i32 + m.xmin;
                let gy = baseline - m.ymin - m.height as i32;
                for yy in 0..m.height {
                    let ty = gy + yy as i32;
                    if ty < 0 || ty >= hi as i32 {
                        continue;
                    }
                    for xx in 0..m.width {
                        let tx = gx + xx as i32;
                        if tx < 0 || tx >= wi as i32 {
                            continue;
                        }
                        let c = coverage[yy * m.width + xx] as f32 / 255.0;
                        let i = ty as usize * wi + tx as usize;
                        mask[i] = mask[i].max(c);
                    }
                }
            }
        }
        let mut rgba = vec![[0f32; 4]; wi * hi];
        let over = |dst: &mut [f32; 4], color: u32, a: f32| {
            if a <= 0.0 {
                return;
            }
            let r = ((color >> 16) & 0xFF) as f32 / 255.0;
            let g = ((color >> 8) & 0xFF) as f32 / 255.0;
            let b = (color & 0xFF) as f32 / 255.0;
            let inv = 1.0 - a;
            dst[0] = r * a + dst[0] * inv;
            dst[1] = g * a + dst[1] * inv;
            dst[2] = b * a + dst[2] * inv;
            dst[3] = a + dst[3] * inv;
        };
        let sample = |x: i32, y: i32| -> f32 {
            if x < 0 || y < 0 || x >= wi as i32 || y >= hi as i32 { 0.0 } else { mask[y as usize * wi + x as usize] }
        };
        let border_alpha = ((self.border_color >> 24) & 0xFF) as f32 / 255.0;
        let border_alpha = if border_alpha == 0.0 { 1.0 } else { border_alpha };
        let iterations = ((self.border_size * self.border_quality) as i32).max(1);
        let delta = self.border_size / iterations as f64;
        let mut offsets: Vec<(i32, i32)> = Vec::new();
        match self.border {
            Border::Outline => {
                for i in 1..=iterations {
                    let d = (delta * i as f64).round() as i32;
                    offsets.extend_from_slice(&[(-d, -d), (0, -d), (d, -d), (d, 0), (d, d), (0, d), (-d, d), (-d, 0)]);
                }
            }
            Border::None => {}
        }
        for &(ox, oy) in &offsets {
            for y in 0..hi as i32 {
                for x in 0..wi as i32 {
                    let a = sample(x - ox, y - oy) * border_alpha;
                    over(&mut rgba[y as usize * wi + x as usize], self.border_color, a);
                }
            }
        }
        let text_alpha = ((self.color >> 24) & 0xFF) as f32 / 255.0;
        let text_alpha = if text_alpha == 0.0 { 1.0 } else { text_alpha };
        for i in 0..wi * hi {
            over(&mut rgba[i], self.color, mask[i] * text_alpha);
        }
        self.pixels.clear();
        self.pixels.reserve(wi * hi * 4);
        for p in rgba {
            self.pixels.extend_from_slice(&[
                (p[0] * 255.0).round() as u8,
                (p[1] * 255.0).round() as u8,
                (p[2] * 255.0).round() as u8,
                (p[3] * 255.0).round() as u8,
            ]);
        }
    }

    pub fn prepare(&mut self, gfx: &mut Gfx) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        self.rasterize();
        let (w, h) = self.bitmap;
        let texture = match self.texture.as_mut() {
            Some(t) => {
                gfx.update_texture(t, w, h, &self.pixels);
                *t
            }
            None => {
                let t = gfx.create_texture(w, h, &self.pixels);
                self.texture = Some(t);
                t
            }
        };
        let keep = (self.sprite.width, self.sprite.height, self.sprite.origin, self.sprite.offset);
        self.sprite.set_frames(Rc::new(Frames::single(texture)));
        self.sprite.width = keep.0;
        self.sprite.height = keep.1;
        self.sprite.origin = keep.2;
        self.sprite.offset = keep.3;
    }

    pub fn draw(&mut self, gfx: &mut Gfx, dl: &mut DrawList, camera: usize) {
        if !self.sprite.visible || !self.sprite.exists || self.text.is_empty() {
            return;
        }
        self.prepare(gfx);
        self.sprite.draw(dl, camera);
    }
}