use std::rc::Rc;

use miniquad::{window, EventHandler, KeyCode, KeyMods, MouseButton};

use super::assets::Assets;
use super::atlas::Frames;
use super::audio::Audio;
use super::camera::Camera;
use super::gfx::{Affine, DrawList, Gfx, View, GAME_HEIGHT, GAME_WIDTH};
use super::input::{Keys, Mouse};
use super::math::Rng;
use super::save::Save;
use super::sprite::Sprite;
use super::text::{collect_garbage, Align, Text};
use super::tween::{Prop, Tween};
use crate::game::GameData;

pub type Factory = Box<dyn FnOnce(&mut Ctx) -> Box<dyn State>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransData {
    pub color: u32,
    pub duration: f64,
    pub dir_y: f64,
}

pub trait State {
    fn update(&mut self, ctx: &mut Ctx);
    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList);
    fn trans_in_done(&mut self, _ctx: &mut Ctx) {}
}

pub struct Ctx {
    pub gfx: Gfx,
    pub assets: Assets,
    pub audio: Audio,
    pub keys: Keys,
    pub mouse: Mouse,
    pub cameras: Vec<Camera>,
    pub rng: Rng,
    pub save: Save,
    pub elapsed: f64,
    pub ticks: f64,
    pub data: GameData,
    pub default_trans_in: Option<TransData>,
    pub default_trans_out: Option<TransData>,
    pub trans_in: Option<TransData>,
    pub trans_out: Option<TransData>,
    pub skip_next_trans_in: bool,
    pub skip_next_trans_out: bool,
    pub show_fps: bool,
    next: Option<Factory>,
    exiting: bool,
}

impl Ctx {
    pub fn switch_state(&mut self, factory: impl FnOnce(&mut Ctx) -> Box<dyn State> + 'static) {
        if self.exiting {
            return;
        }
        self.next = Some(Box::new(factory));
    }

    pub fn is_exiting(&self) -> bool {
        self.exiting
    }

    pub fn camera(&mut self, index: usize) -> &mut Camera {
        &mut self.cameras[index]
    }

    pub fn add_camera(&mut self, camera: Camera) -> usize {
        self.cameras.push(camera);
        self.cameras.len() - 1
    }

    pub fn solid(&mut self, width: f64, height: f64, color: u32) -> Rc<Frames> {
        self.assets.solid(&mut self.gfx, width, height, color)
    }

    pub fn image(&mut self, key: &str) -> Rc<Frames> {
        self.assets.image(&mut self.gfx, key)
    }

    pub fn atlas(&mut self, key: &str) -> Rc<Frames> {
        self.assets.atlas(&mut self.gfx, key)
    }

    pub fn grid(&mut self, key: &str, w: u32, h: u32) -> Rc<Frames> {
        self.assets.grid(&mut self.gfx, key, w, h)
    }
}

const QUICK_TRANSITION_SCALE: f64 = 0.4;

struct Transition {
    sprite: Sprite,
    going_out: bool,
    delay_frame: bool,
}

impl Transition {
    fn new(ctx: &mut Ctx, data: TransData, going_out: bool) -> Transition {
        let region = [-200.0, -200.0, GAME_WIDTH * 1.4, GAME_HEIGHT * 1.4];
        let height = (region[3] * 2.0) as usize;
        let half = region[3] as usize;
        let mut pixels = Vec::with_capacity(height * 4);
        let channel = |shift: u32| (data.color >> shift) & 0xFF;
        for y in 0..height {
            let in_gradient = if data.dir_y > 0.0 { y >= half } else { y < half };
            let alpha = if in_gradient {
                let local = if data.dir_y > 0.0 { y - half } else { y };
                let t = (local as f64 + 0.5) / half as f64;
                if data.dir_y > 0.0 { 1.0 - t } else { t }
            } else {
                1.0
            };
            let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
            for shift in [16, 8, 0] {
                pixels.push(((channel(shift) * a + 127) / 255) as u8);
            }
            pixels.push(a as u8);
        }
        let texture = ctx.gfx.create_texture(1, height as u32, &pixels);
        let mut sprite = Sprite::with_frames(region[0], region[1], Rc::new(Frames::single(texture)));
        sprite.scroll.set(0.0, 0.0);
        sprite.scale.set(region[2], 1.0);
        sprite.update_hitbox();
        sprite.antialiasing = false;
        let back_height = sprite.height;
        let (start, end) = if data.dir_y > 0.0 {
            if going_out { (-back_height, 0.0) } else { (0.0, -back_height) }
        } else if going_out {
            (GAME_HEIGHT, -back_height / 2.0)
        } else {
            (-back_height / 2.0, GAME_HEIGHT)
        };
        let duration = if ctx.data.prefs.quick_transitions { data.duration * QUICK_TRANSITION_SCALE } else { data.duration };
        sprite.y = start;
        sprite.tween(Prop::Y, Tween::new(end, duration));
        Transition { sprite, going_out, delay_frame: false }
    }

