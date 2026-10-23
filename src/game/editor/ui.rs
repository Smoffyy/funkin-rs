use std::collections::HashMap;
use std::rc::Rc;

use fontdue::Font;
use miniquad::{KeyCode, TextureId};

use crate::engine::app::Ctx;
use crate::engine::atlas::Frames;
use crate::engine::gfx::{DrawList, Gfx};
use crate::engine::math::color_rgba;
use crate::engine::sprite::Sprite;
use crate::engine::text::Text;

pub const PANEL: u32 = 0xF0181A24;
pub const PANEL_LIGHT: u32 = 0xFF242736;
pub const WIDGET: u32 = 0xFF2E3245;
pub const WIDGET_HOVER: u32 = 0xFF3B4058;
pub const ACCENT: u32 = 0xFF7C5CFF;
pub const ACCENT_DIM: u32 = 0xFF4A3D8F;
pub const TEXT: u32 = 0xFFE8E8F0;
pub const TEXT_DIM: u32 = 0xFF9A9CB0;
pub const BORDER: u32 = 0xFF3A3E52;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

enum Cmd {
    Fill(Rect, u32),
    Text(String, f64, f64, f64, u32),
    Image(Rc<Frames>, usize, Rect, f64),
}

struct CachedText {
    text: Text,
    used: u64,
}

struct Dropdown {
    id: String,
    list: Rect,
    options: Vec<String>,
}

pub struct Ui {
    font: Rc<Font>,
    white: TextureId,
    cmds: Vec<Cmd>,
    overlay: Vec<Cmd>,
    texts: HashMap<(String, u32), CachedText>,
    frame: u64,
    pub mx: f64,
    pub my: f64,
    down: bool,
    clicked: bool,
    released: bool,
    pub wheel: f64,
    typed: Vec<char>,
    backspace: bool,
    delete: bool,
    commit: bool,
    cancel: bool,
    active: Option<String>,
    focus: Option<String>,
    buffer: String,
    dropdown: Option<Dropdown>,
    blocked: bool,
    pub hovering: bool,
    pub tooltip: Option<String>,
    scratch: Sprite,
    keys_used: bool,
}

impl Ui {
    pub fn new(ctx: &mut Ctx) -> Ui {
        let white = ctx.solid(1.0, 1.0, 0xFFFFFFFF).texture.id;
        Ui {
            font: ctx.data.fonts.ui.clone(),
            white,
            cmds: Vec::new(),
            overlay: Vec::new(),
            texts: HashMap::new(),
            frame: 0,
            mx: 0.0,
            my: 0.0,
            down: false,
            clicked: false,
            released: false,
            wheel: 0.0,
            typed: Vec::new(),
            backspace: false,
            delete: false,
            commit: false,
            cancel: false,
            active: None,
            focus: None,
            buffer: String::new(),
            dropdown: None,
            blocked: false,
            hovering: false,
            tooltip: None,
            scratch: Sprite::new(0.0, 0.0),
            keys_used: false,
        }
    }

    pub fn begin(&mut self, ctx: &Ctx) {
        self.frame += 1;
        self.cmds.clear();
        self.overlay.clear();
        self.mx = ctx.mouse.x;
        self.my = ctx.mouse.y;
        self.down = ctx.mouse.pressed(0);
        self.clicked = ctx.mouse.just_pressed(0);
        self.released = ctx.mouse.just_released(0);
        self.wheel = ctx.mouse.wheel;
        self.typed = ctx.keys.typed.clone();
        self.backspace = ctx.keys.just_pressed_or_repeated(KeyCode::Backspace);
        self.delete = ctx.keys.just_pressed_or_repeated(KeyCode::Delete);
        self.commit = ctx.keys.just_pressed(KeyCode::Enter) || ctx.keys.just_pressed(KeyCode::KpEnter) || ctx.keys.just_pressed(KeyCode::Tab);
        self.cancel = ctx.keys.just_pressed(KeyCode::Escape);
        self.hovering = false;
        self.tooltip = None;
        self.keys_used = false;
        self.blocked = self.dropdown.as_ref().map(|d| d.list.contains(self.mx, self.my)).unwrap_or(false);
        if self.clicked && !self.blocked {
            self.dropdown = None;
        }
        if !self.down {
            self.active = None;
        }
    }

