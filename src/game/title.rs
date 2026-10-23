use miniquad::{window, KeyCode};

use crate::engine::app::{Ctx, State, TransData};
use crate::engine::gfx::{DrawList, Effect, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::sprite::{Axes, Sprite};
use crate::engine::timer::Timer;

use super::conductor::BeatTracker;
use super::data::{Control, Input};
use super::library;
use super::alphabet::Alphabet;

pub struct Title {
    started: bool,
    start_timer: Timer,
    bg: Option<Sprite>,
    logo: Option<Sprite>,
    dancer: Option<Sprite>,
    enter: Option<Sprite>,
    badge: Option<Sprite>,
    black: Option<Sprite>,
    texts: Vec<Alphabet>,
    show_credits: bool,
    skipped_intro: bool,
    transitioning: bool,
    dance_left: bool,
    last_beat: i64,
    hue: f64,
    wacky: Vec<String>,
    beat: BeatTracker,
    fullscreen: bool,
}

impl Title {
    pub fn new(ctx: &mut Ctx) -> Title {
        let lines = ctx.assets.read_text("data/introText.txt").unwrap_or_default();
        let options: Vec<Vec<String>> = lines.lines().filter(|l| !l.trim().is_empty()).map(|l| l.split("--").map(|s| s.trim().to_string()).collect()).collect();
        let wacky = ctx.rng.pick(&options).cloned().unwrap_or_else(|| vec!["hello".into(), "there".into()]);
        Title {
            started: false,
            start_timer: Timer::started(1.0, 1),
            bg: None,
            logo: None,
            dancer: None,
            enter: None,
            badge: None,
            black: None,
            texts: Vec::new(),
            show_credits: true,
            skipped_intro: false,
            transitioning: false,
            dance_left: false,
            last_beat: 0,
            hue: 0.0,
            wacky,
            beat: BeatTracker::default(),
            fullscreen: false,
        }
    }

    fn start_intro(&mut self, ctx: &mut Ctx) {
        if !ctx.data.title_initialized {
            ctx.default_trans_in = Some(TransData { color: 0xFF000000, duration: 1.0, dir_y: -1.0 });
            ctx.default_trans_out = Some(TransData { color: 0xFF000000, duration: 0.7, dir_y: 1.0 });
        }
        if !ctx.audio.music.playing() {
            library::play_music(ctx, "freakyMenu", 0.0, true);
            ctx.audio.fade_in_music(4.0, 0.0, 0.7);
        }
        ctx.data.conductor.change_bpm(102.0);
        ctx.data.conductor.bpm_change_map.clear();
        self.bg = Some(Sprite::with_frames(0.0, 0.0, ctx.solid(GAME_WIDTH, GAME_HEIGHT, 0xFF000000)));
        let mut logo = Sprite::with_frames(-150.0, -100.0, ctx.atlas("logoBumpin"));
        logo.antialiasing = true;
        logo.add_by_prefix("bump", "logo bumpin", 24.0, true);
        logo.play("bump", false);
        logo.update_hitbox();
        let mut dancer = Sprite::with_frames(GAME_WIDTH * 0.4, GAME_HEIGHT * 0.07, ctx.atlas("gfDanceTitle"));
        let left: Vec<i64> = std::iter::once(30).chain(0..15).collect();
        let right: Vec<i64> = (15..30).collect();
        dancer.add_by_indices("danceLeft", "gfDance", &left, 24.0, false);
        dancer.add_by_indices("danceRight", "gfDance", &right, 24.0, false);
        dancer.antialiasing = true;
        let mut enter = Sprite::with_frames(100.0, GAME_HEIGHT * 0.8, ctx.atlas("titleEnter"));
        enter.add_by_prefix("idle", "Press Enter to Begin", 24.0, true);
        enter.add_by_prefix("press", "ENTER PRESSED", 24.0, true);
        enter.antialiasing = true;
        enter.play("idle", false);
        enter.update_hitbox();
        let mut badge = Sprite::with_frames(0.0, GAME_HEIGHT * 0.52, ctx.image("newgrounds_logo"));
        badge.visible = false;
        let w = badge.width;
        badge.set_graphic_size((w * 0.8) as i64, 0);
        badge.update_hitbox();
        badge.screen_center(Axes::X);
        badge.antialiasing = true;
        self.logo = Some(logo);
        self.dancer = Some(dancer);
        self.enter = Some(enter);
        self.badge = Some(badge);
        self.black = Some(Sprite::with_frames(0.0, 0.0, ctx.solid(GAME_WIDTH, GAME_HEIGHT, 0xFF000000)));
        if ctx.data.title_initialized {
            self.skip_intro(ctx);
        } else {
            ctx.data.title_initialized = true;
        }
        self.started = true;
    }

    fn create_cool_text(&mut self, ctx: &mut Ctx, lines: &[&str]) {
        for (i, line) in lines.iter().enumerate() {
            let mut t = Alphabet::new(ctx, 0.0, 0.0, line, true);
            t.screen_center_x();
            t.y += (i as f64 * 60.0) + 200.0;
            self.texts.push(t);
        }
    }

    fn add_more_text(&mut self, ctx: &mut Ctx, line: &str) {
        let mut t = Alphabet::new(ctx, 0.0, 0.0, line, true);
        t.screen_center_x();
        t.y += (self.texts.len() as f64 * 60.0) + 200.0;
        self.texts.push(t);
    }

    fn skip_intro(&mut self, ctx: &mut Ctx) {
        if !self.skipped_intro {
            if let Some(b) = self.badge.as_mut() {
                b.visible = false;
            }
            ctx.camera(0).flash(0xFFFFFFFF, 4.0, false);
            self.show_credits = false;
            self.skipped_intro = true;
        }
    }

    fn beat_hit(&mut self, ctx: &mut Ctx) {
        if !self.started {
            return;
        }
        if self.skipped_intro {
            if let Some(logo) = self.logo.as_mut() {
                logo.play("bump", true);
            }
            self.dance_left = !self.dance_left;
            if let Some(d) = self.dancer.as_mut() {
                d.play(if self.dance_left { "danceRight" } else { "danceLeft" }, false);
            }
            return;
        }
        let cur = self.beat.cur_beat;
        if cur > self.last_beat {
            for i in self.last_beat..cur {
                match i + 1 {
                    1 => self.create_cool_text(ctx, &["ninjamuffin99", "phantomArcade", "kawaisprite", "evilsk8er"]),
                    3 => self.add_more_text(ctx, "present"),
                    4 => self.texts.clear(),
                    5 => self.create_cool_text(ctx, &["In association", "with"]),
                    7 => {
                        self.add_more_text(ctx, "newgrounds");
                        if let Some(b) = self.badge.as_mut() {
                            b.visible = true;
                        }
                    }
                    8 => {
                        self.texts.clear();
                        if let Some(b) = self.badge.as_mut() {
                            b.visible = false;
                        }
                    }
                    9 => {
                        let first = self.wacky.first().cloned().unwrap_or_default();
                        self.create_cool_text(ctx, &[&first]);
                    }
                    11 => {
                        let second = self.wacky.get(1).cloned().unwrap_or_default();
                        self.add_more_text(ctx, &second);
                    }
                    12 => self.texts.clear(),
                    13 => self.add_more_text(ctx, "Friday"),
                    14 => self.add_more_text(ctx, "Night"),
                    15 => self.add_more_text(ctx, "Funkin"),
                    16 => self.skip_intro(ctx),
                    _ => {}
                }
            }
        }
        self.last_beat = cur;
    }
}

impl State for Title {
    fn persistent_update(&self) -> bool {
        true
    }

    fn update(&mut self, ctx: &mut Ctx) {
        if self.start_timer.update(ctx.elapsed) {
            self.start_intro(ctx);
        }
        ctx.data.conductor.song_position = ctx.audio.music.time();
        if ctx.keys.just_pressed(KeyCode::F) {
            self.fullscreen = !self.fullscreen;
            window::set_fullscreen(self.fullscreen);
        }
        let pressed_enter = ctx.keys.just_pressed(KeyCode::Enter) || ctx.keys.just_pressed(KeyCode::KpEnter);
        if pressed_enter && !self.transitioning && self.skipped_intro {
            if let Some(e) = self.enter.as_mut() {
                e.play("press", false);
            }
            ctx.camera(0).flash(0xFFFFFFFF, 1.0, false);
            library::play_sound(ctx, "confirmMenu", 0.7);
            self.transitioning = true;
            ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
        }
        if pressed_enter && !self.skipped_intro && ctx.data.title_initialized {
            self.skip_intro(ctx);
        }
        if ctx.pressed(Control::UiLeft) {
            self.hue -= ctx.elapsed * 0.1;
        }
        if ctx.pressed(Control::UiRight) {
            self.hue += ctx.elapsed * 0.1;
        }
        if let Some(event) = self.beat.update(&ctx.data.conductor, true) {
            if event.beat.is_some() {
                self.beat_hit(ctx);
            }
        }
        let effect = if self.hue != 0.0 { Effect::Hue(self.hue as f32) } else { Effect::None };
        if let Some(logo) = self.logo.as_mut() {
            logo.effect = effect;
            logo.update(ctx.elapsed);
        }
        if let Some(d) = self.dancer.as_mut() {
            d.effect = effect;
            d.update(ctx.elapsed);
        }
        if let Some(e) = self.enter.as_mut() {
            e.update(ctx.elapsed);
        }
        for t in self.texts.iter_mut() {
            t.update(ctx.elapsed);
        }
    }

    fn draw(&mut self, _ctx: &mut Ctx, dl: &mut DrawList) {
        if let Some(bg) = &self.bg {
            bg.draw(dl, 0);
        }
        if let Some(d) = &self.dancer {
            d.draw(dl, 0);
        }
        if let Some(logo) = &self.logo {
            logo.draw(dl, 0);
        }
        if let Some(e) = &self.enter {
            e.draw(dl, 0);
        }
        if self.show_credits {
            if let Some(black) = &self.black {
                black.draw(dl, 0);
            }
            for t in self.texts.iter_mut() {
                t.draw(dl, 0);
            }
        }
        if let Some(b) = &self.badge {
            b.draw(dl, 0);
        }
    }
}