    fn update(&mut self, elapsed: f64) -> bool {
        if self.delay_frame {
            return true;
        }
        self.sprite.update_tweens(elapsed);
        if !self.sprite.tweens.is_active() {
            self.delay_frame = true;
        }
        false
    }
}

struct SoundTray {
    y: f64,
    timer: f64,
    visible: bool,
    label: Option<Text>,
}

impl SoundTray {
    fn show(&mut self, ctx: &mut Ctx) {
        self.timer = 1.0;
        self.y = 0.0;
        self.visible = true;
        ctx.audio.play_beep();
    }

    fn update(&mut self, elapsed: f64) {
        if self.timer > 0.0 {
            self.timer -= elapsed;
        } else if self.y > -60.0 {
            self.y -= elapsed * GAME_HEIGHT * 2.0;
            if self.y <= -60.0 {
                self.visible = false;
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList, slot: usize) {
        if !self.visible {
            return;
        }
        let scale = 2.0;
        let x = ((dl.view.screen_width - 80.0 * scale) * 0.5).floor();
        let y = self.y;
        let mut bg = Sprite::with_frames(x, y, ctx.solid(80.0 * scale, 30.0 * scale, 0x7F000000));
        bg.scroll.set(0.0, 0.0);
        bg.draw(dl, slot);
        let level = if ctx.audio.muted { 0 } else { (ctx.audio.volume * 10.0).round() as i32 };
        for i in 0..10 {
            let mut bar = Sprite::with_frames(
                x + (10.0 + 6.0 * i as f64) * scale,
                y + (14.0 - i as f64) * scale,
                ctx.solid(4.0 * scale, (i as f64 + 1.0) * scale, 0xFFFFFFFF),
            );
            bar.scroll.set(0.0, 0.0);
            bar.set_alpha(if i < level { 1.0 } else { 0.5 });
            bar.draw(dl, slot);
        }
        if self.label.is_none() {
            let font = ctx.data.fonts.ui.clone();
            let mut t = Text::new(0.0, 0.0, 80.0 * scale, "VOLUME", 10.0 * scale, font);
            t.set_align(Align::Center);
            self.label = Some(t);
        }
        if let Some(label) = self.label.as_mut() {
            label.sprite.x = x;
            label.sprite.y = y + 16.0 * scale;
            label.sprite.scroll.set(0.0, 0.0);
            label.draw(&mut ctx.gfx, dl, slot);
        }
    }
}

struct FpsCounter {
    times: Vec<f64>,
    cache_count: usize,
    current: usize,
    text: Option<Text>,
}

impl FpsCounter {
    fn update(&mut self, now_ms: f64) {
        self.times.push(now_ms);
        self.times.retain(|t| *t >= now_ms - 1000.0);
        let count = self.times.len();
        self.current = ((count + self.cache_count) as f64 / 2.0).round() as usize;
        self.cache_count = count;
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList, slot: usize) {
        if self.text.is_none() {
            let font = ctx.assets.fallback_font();
            self.text = Some(Text::new(10.0, 3.0, 0.0, "FPS: ", 12.0, font));
        }
        let label = format!("FPS: {}", self.current);
        if let Some(t) = self.text.as_mut() {
            t.set_text(&label);
            t.sprite.scroll.set(0.0, 0.0);
            t.draw(&mut ctx.gfx, dl, slot);
        }
    }
}

#[cfg(windows)]
fn screen_size() -> Option<(f32, f32)> {
    unsafe extern "system" {
        fn GetSystemMetrics(index: i32) -> i32;
    }
    let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
    Some((w as f32, h as f32))
}

#[cfg(not(windows))]
fn screen_size() -> Option<(f32, f32)> {
    None
}

fn center_window() {
    let Some((sw, sh)) = screen_size() else { return };
    let (w, h) = window::screen_size();
    window::set_window_position(((sw - w) / 2.0).max(0.0) as u32, ((sh - h) / 2.0).max(0.0) as u32);
}

#[cfg(windows)]
fn dark_title_bar(title: &str) {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(class: *const u16, name: *const u16) -> *mut c_void;
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(hwnd: *mut c_void, attribute: u32, value: *const c_void, size: u32) -> i32;
    }

    let name: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let dark: u32 = 1;
    let caption: u32 = 0x00241A18;
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), name.as_ptr());
        if !hwnd.is_null() {
            for attribute in [19, 20] {
                DwmSetWindowAttribute(hwnd, attribute, &dark as *const u32 as *const c_void, 4);
            }
            DwmSetWindowAttribute(hwnd, 35, &caption as *const u32 as *const c_void, 4);
        }
    }
}