    pub fn typing(&self) -> bool {
        self.focus.is_some() || self.keys_used
    }

    pub fn mouse_captured(&self) -> bool {
        self.hovering || self.blocked || self.active.is_some() || self.dropdown.is_some()
    }

    pub fn fill(&mut self, rect: Rect, color: u32) {
        self.cmds.push(Cmd::Fill(rect, color));
    }

    pub fn image(&mut self, frames: &Rc<Frames>, index: usize, rect: Rect, alpha: f64) {
        self.cmds.push(Cmd::Image(frames.clone(), index, rect, alpha));
    }

    pub fn outline(&mut self, rect: Rect, thickness: f64, color: u32) {
        let t = thickness;
        self.fill(Rect::new(rect.x, rect.y, rect.w, t), color);
        self.fill(Rect::new(rect.x, rect.y + rect.h - t, rect.w, t), color);
        self.fill(Rect::new(rect.x, rect.y, t, rect.h), color);
        self.fill(Rect::new(rect.x + rect.w - t, rect.y, t, rect.h), color);
    }

    fn entry(&mut self, text: &str, size: f64) -> &mut CachedText {
        let key = (text.to_string(), size as u32);
        let frame = self.frame;
        let font = self.font.clone();
        let entry = self.texts.entry(key).or_insert_with(|| CachedText { text: Text::new(0.0, 0.0, 0.0, text, size, font), used: frame });
        entry.used = frame;
        entry
    }

