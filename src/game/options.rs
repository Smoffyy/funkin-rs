use miniquad::KeyCode;

use crate::engine::app::{Ctx, State};
use crate::engine::camera::{Camera, FollowStyle, Target};
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::input::key_name;
use crate::engine::sprite::{Axes, Sprite};

use super::data::{Control, Input};
use super::library;
use super::alphabet::{Alphabet, Font};
use super::widgets::Flicker;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Options,
    Preferences,
    Controls,
}

#[derive(Clone, Copy, PartialEq)]
enum Pref {
    Downscroll,
    FlashingMenu,
    CameraZoom,
    FpsCounter,
    AutoPause,
}

const PREFS: [(&str, Pref); 5] = [
    ("downscroll", Pref::Downscroll),
    ("flashing menu", Pref::FlashingMenu),
    ("camera zooming on beat", Pref::CameraZoom),
    ("fps counter", Pref::FpsCounter),
    ("auto pause", Pref::AutoPause),
];

struct Binding {
    text: Alphabet,
    control: Control,
    key: Option<KeyCode>,
}

pub struct Options {
    bg: Sprite,
    page: Page,
    enabled: bool,
    busy: bool,
    flicker: Flicker,
    flicker_target: (Page, usize),
    option_items: Vec<Alphabet>,
    option_selected: usize,
    pref_items: Vec<Alphabet>,
    pref_values: Vec<Alphabet>,
    pref_selected: usize,
    pref_cam: usize,
    control_headers: Vec<Alphabet>,
    control_labels: Vec<Alphabet>,
    bindings: Vec<Binding>,
    binding_selected: usize,
    control_cam: usize,
    prompt_bg: Sprite,
    prompt_text: Alphabet,
    prompt_open: bool,
}