#[cfg(not(windows))]
fn dark_title_bar(_title: &str) {}

pub struct App {
    ctx: Ctx,
    state: Option<Box<dyn State>>,
    transition: Option<Transition>,
    pending: Option<Factory>,
    first_frame: bool,
    centered: bool,
    last_time: f64,
    tray: SoundTray,
    fps: FpsCounter,
    fullscreen: bool,
}

impl App {
    pub fn new(assets: Assets, data: impl FnOnce(&mut Assets) -> GameData, first: Factory) -> App {
        let gfx = Gfx::new();
        let audio = Audio::new();
        let mut assets = assets;
        let data = data(&mut assets);
        let save = Save::open("funkin-rs");
        let mut ctx = Ctx {
            gfx,
            assets,
            audio,
            keys: Keys::default(),
            mouse: Mouse::default(),
            cameras: vec![Camera::new()],
            rng: Rng::from_time(),
            save,
            elapsed: 0.0,
            ticks: 0.0,
            data,
            default_trans_in: None,
            default_trans_out: None,
            trans_in: None,
            trans_out: None,
            skip_next_trans_in: false,
            skip_next_trans_out: false,
            show_fps: true,
            next: None,
            exiting: false,
        };
        if let Some(v) = ctx.save.get("volume").and_then(|v| v.as_f64()) {
            ctx.audio.volume = v.clamp(0.0, 1.0);
        }
        if let Some(m) = ctx.save.get("mute").and_then(|v| v.as_bool()) {
            ctx.audio.muted = m;
        }
        let mut app = App {
            ctx,
            state: None,
            transition: None,
            pending: Some(first),
            first_frame: true,
            centered: false,
            last_time: miniquad::date::now(),
            tray: SoundTray { y: -60.0, timer: 0.0, visible: false, label: None },
            fps: FpsCounter { times: Vec::new(), cache_count: 0, current: 0, text: None },
            fullscreen: false,
        };
        app.switch_now();
        app
    }

    fn switch_now(&mut self) {
        let Some(factory) = self.pending.take() else { return };
        self.state = None;
        self.transition = None;
        self.ctx.cameras = vec![Camera::new()];
        self.ctx.keys.reset();
        self.ctx.audio.stop_one_shots();
        self.ctx.exiting = false;
        self.ctx.trans_in = self.ctx.default_trans_in;
        self.ctx.trans_out = self.ctx.default_trans_out;
        let mut state = factory(&mut self.ctx);
        self.first_frame = true;
        match self.ctx.trans_in {
            Some(data) if !self.ctx.skip_next_trans_in => {
                self.transition = Some(Transition::new(&mut self.ctx, data, false));
            }
            Some(_) => {
                self.ctx.skip_next_trans_in = false;
                state.trans_in_done(&mut self.ctx);
            }
            None => {}
        }
        self.state = Some(state);
    }

    fn handle_requests(&mut self) {
        let Some(next) = self.ctx.next.take() else { return };
        match self.ctx.trans_out {
            Some(data) => {
                self.ctx.exiting = true;
                self.pending = Some(next);
                if self.ctx.skip_next_trans_out {
                    self.ctx.skip_next_trans_out = false;
                    self.switch_now();
                } else {
                    self.transition = Some(Transition::new(&mut self.ctx, data, true));
                }
            }
            None => {
                self.pending = Some(next);
                self.switch_now();
            }
        }
    }