    pub fn measure(&mut self, text: &str, size: f64) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        self.entry(text, size).text.width()
    }

    pub fn text(&mut self, text: &str, x: f64, y: f64, size: f64, color: u32) {
        if !text.is_empty() {
            self.cmds.push(Cmd::Text(text.to_string(), x, y, size, color));
        }
    }

    pub fn text_centered(&mut self, text: &str, rect: Rect, size: f64, color: u32) {
        let w = self.measure(text, size);
        let x = rect.x + (rect.w - w) / 2.0;
        let y = rect.y + (rect.h - size) / 2.0 - 1.0;
        self.text(text, x, y, size, color);
    }

    pub fn text_right(&mut self, text: &str, right: f64, y: f64, size: f64, color: u32) {
        let w = self.measure(text, size);
        self.text(text, right - w, y, size, color);
    }

    pub fn label(&mut self, text: &str, rect: Rect) {
        let y = rect.y + (rect.h - 18.0) / 2.0 - 1.0;
        self.text(text, rect.x, y, 18.0, TEXT_DIM);
    }

    fn interact(&mut self, id: &str, rect: Rect) -> (bool, bool) {
        let hover = !self.blocked && rect.contains(self.mx, self.my);
        if hover {
            self.hovering = true;
        }
        if hover && self.clicked {
            self.active = Some(id.to_string());
            if self.focus.as_deref() != Some(id) {
                self.focus = None;
            }
            return (true, true);
        }
        (hover, false)
    }

    pub fn tip(&mut self, rect: Rect, text: &str) {
        if !self.blocked && rect.contains(self.mx, self.my) {
            self.tooltip = Some(text.to_string());
        }
    }

    pub fn button(&mut self, id: &str, rect: Rect, label: &str) -> bool {
        self.button_colored(id, rect, label, false)
    }

    pub fn button_colored(&mut self, id: &str, rect: Rect, label: &str, selected: bool) -> bool {
        let (hover, clicked) = self.interact(id, rect);
        let pressed = self.active.as_deref() == Some(id) && self.down;
        let color = match (selected, hover || pressed) {
            (true, _) => ACCENT,
            (false, true) => WIDGET_HOVER,
            _ => WIDGET,
        };
        self.fill(rect, color);
        self.text_centered(label, rect, 18.0, TEXT);
        clicked
    }

    pub fn checkbox(&mut self, id: &str, rect: Rect, label: &str, value: &mut bool) -> bool {
        let (hover, clicked) = self.interact(id, rect);
        let size = rect.h.min(22.0);
        let box_rect = Rect::new(rect.x, rect.y + (rect.h - size) / 2.0, size, size);
        self.fill(box_rect, if hover { WIDGET_HOVER } else { WIDGET });
        if *value {
            self.fill(Rect::new(box_rect.x + 5.0, box_rect.y + 5.0, size - 10.0, size - 10.0), ACCENT);
        }
        let y = rect.y + (rect.h - 18.0) / 2.0 - 1.0;
        self.text(label, rect.x + size + 8.0, y, 18.0, TEXT);
        if clicked {
            *value = !*value;
        }
        clicked
    }

    pub fn tabs(&mut self, id: &str, rect: Rect, labels: &[&str], selected: &mut usize) -> bool {
        let w = rect.w / labels.len() as f64;
        let mut changed = false;
        for (i, label) in labels.iter().enumerate() {
            let r = Rect::new(rect.x + w * i as f64, rect.y, w - 2.0, rect.h);
            if self.button_colored(&format!("{id}#{i}"), r, label, *selected == i) && *selected != i {
                *selected = i;
                changed = true;
            }
        }
        changed
    }

    fn edit_field(&mut self, id: &str, rect: Rect, display: &str, numeric: bool) -> Option<String> {
        let (hover, clicked) = self.interact(id, rect);
        let focused = self.focus.as_deref() == Some(id);
        if clicked && !focused {
            self.focus = Some(id.to_string());
            self.buffer = display.to_string();
        }
        let focused = self.focus.as_deref() == Some(id);
        let mut result = None;
        if focused {
            for ch in std::mem::take(&mut self.typed) {
                if !numeric || ch.is_ascii_digit() || ch == '.' || ch == '-' {
                    self.buffer.push(ch);
                }
            }
            if self.backspace {
                self.buffer.pop();
            }
            if self.delete {
                self.buffer.clear();
            }
            let clicked_away = self.clicked && !rect.contains(self.mx, self.my);
            if self.commit || clicked_away {
                result = Some(self.buffer.clone());
                self.focus = None;
                self.keys_used = self.commit;
            } else if self.cancel {
                self.focus = None;
                self.keys_used = true;
            }
        }
        let still = self.focus.as_deref() == Some(id);
        self.fill(rect, if still { PANEL_LIGHT } else if hover { WIDGET_HOVER } else { WIDGET });
        if still {
            self.outline(rect, 2.0, ACCENT);
        }
        let shown = if still { format!("{}_", self.buffer) } else { display.to_string() };
        let y = rect.y + (rect.h - 18.0) / 2.0 - 1.0;
        self.text(&shown, rect.x + 6.0, y, 18.0, TEXT);
        result
    }

    pub fn text_field(&mut self, id: &str, rect: Rect, value: &mut String) -> bool {
        let current = value.clone();
        match self.edit_field(id, rect, &current, false) {
            Some(text) if text != *value => {
                *value = text;
                true
            }
            _ => false,
        }
    }

    pub fn stepper(&mut self, id: &str, rect: Rect, value: &mut f64, step: f64, min: f64, max: f64, decimals: usize) -> bool {
        let button = rect.h;
        let minus = Rect::new(rect.x, rect.y, button, rect.h);
        let plus = Rect::new(rect.x + rect.w - button, rect.y, button, rect.h);
        let field = Rect::new(rect.x + button + 2.0, rect.y, rect.w - button * 2.0 - 4.0, rect.h);
        let before = *value;
        if self.button(&format!("{id}-"), minus, "-") {
            *value -= step;
        }
        if self.button(&format!("{id}+"), plus, "+") {
            *value += step;
        }
        if field.contains(self.mx, self.my) && self.wheel != 0.0 && !self.blocked {
            *value += step * self.wheel.signum();
        }
        let display = format!("{:.*}", decimals, value);
        if let Some(text) = self.edit_field(id, field, &display, true) {
            if let Ok(parsed) = text.trim().parse::<f64>() {
                *value = parsed;
            }
        }
        let scale = 10f64.powi(decimals as i32);
        *value = ((value.clamp(min, max)) * scale).round() / scale;
        *value != before
    }

    pub fn dropdown(&mut self, id: &str, rect: Rect, options: &[String], selected: &mut usize) -> bool {
        let label = options.get(*selected).cloned().unwrap_or_default();
        let open = self.dropdown.as_ref().map(|d| d.id == id).unwrap_or(false);
        let mut changed = false;
        if open {
            let list = self.dropdown.as_ref().map(|d| d.list).unwrap_or(rect);
            let row = rect.h;
            let count = options.len();
            for i in 0..count {
                let r = Rect::new(list.x, list.y + row * i as f64, list.w, row);
                if r.contains(self.mx, self.my) && self.clicked {
                    *selected = i;
                    changed = true;
                    self.dropdown = None;
                }
            }
        }
        let (_, clicked) = self.interact(id, rect);
        if clicked && !open {
            let row = rect.h;
            let height = row * options.len() as f64;
            let y = if rect.y + rect.h + height > 715.0 { (rect.y - height).max(5.0) } else { rect.y + rect.h };
            self.dropdown = Some(Dropdown { id: id.to_string(), list: Rect::new(rect.x, y, rect.w, height), options: options.to_vec() });
        } else if clicked && open {
            self.dropdown = None;
        }
        self.fill(rect, WIDGET);
        let y = rect.y + (rect.h - 18.0) / 2.0 - 1.0;
        self.text(&label, rect.x + 6.0, y, 18.0, TEXT);
        self.text_right("v", rect.x + rect.w - 6.0, y, 18.0, TEXT_DIM);
        changed
    }

    fn draw_dropdown(&mut self) {
        let Some(d) = self.dropdown.as_ref() else { return };
        let list = d.list;
        let options = d.options.clone();
        let row = list.h / options.len().max(1) as f64;
        let mut cmds = vec![Cmd::Fill(Rect::new(list.x - 2.0, list.y - 2.0, list.w + 4.0, list.h + 4.0), BORDER), Cmd::Fill(list, PANEL_LIGHT)];
        for (i, option) in options.iter().enumerate() {
            let r = Rect::new(list.x, list.y + row * i as f64, list.w, row);
            if r.contains(self.mx, self.my) {
                cmds.push(Cmd::Fill(r, ACCENT_DIM));
            }
            cmds.push(Cmd::Text(option.clone(), r.x + 6.0, r.y + (row - 18.0) / 2.0 - 1.0, 18.0, TEXT));
        }
        self.overlay.extend(cmds);
    }

    fn draw_tooltip(&mut self) {
        let Some(tip) = self.tooltip.clone() else { return };
        let w = self.measure(&tip, 16.0) + 12.0;
        let x = (self.mx + 14.0).min(1280.0 - w - 4.0);
        let y = (self.my + 18.0).min(690.0);
        self.overlay.push(Cmd::Fill(Rect::new(x, y, w, 24.0), 0xF0000000));
        self.overlay.push(Cmd::Text(tip, x + 6.0, y + 3.0, 16.0, TEXT));
    }

    pub fn finish(&mut self) {
        self.draw_dropdown();
        self.draw_tooltip();
        let frame = self.frame;
        self.texts.retain(|_, t| frame - t.used < 120);
    }

    pub fn render(&mut self, gfx: &mut Gfx, dl: &mut DrawList, camera: usize) {
        let cmds = std::mem::take(&mut self.cmds);
        let overlay = std::mem::take(&mut self.overlay);
        for cmd in cmds.iter().chain(overlay.iter()) {
            self.render_cmd(gfx, dl, camera, cmd);
        }
        self.cmds = cmds;
        self.overlay = overlay;
    }

    fn render_cmd(&mut self, gfx: &mut Gfx, dl: &mut DrawList, camera: usize, cmd: &Cmd) {
        match cmd {
            Cmd::Fill(r, color) => dl.fill(camera, self.white, [r.x, r.y, r.w, r.h], color_rgba(*color)),
            Cmd::Image(frames, index, rect, alpha) => {
                let s = &mut self.scratch;
                if s.frames.as_ref().map(|f| !Rc::ptr_eq(f, frames)).unwrap_or(true) {
                    s.set_frames(frames.clone());
                    s.antialiasing = true;
                }
                s.anim.frame_index = *index;
                s.scale.set(rect.w / s.frame_width().max(1.0), rect.h / s.frame_height().max(1.0));
                s.update_hitbox();
                s.set_position(rect.x, rect.y);
                s.set_alpha(*alpha);
                s.draw(dl, camera);
            }
            Cmd::Text(text, x, y, size, color) => {
                let entry = self.entry(text, *size);
                entry.text.sprite.x = x.round();
                entry.text.sprite.y = y.round();
                entry.text.sprite.color = color & 0xFFFFFF;
                entry.text.sprite.set_alpha(((color >> 24) & 0xFF) as f64 / 255.0);
                entry.text.draw(gfx, dl, camera);
            }
        }
    }
}