fn pref_value(ctx: &Ctx, pref: Pref) -> bool {
    let p = &ctx.data.prefs;
    match pref {
        Pref::Downscroll => p.downscroll,
        Pref::FlashingMenu => p.flashing_menu,
        Pref::CameraZoom => p.camera_zoom,
        Pref::FpsCounter => p.fps_counter,
        Pref::AutoPause => p.auto_pause,
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}

impl Options {
    pub fn new(ctx: &mut Ctx) -> Options {
        ctx.trans_in = ctx.default_trans_in;
        ctx.trans_out = ctx.default_trans_out;
        let mut bg = Sprite::with_frames(0.0, 0.0, ctx.image("menuDesat"));
        bg.color = 0xEA71FD;
        let w = bg.width;
        bg.set_graphic_size((w * 1.1) as i64, 0);
        bg.update_hitbox();
        bg.screen_center(Axes::XY);
        bg.scroll.set(0.0, 0.0);

        let mut option_items = Vec::new();
        for (i, label) in ["PREFERENCES", "CONTROLS", "EXIT"].iter().enumerate() {
            let mut t = Alphabet::atlas(ctx, 0.0, 100.0 + i as f64 * 100.0, label, Font::Bold);
            t.screen_center_x();
            t.scroll.set(0.0, 0.0);
            option_items.push(t);
        }

        let mut pref_cam = Camera::transparent();
        pref_cam.visible = false;
        let pref_cam = ctx.add_camera(pref_cam);
        let mut pref_items = Vec::new();
        let mut pref_values = Vec::new();
        for (i, (label, pref)) in PREFS.iter().enumerate() {
            let y = 120.0 * i as f64 + 30.0;
            pref_items.push(Alphabet::atlas(ctx, 120.0, y, &label.to_uppercase(), Font::Bold));
            let value = pref_value(ctx, *pref);
            pref_values.push(Alphabet::atlas(ctx, 0.0, y + 14.0, on_off(value), Font::Default));
        }
        {
            let y = pref_items[0].y;
            let cam = ctx.camera(pref_cam);
            cam.follow(Target { x: GAME_WIDTH / 2.0, y, width: 140.0, height: 70.0 }, FollowStyle::LockOn, 0.06);
            cam.deadzone = Some([0.0, 160.0, GAME_WIDTH, 40.0]);
            cam.min_scroll_y = Some(0.0);
        }

        let mut control_cam = Camera::transparent();
        control_cam.visible = false;
        let control_cam = ctx.add_camera(control_cam);
        let mut control_headers = Vec::new();
        let mut control_labels = Vec::new();
        let mut bindings = Vec::new();
        let mut y = 30.0;
        let spacer = 70.0;
        let mut header: Option<&str> = None;
        for control in Control::ALL {
            let mut name = control.name().to_string();
            if header != Some("UI_") && name.starts_with("UI_") {
                header = Some("UI_");
                let mut h = Alphabet::atlas(ctx, 0.0, y, "UI", Font::Bold);
                h.screen_center_x();
                control_headers.push(h);
                y += spacer;
            } else if header != Some("NOTE_") && name.starts_with("NOTE_") {
                header = Some("NOTE_");
                let mut h = Alphabet::atlas(ctx, 0.0, y, "NOTES", Font::Bold);
                h.screen_center_x();
                control_headers.push(h);
                y += spacer;
            }
            if let Some(prefix) = header {
                if let Some(stripped) = name.strip_prefix(prefix) {
                    name = stripped.to_string();
                }
            }
            let mut label = Alphabet::atlas(ctx, 150.0, y, &name.replace('_', " "), Font::Bold);
            label.set_alpha(0.6);
            let keys: Vec<KeyCode> = ctx.data.controls.keys(control).iter().copied().filter(|k| *k != KeyCode::Escape).collect();
            for column in 0..2 {
                let key = keys.get(column).copied();
                let text = key.map(key_name).unwrap_or("---");
                let mut t = Alphabet::atlas(ctx, 150.0 + 400.0 + column as f64 * 300.0, y + 8.0, text, Font::Default);
                t.set_alpha(0.6);
                bindings.push(Binding { text: t, control, key });
            }
            control_labels.push(label);
            y += spacer;
        }
        {
            let y = bindings[0].text.y;
            let cam = ctx.camera(control_cam);
            cam.follow(Target { x: GAME_WIDTH / 2.0, y, width: 70.0, height: 70.0 }, FollowStyle::LockOn, 0.06);
            cam.deadzone = Some([0.0, 100.0, GAME_WIDTH, GAME_HEIGHT - 200.0]);
            cam.min_scroll_y = Some(0.0);
        }
        let mut prompt_bg = Sprite::with_frames(0.0, 0.0, ctx.solid(GAME_WIDTH - 200.0, GAME_HEIGHT - 200.0, 0xFFFAFD6D));
        prompt_bg.screen_center(Axes::XY);
        prompt_bg.scroll.set(0.0, 0.0);
        let mut prompt_text = Alphabet::atlas(ctx, 0.0, 100.0, "PRESS ANY KEY TO REBIND\n\n\nESCAPE TO CANCEL", Font::Bold);
        prompt_text.screen_center_x();
        prompt_text.y = (GAME_HEIGHT - prompt_text.height()) / 2.0;
        prompt_text.scroll.set(0.0, 0.0);

        let mut o = Options {
            bg,
            page: Page::Options,
            enabled: false,
            busy: false,
            flicker: Flicker::default(),
            flicker_target: (Page::Options, 0),
            option_items,
            option_selected: 0,
            pref_items,
            pref_values,
            pref_selected: 0,
            pref_cam,
            control_headers,
            control_labels,
            bindings,
            binding_selected: 0,
            control_cam,
            prompt_bg,
            prompt_text,
            prompt_open: false,
        };
        o.refresh_options();
        o.refresh_prefs();
        o.refresh_bindings(ctx);
        o
    }

    fn refresh_options(&mut self) {
        for (i, item) in self.option_items.iter_mut().enumerate() {
            item.set_alpha(if i == self.option_selected { 1.0 } else { 0.6 });
        }
    }

    fn refresh_prefs(&mut self) {
        for (i, item) in self.pref_items.iter_mut().enumerate() {
            item.set_alpha(if i == self.pref_selected { 1.0 } else { 0.6 });
        }
    }

    fn refresh_bindings(&mut self, ctx: &mut Ctx) {
        for (i, b) in self.bindings.iter_mut().enumerate() {
            b.text.set_alpha(if i == self.binding_selected { 1.0 } else { 0.6 });
        }
        let row = self.binding_selected / 2;
        for (i, l) in self.control_labels.iter_mut().enumerate() {
            l.set_alpha(if i == row { 1.0 } else { 0.6 });
        }
        let y = self.bindings[self.binding_selected].text.y;
        ctx.camera(self.control_cam).set_target_position(GAME_WIDTH / 2.0, y);
    }

    fn set_page(&mut self, ctx: &mut Ctx, page: Page) {
        self.page = page;
        ctx.camera(self.pref_cam).visible = page == Page::Preferences;
        ctx.camera(self.control_cam).visible = page == Page::Controls;
    }

    fn accept(&mut self, ctx: &mut Ctx, page: Page, index: usize) {
        self.busy = true;
        library::play_sound(ctx, "confirmMenu", 1.0);
        self.flicker = Flicker::start(1.0, 0.06, true);
        self.flicker_target = (page, index);
    }

    fn fire(&mut self, ctx: &mut Ctx, page: Page, index: usize) {
        match page {
            Page::Options => match index {
                0 => self.set_page(ctx, Page::Preferences),
                1 => self.set_page(ctx, Page::Controls),
                _ => {
                    self.enabled = false;
                    ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
                }
            },
            Page::Preferences => self.toggle_pref(ctx, index),
            Page::Controls => self.prompt_open = true,
        }
    }

    fn toggle_pref(&mut self, ctx: &mut Ctx, index: usize) {
        let pref = PREFS[index].1;
        let prefs = &mut ctx.data.prefs;
        let flag = match pref {
            Pref::Downscroll => &mut prefs.downscroll,
            Pref::FlashingMenu => &mut prefs.flashing_menu,
            Pref::CameraZoom => &mut prefs.camera_zoom,
            Pref::FpsCounter => &mut prefs.fps_counter,
            Pref::AutoPause => &mut prefs.auto_pause,
        };
        *flag = !*flag;
        let value = *flag;
        self.pref_values[index].set_text(on_off(value));
        ctx.show_fps = ctx.data.prefs.fps_counter;
        ctx.data.prefs.store(&mut ctx.save);
    }

    fn rebind(&mut self, ctx: &mut Ctx, key: KeyCode) {
        let selected = self.binding_selected;
        let row_start = (selected / 2) * 2;
        if (0..2).any(|i| self.bindings[row_start + i].key == Some(key)) {
            return;
        }
        let control = self.bindings[selected].control;
        let old = self.bindings[selected].key;
        let groups: [&[Control]; 2] = [
            &[Control::NoteUp, Control::NoteDown, Control::NoteLeft, Control::NoteRight],
            &[Control::UiUp, Control::UiDown, Control::UiLeft, Control::UiRight, Control::Accept, Control::Back],
        ];
        for group in groups {
            if !group.contains(&control) {
                continue;
            }
            for i in 0..self.bindings.len() {
                if i == selected || !group.contains(&self.bindings[i].control) || self.bindings[i].key != Some(key) {
                    continue;
                }
                let other = self.bindings[i].control;
                if let Some(old_key) = old {
                    ctx.data.controls.replace(other, Some(key), old_key);
                }
                self.bindings[i].key = old;
                let label = old.map(key_name).unwrap_or("---");
                self.bindings[i].text.set_text(label);
            }
        }
        ctx.data.controls.replace(control, old, key);
        self.bindings[selected].key = Some(key);
        self.bindings[selected].text.set_text(key_name(key));
        ctx.data.controls.store(&mut ctx.save);
    }

    fn nav(ctx: &Ctx, index: usize, len: usize) -> Option<usize> {
        let up = ctx.just_pressed(Control::UiUp);
        let down = ctx.just_pressed(Control::UiDown);
        if up == down {
            return None;
        }
        Some(if up { (index + len - 1) % len } else { (index + 1) % len })
    }
}

impl State for Options {
    fn trans_in_done(&mut self, _ctx: &mut Ctx) {
        self.enabled = true;
    }

    fn update(&mut self, ctx: &mut Ctx) {
        if self.busy {
            let (page, index) = self.flicker_target;
            let item = match page {
                Page::Options => &mut self.option_items[index],
                Page::Preferences => &mut self.pref_items[index],
                Page::Controls => &mut self.bindings[index].text,
            };
            let mut visible = item.visible;
            let done = self.flicker.update(ctx.elapsed, &mut visible);
            item.visible = visible;
            if done {
                self.busy = false;
                self.fire(ctx, page, index);
            }
        }
        for (i, item) in self.pref_items.iter_mut().enumerate() {
            item.x = if i == self.pref_selected { 150.0 } else { 120.0 };
        }
        for (i, value) in self.pref_values.iter_mut().enumerate() {
            let item = &self.pref_items[i];
            value.x = item.x + item.width() + 30.0;
        }
        if !self.enabled || self.busy {
            return;
        }
        if self.prompt_open {
            if let Some(key) = ctx.keys.first_just_released() {
                if key != KeyCode::Escape {
                    self.rebind(ctx, key);
                }
                self.prompt_open = false;
            }
            return;
        }
        match self.page {
            Page::Options => {
                if let Some(next) = Self::nav(ctx, self.option_selected, self.option_items.len()) {
                    library::play_sound(ctx, "scrollMenu", 1.0);
                    self.option_selected = next;
                    self.refresh_options();
                }
                if ctx.just_pressed(Control::Accept) {
                    self.accept(ctx, Page::Options, self.option_selected);
                } else if ctx.just_pressed(Control::Back) {
                    library::play_sound(ctx, "cancelMenu", 1.0);
                    self.enabled = false;
                    ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
                }
            }
            Page::Preferences => {
                if let Some(next) = Self::nav(ctx, self.pref_selected, self.pref_items.len()) {
                    library::play_sound(ctx, "scrollMenu", 1.0);
                    self.pref_selected = next;
                    self.refresh_prefs();
                    let y = self.pref_items[next].y;
                    ctx.camera(self.pref_cam).set_target_position(GAME_WIDTH / 2.0, y);
                }
                if ctx.just_pressed(Control::Accept) {
                    self.accept(ctx, Page::Preferences, self.pref_selected);
                } else if ctx.just_pressed(Control::Back) {
                    library::play_sound(ctx, "cancelMenu", 1.0);
                    self.set_page(ctx, Page::Options);
                }
            }
            Page::Controls => {
                let rows = self.bindings.len() / 2;
                let row = self.binding_selected / 2;
                let mut column = self.binding_selected % 2;
                let mut new_row = row;
                if let Some(next) = Self::nav(ctx, row, rows) {
                    new_row = next;
                }
                if ctx.just_pressed(Control::UiLeft) && column > 0 {
                    column -= 1;
                }
                if ctx.just_pressed(Control::UiRight) && column < 1 {
                    column += 1;
                }
                let next = new_row * 2 + column;
                if next != self.binding_selected {
                    library::play_sound(ctx, "scrollMenu", 1.0);
                    self.binding_selected = next;
                    self.refresh_bindings(ctx);
                }
                if ctx.just_pressed(Control::Accept) {
                    self.accept(ctx, Page::Controls, self.binding_selected);
                } else if ctx.just_pressed(Control::Back) {
                    library::play_sound(ctx, "cancelMenu", 1.0);
                    self.set_page(ctx, Page::Options);
                }
            }
        }
    }

    fn draw(&mut self, _ctx: &mut Ctx, dl: &mut DrawList) {
        self.bg.draw(dl, 0);
        match self.page {
            Page::Options => {
                for item in self.option_items.iter_mut() {
                    item.draw(dl, 0);
                }
            }
            Page::Preferences => {
                let cam = self.pref_cam;
                for item in self.pref_items.iter_mut() {
                    item.draw(dl, cam);
                }
                for v in self.pref_values.iter_mut() {
                    v.draw(dl, cam);
                }
            }
            Page::Controls => {
                let cam = self.control_cam;
                for h in self.control_headers.iter_mut() {
                    h.draw(dl, cam);
                }
                for l in self.control_labels.iter_mut() {
                    l.draw(dl, cam);
                }
                for b in self.bindings.iter_mut() {
                    b.text.draw(dl, cam);
                }
                if self.prompt_open {
                    self.prompt_bg.draw(dl, cam);
                    self.prompt_text.draw(dl, cam);
                }
            }
        }
    }
}