    fn volume_keys(&mut self) {
        let k = &self.ctx.keys;
        let mute = k.just_released(KeyCode::Key0) || k.just_released(KeyCode::Kp0);
        let up = k.just_released(KeyCode::Equal) || k.just_released(KeyCode::KpAdd);
        let down = k.just_released(KeyCode::Minus) || k.just_released(KeyCode::KpSubtract);
        if mute {
            self.ctx.audio.toggle_mute();
        } else if up {
            self.ctx.audio.change_volume(0.1);
        } else if down {
            self.ctx.audio.change_volume(-0.1);
        } else {
            return;
        }
        self.tray.show(&mut self.ctx);
        self.ctx.save.set("volume", serde_json::json!(self.ctx.audio.volume));
        self.ctx.save.set("mute", serde_json::json!(self.ctx.audio.muted));
        self.ctx.save.flush();
    }
}

impl EventHandler for App {
    fn update(&mut self) {
        if !self.centered {
            self.centered = true;
            center_window();
            dark_title_bar("Friday Night Funkin'");
        }
        let now = miniquad::date::now();
        let elapsed = (now - self.last_time).clamp(0.0, 0.1);
        self.last_time = now;
        self.ctx.elapsed = elapsed;
        self.ctx.ticks += elapsed * 1000.0;
        self.fps.update(now * 1000.0);
        self.ctx.keys.update();
        let (w, h) = window::screen_size();
        self.ctx.mouse.update(&View::fit(w as f64, h as f64));
        if self.ctx.keys.just_pressed(KeyCode::F11) {
            self.fullscreen = !self.fullscreen;
            window::set_fullscreen(self.fullscreen);
        }
        self.volume_keys();
        self.ctx.audio.update(elapsed);
        self.tray.update(elapsed);

        let mut finished_in = false;
        let mut finished_out = false;
        if let Some(t) = self.transition.as_mut() {
            if t.update(elapsed) {
                if t.going_out {
                    finished_out = true;
                } else {
                    finished_in = true;
                }
            }
        }
        if let Some(mut state) = self.state.take() {
            self.first_frame = false;
            state.update(&mut self.ctx);
            if finished_in {
                self.transition = None;
                state.trans_in_done(&mut self.ctx);
            }
            self.state = Some(state);
        }
        let mut rng = std::mem::replace(&mut self.ctx.rng, Rng::new(0));
        for cam in self.ctx.cameras.iter_mut() {
            cam.update(elapsed, &mut rng);
        }
        self.ctx.rng = rng;
        if finished_out {
            self.switch_now();
        } else {
            self.handle_requests();
        }
    }

    fn draw(&mut self) {
        collect_garbage(&mut self.ctx.gfx);
        let (w, h) = window::screen_size();
        let view = View::fit(w as f64, h as f64);
        let mut dl = DrawList::new(view);
        for (i, cam) in self.ctx.cameras.iter().enumerate() {
            cam.push_layer(i, &mut dl);
        }
        if let Some(state) = self.state.as_mut() {
            state.draw(&mut self.ctx, &mut dl);
        }
        let overlay = self.ctx.cameras.len();
        let top = overlay + 1;
        if let Some(t) = self.transition.as_ref() {
            Camera::transparent().push_layer(overlay, &mut dl);
            t.sprite.draw(&mut dl, overlay);
        }
        dl.push_layer(top, Affine::IDENTITY, [0.0, 0.0], [-view.offset_x / view.scale, -view.offset_y / view.scale, w as f64 / view.scale, h as f64 / view.scale], None, 1.0);
        self.tray.draw(&mut self.ctx, &mut dl, top);
        if self.ctx.show_fps {
            self.fps.draw(&mut self.ctx, &mut dl, top);
        }
        self.ctx.gfx.render(&dl);
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, repeat: bool) {
        if repeat {
            self.ctx.keys.repeat_event(keycode);
        } else {
            self.ctx.keys.key_event(keycode, true);
        }
    }

    fn char_event(&mut self, character: char, _mods: KeyMods, _repeat: bool) {
        self.ctx.keys.char_event(character);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.ctx.mouse.raw_x = x as f64;
        self.ctx.mouse.raw_y = y as f64;
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.ctx.mouse.wheel_event(y as f64);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        self.ctx.mouse.button_event(button, true);
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        self.ctx.mouse.button_event(button, false);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        self.ctx.keys.key_event(keycode, false);
    }

    fn quit_requested_event(&mut self) {
        self.ctx.save.flush();
    }
}