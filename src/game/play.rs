use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use crate::engine::app::{Ctx, State};
use crate::engine::audio::Sound;
use crate::engine::camera::{Camera, CameraEvent, FollowStyle, Target};
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::math::{decay, remap, round_decimal, Point};
use crate::engine::sprite::{Axes, Sprite};
use crate::engine::text::{Align, Border, Text};
use crate::engine::timer::Timer;
use crate::engine::tween::{Ease, Prop, Tween};

use super::character::Character;
use super::conductor::BeatTracker;
use super::data::{Control, Input};
use super::dialogue::DialogueBox;
use super::game_over::GameOver;
use super::gitaroo::GitarooPause;
use super::library;
use super::note::{strum, Note, SWAG_WIDTH};
use super::pause::{PauseAction, PauseMenu};
use super::song::{ChartEvent, Side, Song};
use super::stage::{Kind, Stage, PIXEL_ZOOM};
use super::widgets::{HealthBar, HealthIcon};

const CAM_GAME: usize = 0;
const CAM_HUD: usize = 1;
const TAG_DESTROY: u32 = 1;
const COLORS: [&str; 4] = ["LEFT", "DOWN", "UP", "RIGHT"];
const WHITE: u32 = 0xFFFFFFFF;

enum Sub {
    None,
    Pause(PauseMenu),
    GameOver(GameOver),
}

#[derive(PartialEq)]
enum Cutscene {
    None,
    WinterDark,
    WinterLights,
    SchoolFade,
    SenpaiRise,
    SenpaiDies,
    WhiteIn,
    Dialogue,
    LightsOff,
}

pub struct Resume {
    pub time: f64,
    pub health: f64,
    pub score: i64,
    pub combo: i64,
    pub cam_follow: Point,
}

struct HoldSpan {
    top: f64,
    top_index: usize,
    bottom: f64,
    bottom_index: usize,
}

struct SongClock {
    last_audio: f64,
    anchor: f64,
}

pub struct PlayState {
    song: Rc<Song>,
    stage: Stage,
    pixel: bool,
    gf: Character,
    dad: Character,
    bf: Character,
    strum_line_y: f64,
    strums: Vec<Sprite>,
    notes: Vec<Note>,
    alive: Vec<usize>,
    unspawn: VecDeque<usize>,
    popups: Vec<Sprite>,
    health_bar_bg: Sprite,
    health_bar: HealthBar,
    icon_p1: HealthIcon,
    icon_p2: HealthIcon,
    score_txt: Text,
    cam_pos: Point,
    cam_follow: Point,
    vocals: Sound,
    vocals_finished: bool,
    beat: BeatTracker,
    generated_music: bool,
    starting_song: bool,
    started_countdown: bool,
    paused: bool,
    can_pause: bool,
    cam_zooming: bool,
    in_cutscene: bool,
    camera_right_side: bool,
    persistent_update: bool,
    persistent_draw: bool,
    health: f64,
    combo: i64,
    song_score: i64,
    gf_speed: i64,
    default_cam_zoom: f64,
    start_timer: Timer,
    start_time: f64,
    playtesting: bool,
    silent: bool,
    event_index: usize,
    swag_counter: usize,
    clock: SongClock,
    cam_zoom_tween: Option<Tween>,
    countdown_after_zoom: bool,
    cutscene: Cutscene,
    cut_timer: Timer,
    white_timer: Timer,
    cut_sound: Sound,
    black_alpha: f64,
    black: Option<Sprite>,
    red: Option<Sprite>,
    senpai_evil: Option<Sprite>,
    dialogue: Option<DialogueBox>,
    dialogue_shown: bool,
    sub: Sub,
    sub_fresh: bool,
    ended: bool,
    botplay: bool,
}

fn cover(ctx: &mut Ctx, x: f64, y: f64, width: f64, height: f64, color: u32) -> Sprite {
    let mut s = Sprite::with_frames(x, y, ctx.solid(width, height, color));
    s.scroll.set(0.0, 0.0);
    s
}

fn dialogue_lines(ctx: &mut Ctx, id: &str) -> Vec<String> {
    ctx.assets
        .read_text(&format!("data/{id}/{id}Dialogue.txt"))
        .map(|t| t.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default()
}

impl PlayState {
    pub fn new(ctx: &mut Ctx) -> PlayState {
        ctx.audio.music.stop();
        let mut cam_game = Camera::new();
        cam_game.bg_color = 0xFF000000;
        ctx.cameras = vec![cam_game, Camera::transparent()];

        let song = match ctx.data.session.song.clone() {
            Some(s) => s,
            None => library::chart(ctx, "tutorial", "normal").expect("tutorial chart missing"),
        };
        let playtesting = ctx.data.session.editor.is_some();
        let resume = ctx.data.session.resume.take();
        let start_time = match &resume {
            Some(r) => r.time,
            None if playtesting => ctx.data.session.playtest_start.max(0.0),
            None => 0.0,
        };
        ctx.data.conductor.map_bpm_changes(&song);
        ctx.data.conductor.change_bpm(song.bpm());
        ctx.data.conductor.offset = ctx.data.prefs.offset - song.offset;

        let kind = Kind::for_chart(song.stage.as_deref(), &song.id);
        let stage = Stage::new(ctx, kind, &song.id);
        let pixel = stage.kind.is_pixel();
        let default_cam_zoom = stage.zoom;

        let gf_name = song.girlfriend.clone().unwrap_or_else(|| kind.gf_version().to_string());
        let mut gf = Character::new(ctx, 400.0, 130.0, &gf_name, false);
        gf.sprite.scroll.set(0.95, 0.95);

        let mut dad = Character::new(ctx, 100.0, 100.0, &song.player2, false);
        let mut cam_pos = dad.sprite.graphic_midpoint();
        match song.player2.as_str() {
            "gf" => {
                dad.sprite.set_position(gf.sprite.x, gf.sprite.y);
                gf.sprite.visible = false;
                if ctx.data.session.is_story_mode {
                    cam_pos.x += 600.0;
                }
            }
            "spooky" => dad.sprite.y += 200.0,
            "monster" => dad.sprite.y += 100.0,
            "monster-christmas" => dad.sprite.y += 130.0,
            "dad" => cam_pos.x += 400.0,
            "pico" => {
                cam_pos.x += 600.0;
                dad.sprite.y += 300.0;
            }
            "parents-christmas" => dad.sprite.x -= 500.0,
            "senpai" | "senpai-angry" => {
                dad.sprite.x += 150.0;
                dad.sprite.y += 360.0;
                let mid = dad.sprite.graphic_midpoint();
                cam_pos = Point::new(mid.x + 300.0, mid.y);
            }
            "spirit" => {
                dad.sprite.x -= 150.0;
                dad.sprite.y += 100.0;
                let mid = dad.sprite.graphic_midpoint();
                cam_pos = Point::new(mid.x + 300.0, mid.y);
            }
            _ => {}
        }

        let mut bf = Character::new(ctx, 770.0, 450.0, &song.player1, true);
        let mut stage = stage;
        stage.reposition(&mut bf, &mut gf, &mut dad);
        stage.attach_trail(&dad.sprite);

        let dialogue = if matches!(song.id.as_str(), "senpai" | "roses" | "thorns") {
            let lines = dialogue_lines(ctx, &song.id);
            Some(DialogueBox::new(ctx, &song.id, lines))
        } else {
            None
        };

        ctx.data.conductor.song_position = -5000.0;
        let downscroll = ctx.data.prefs.downscroll;
        let strum_line_y = if downscroll { GAME_HEIGHT - 150.0 } else { 50.0 };

        let mut cam_follow = cam_pos;
        if let Some(prev) = resume.as_ref().map(|r| r.cam_follow).or(ctx.data.session.prev_cam_follow.take()) {
            cam_follow = prev;
        }
        {
            let cam = ctx.camera(CAM_GAME);
            cam.follow(Target { x: cam_follow.x, y: cam_follow.y, width: 1.0, height: 1.0 }, FollowStyle::LockOn, 0.04);
            cam.zoom = default_cam_zoom;
            cam.focus_on(cam_follow.x, cam_follow.y);
        }

        let bar_frames = ctx.image("healthBar");
        let mut health_bar_bg = Sprite::with_frames(0.0, GAME_HEIGHT * 0.9, bar_frames);
        health_bar_bg.screen_center(Axes::X);
        health_bar_bg.scroll.set(0.0, 0.0);
        if downscroll {
            health_bar_bg.y = GAME_HEIGHT * 0.1;
        }
        let health_bar = HealthBar::new(health_bar_bg.x + 4.0, health_bar_bg.y + 4.0, (health_bar_bg.width - 8.0).trunc(), (health_bar_bg.height - 8.0).trunc(), 0.0, 2.0);

        let mut score_txt = Text::new(health_bar_bg.x + health_bar_bg.width - 190.0, health_bar_bg.y + 30.0, 0.0, "", 20.0, ctx.data.fonts.ui.clone());
        score_txt.set_format(ctx.data.fonts.ui.clone(), 26.0, 0xFFFFFFFF, Align::Right, Border::Outline, 0xFF000000);
        score_txt.sprite.scroll.set(0.0, 0.0);

        let mut icon_p1 = HealthIcon::new(ctx, &song.player1, true);
        icon_p1.sprite.y = health_bar.y - icon_p1.sprite.height / 2.0;
        let mut icon_p2 = HealthIcon::new(ctx, &song.player2, false);
        icon_p2.sprite.y = health_bar.y - icon_p2.sprite.height / 2.0;

        let mut state = PlayState {
            song: song.clone(),
            stage,
            pixel,
            gf,
            dad,
            bf,
            strum_line_y,
            strums: Vec::new(),
            notes: Vec::new(),
            alive: Vec::new(),
            unspawn: VecDeque::new(),
            popups: Vec::new(),
            health_bar_bg,
            health_bar,
            icon_p1,
            icon_p2,
            score_txt,
            cam_pos,
            cam_follow,
            vocals: Sound::empty(),
            vocals_finished: false,
            beat: BeatTracker::default(),
            generated_music: false,
            starting_song: true,
            started_countdown: false,
            paused: false,
            can_pause: true,
            cam_zooming: false,
            in_cutscene: false,
            camera_right_side: false,
            persistent_update: true,
            persistent_draw: true,
            health: resume.as_ref().map(|r| r.health).unwrap_or(1.0),
            combo: resume.as_ref().map(|r| r.combo).unwrap_or(0),
            song_score: resume.as_ref().map(|r| r.score).unwrap_or(0),
            gf_speed: 1,
            default_cam_zoom,
            start_timer: Timer::new(),
            start_time,
            playtesting,
            silent: false,
            event_index: song.events.iter().position(|e| e.time >= start_time).unwrap_or(song.events.len()),
            swag_counter: 0,
            clock: SongClock { last_audio: f64::NAN, anchor: 0.0 },
            cam_zoom_tween: None,
            countdown_after_zoom: false,
            cutscene: Cutscene::None,
            cut_timer: Timer::new(),
            white_timer: Timer::new(),
            cut_sound: Sound::empty(),
            black_alpha: 1.0,
            black: None,
            red: None,
            senpai_evil: None,
            dialogue,
            dialogue_shown: false,
            sub: Sub::None,
            sub_fresh: false,
            ended: false,
            botplay: false,
        };
        if song.player2 == "gf" && ctx.data.session.is_story_mode {
            state.tween_cam_zoom(ctx, 1.3);
        }
        state.generate_song(ctx);
        if ctx.data.session.is_story_mode && !ctx.data.session.seen_cutscene {
            ctx.data.session.seen_cutscene = true;
            match song.id.as_str() {
                "winter-horrorland" => state.winter_intro(ctx),
                "senpai" | "roses" | "thorns" => state.school_intro(ctx),
                _ => state.start_countdown(ctx),
            }
        } else {
            state.start_countdown(ctx);
        }
        state
    }

    fn tween_cam_zoom(&mut self, ctx: &Ctx, zoom: f64) {
        self.cam_zoom_tween = Some(Tween::new(zoom, ctx.data.conductor.step_crochet * 4.0 / 1000.0).ease(Ease::ElasticInOut));
    }

    fn set_cam_follow(&mut self, ctx: &mut Ctx, x: f64, y: f64) {
        self.cam_follow = Point::new(x, y);
        ctx.camera(CAM_GAME).set_target_position(x, y);
    }

    fn winter_intro(&mut self, ctx: &mut Ctx) {
        self.black = Some(cover(ctx, 0.0, 0.0, GAME_WIDTH * 2.0, GAME_HEIGHT * 2.0, 0xFF000000));
        ctx.camera(CAM_HUD).visible = false;
        self.cut_timer.start(0.1, 1);
        self.cutscene = Cutscene::WinterDark;
    }

    fn school_intro(&mut self, ctx: &mut Ctx) {
        let mut evil = Sprite::with_frames(0.0, 0.0, ctx.atlas("weeb/senpaiCrazy"));
        evil.add_by_prefix("idle", "Senpai Pre Explosion", 24.0, false);
        let w = evil.width;
        evil.set_graphic_size((w * PIXEL_ZOOM) as i64, 0);
        evil.scroll.set(0.0, 0.0);
        evil.update_hitbox();
        evil.screen_center(Axes::XY);
        evil.x += evil.width / 5.0;
        evil.set_alpha(0.0);
        self.senpai_evil = Some(evil);
        let (x, y) = (self.cam_pos.x, self.cam_pos.y);
        self.set_cam_follow(ctx, x, y);
        match self.song.id.as_str() {
            "thorns" => {
                self.red = Some(cover(ctx, -100.0, -100.0, GAME_WIDTH * 2.0, GAME_HEIGHT * 2.0, 0xFFFF1B31));
                ctx.camera(CAM_HUD).visible = false;
            }
            "roses" => library::play_sound(ctx, "ANGRY", 1.0),
            _ => self.black = Some(cover(ctx, -100.0, -100.0, GAME_WIDTH * 2.0, GAME_HEIGHT * 2.0, 0xFF000000)),
        }
        self.black_alpha = 1.0;
        self.cut_timer.start(0.3, 1);
        self.cutscene = Cutscene::SchoolFade;
    }

    fn show_dialogue(&mut self) {
        self.dialogue_shown = true;
        self.cutscene = Cutscene::Dialogue;
    }

    fn update_cutscene(&mut self, ctx: &mut Ctx) {
        let elapsed = ctx.elapsed;
        let events = ctx.camera(CAM_GAME).take_events();
        self.cut_sound.update(elapsed);
        if let Some(evil) = self.senpai_evil.as_mut() {
            evil.update(elapsed);
        }
        let fired = self.cut_timer.update(elapsed);
        match self.cutscene {
            Cutscene::WinterDark if fired => {
                self.black = None;
                library::play_sound(ctx, "Lights_Turn_On", 1.0);
                let (x, y) = (self.cam_follow.x + 200.0, -2050.0);
                self.set_cam_follow(ctx, x, y);
                let cam = ctx.camera(CAM_GAME);
                cam.focus_on(x, y);
                cam.zoom = 1.5;
                self.cut_timer.start(0.8, 1);
                self.cutscene = Cutscene::WinterLights;
            }
            Cutscene::WinterLights if fired => {
                ctx.camera(CAM_HUD).visible = true;
                self.cam_zoom_tween = Some(Tween::new(self.default_cam_zoom, 2.5).ease(Ease::QuadInOut));
                self.countdown_after_zoom = true;
                self.cutscene = Cutscene::None;
            }
            Cutscene::SchoolFade if fired => {
                self.black_alpha = (self.black_alpha - 0.15).max(0.0);
                if let Some(black) = self.black.as_mut() {
                    black.set_alpha(self.black_alpha);
                }
                if self.black_alpha > 0.0 {
                    self.cut_timer.start(0.3, 1);
                } else {
                    self.black = None;
                    if self.dialogue.as_ref().map(|d| d.has_dialog).unwrap_or(false) {
                        self.in_cutscene = true;
                        if self.song.id == "thorns" {
                            self.cut_timer.start(0.3, 1);
                            self.cutscene = Cutscene::SenpaiRise;
                        } else {
                            self.show_dialogue();
                        }
                    } else {
                        self.cutscene = Cutscene::None;
                        self.start_countdown(ctx);
                    }
                }
            }
            Cutscene::SenpaiRise if fired => {
                let Some(evil) = self.senpai_evil.as_mut() else { return };
                let alpha = evil.alpha() + 0.15;
                evil.set_alpha(alpha);
                if evil.alpha() < 1.0 {
                    self.cut_timer.start(0.3, 1);
                } else {
                    evil.play("idle", false);
                    let data = library::sound(ctx, "Senpai_Dies");
                    self.cut_sound = ctx.audio.play_tracked(data, 1.0);
                    self.white_timer.start(3.2, 1);
                    self.cutscene = Cutscene::SenpaiDies;
                }
            }
            Cutscene::SenpaiDies => {
                if self.white_timer.update(elapsed) {
                    ctx.camera(CAM_GAME).fade(WHITE, 1.6, false, false);
                }
                if !self.cut_sound.playing() {
                    self.senpai_evil = None;
                    self.red = None;
                    ctx.camera(CAM_GAME).fade(WHITE, 0.01, true, true);
                    self.cutscene = Cutscene::WhiteIn;
                }
            }
            Cutscene::WhiteIn if events.contains(&CameraEvent::FadeDone) => {
                self.show_dialogue();
                ctx.camera(CAM_HUD).visible = true;
            }
            Cutscene::Dialogue => {
                if let Some(dialogue) = self.dialogue.as_mut() {
                    dialogue.update(ctx);
                    if dialogue.finished {
                        self.cutscene = Cutscene::None;
                        self.start_countdown(ctx);
                    }
                }
            }
            Cutscene::LightsOff => {
                if !self.cut_sound.playing() {
                    self.cutscene = Cutscene::None;
                    ctx.switch_state(|ctx| Box::new(PlayState::new(ctx)));
                }
            }
            _ => {}
        }
    }

    fn generate_song(&mut self, ctx: &mut Ctx) {
        let song = self.song.clone();
        ctx.data.conductor.change_bpm(song.bpm());
        if song.needs_voices {
            let data = library::voices(ctx, &song.id, &song.name, song.voices.as_deref());
            self.vocals = Sound::new(data, 1.0, false);
        }
        let speed = song.speed;
        let pixel = self.pixel;
        let mut order: Vec<usize> = Vec::new();
        for chart in song.notes.iter().filter(|n| n.time >= self.start_time) {
            let data = chart.lane;
            let gotta_hit = chart.side == Side::Player;
            let old = order.last().copied();
            let index = Note::create(ctx, &mut self.notes, chart.time, data, old, false, speed, pixel);
            self.notes[index].sustain_length = chart.length;
            self.notes[index].alt = chart.alt;
            self.notes[index].sprite.scroll.set(0.0, 0.0);
            order.push(index);
            let step = ctx.data.conductor.step_crochet;
            let end = chart.time + chart.length;
            let mut times = Vec::new();
            if chart.length > 0.0 {
                let mut t = chart.time;
                while t < end - 0.5 {
                    times.push(t);
                    t += step;
                }
                times.push(end);
            }
            for time in times {
                let old = order.last().copied();
                let i = Note::create(ctx, &mut self.notes, time, data, old, true, speed, pixel);
                self.notes[i].sprite.scroll.set(0.0, 0.0);
                self.notes[i].must_press = gotta_hit;
                if gotta_hit {
                    self.notes[i].sprite.x += GAME_WIDTH / 2.0;
                }
                order.push(i);
            }
            if chart.length > 0.0 {
                let body_last = order[order.len() - 2];
                let span = end - self.notes[body_last].strum_time;
                let shift = self.notes[body_last].shift;
                let fh = self.notes[body_last].sprite.frame_height();
                let travel = span * 0.45 * round_decimal(speed, 2);
                let px = if ctx.data.prefs.downscroll { travel + shift } else { (travel - shift).max(0.0) };
                self.notes[body_last].sprite.scale.y = px / fh;
                self.notes[body_last].sprite.update_hitbox();
            }
            self.notes[index].must_press = gotta_hit;
            if gotta_hit {
                self.notes[index].sprite.x += GAME_WIDTH / 2.0;
            }
        }
        let notes = &self.notes;
        order.sort_by(|a, b| notes[*a].strum_time.partial_cmp(&notes[*b].strum_time).unwrap_or(std::cmp::Ordering::Equal));
        self.unspawn = order.into();
        self.generated_music = true;
    }

    fn generate_static_arrows(&mut self, ctx: &mut Ctx, player: usize) {
        for i in 0..4 {
            let mut arrow = strum(ctx, i, player, self.strum_line_y, self.pixel);
            if !ctx.data.session.is_story_mode {
                arrow.y -= 10.0;
                arrow.set_alpha(0.0);
                let delay = 0.5 + 0.2 * i as f64;
                arrow.tween(Prop::Y, Tween::new(arrow.y + 10.0, 1.0).ease(Ease::CircOut).delay(delay));
                arrow.tween(Prop::Alpha, Tween::new(1.0, 1.0).ease(Ease::CircOut).delay(delay));
            }
            self.strums.push(arrow);
        }
    }

    fn start_countdown(&mut self, ctx: &mut Ctx) {
        self.in_cutscene = false;
        ctx.camera(CAM_HUD).visible = true;
        self.generate_static_arrows(ctx, 0);
        self.generate_static_arrows(ctx, 1);
        self.started_countdown = true;
        ctx.data.conductor.song_position = self.start_time;
        ctx.data.conductor.song_position -= ctx.data.conductor.crochet * 5.0;
        self.swag_counter = 0;
        self.start_timer.start(ctx.data.conductor.crochet / 1000.0, 4);
    }

    fn countdown_tick(&mut self, ctx: &mut Ctx) {
        let counter = self.swag_counter as i64;
        if counter % self.gf_speed == 0 {
            self.gf.dance();
        }
        if counter % 2 == 0 {
            if !self.bf.anim_name().starts_with("sing") {
                self.bf.play_anim("idle", false, false, 0);
            }
            if !self.dad.anim_name().starts_with("sing") {
                self.dad.dance();
            }
        } else if self.dad.name == "spooky" && !self.dad.anim_name().starts_with("sing") {
            self.dad.dance();
        }
        if self.generated_music {
            self.sort_notes();
        }
        let (sprites, suffix) = if self.pixel {
            (["weeb/pixelUI/ready-pixel", "weeb/pixelUI/set-pixel", "weeb/pixelUI/date-pixel"], "-pixel")
        } else {
            (["ready", "set", "go"], "")
        };
        let sounds = ["intro3", "intro2", "intro1", "introGo"];
        if self.swag_counter > 0 && self.swag_counter <= 3 {
            self.ready_set_go(ctx, sprites[self.swag_counter - 1]);
        }
        if let Some(name) = sounds.get(self.swag_counter) {
            library::play_sound(ctx, &format!("{name}{suffix}"), 0.6);
        }
        self.swag_counter += 1;
    }

    fn ready_set_go(&mut self, ctx: &mut Ctx, path: &str) {
        let mut spr = Sprite::with_frames(0.0, 0.0, ctx.image(path));
        spr.scroll.set(0.0, 0.0);
        if self.pixel {
            let w = spr.width;
            spr.set_graphic_size((w * PIXEL_ZOOM) as i64, 0);
        }
        spr.update_hitbox();
        spr.screen_center(Axes::XY);
        spr.y += 100.0;
        let duration = ctx.data.conductor.crochet / 1000.0;
        spr.tween_tagged(Prop::Y, Tween::new(spr.y, duration).ease(Ease::CubeInOut), 0);
        spr.tween_tagged(Prop::Alpha, Tween::new(0.0, duration).ease(Ease::CubeInOut), TAG_DESTROY);
        spr.moves = false;
        self.popups.push(spr);
    }

    fn start_song(&mut self, ctx: &mut Ctx) {
        self.starting_song = false;
        if !self.paused {
            let data = library::inst(ctx, &self.song.id, &self.song.name, self.song.inst.as_deref());
            self.silent = data.is_none();
            ctx.audio.play_music(data, 1.0, false);
            ctx.audio.seek_music(self.start_time);
        }
        self.vocals.play(&mut ctx.audio, false, self.start_time);
    }

    fn resync_vocals(&mut self, ctx: &mut Ctx) {
        if ctx.is_exiting() {
            return;
        }
        self.vocals.pause();
        let mut music = std::mem::take(&mut ctx.audio.music);
        music.play(&mut ctx.audio, false, 0.0);
        ctx.audio.music = music;
        ctx.data.conductor.song_position = ctx.audio.music.time() + ctx.data.conductor.offset;
        self.clock.last_audio = f64::NAN;
        if self.vocals_finished {
            return;
        }
        let pos = ctx.data.conductor.song_position;
        self.vocals.set_time(&mut ctx.audio, pos);
        self.vocals.play(&mut ctx.audio, false, 0.0);
    }

    fn sort_notes(&mut self) {
        let notes = &self.notes;
        self.alive.sort_by(|a, b| notes[*b].strum_time.partial_cmp(&notes[*a].strum_time).unwrap_or(std::cmp::Ordering::Equal));
    }

    fn remove_note(&mut self, index: usize) {
        self.alive.retain(|&i| i != index);
        self.notes[index].sprite.exists = false;
    }

    fn song_position(&mut self, ctx: &Ctx) -> f64 {
        let audio = ctx.audio.music.time();
        if audio != self.clock.last_audio {
            self.clock.last_audio = audio;
            self.clock.anchor = ctx.ticks;
        }
        let extra = if ctx.audio.music.playing() { (ctx.ticks - self.clock.anchor).min(50.0) } else { 0.0 };
        audio + extra + ctx.data.conductor.offset
    }

    fn update_members(&mut self, ctx: &mut Ctx) {
        let elapsed = ctx.elapsed;
        self.vocals.update(elapsed);
        self.stage.update(ctx, &mut self.gf, &self.dad.sprite);
        self.gf.update(ctx);
        self.dad.update(ctx);
        self.bf.update(ctx);
        for s in self.strums.iter_mut() {
            s.update(elapsed);
        }
        let conductor = ctx.data.conductor.clone();
        for &i in &self.alive {
            if self.notes[i].sprite.active {
                self.notes[i].update(elapsed, &conductor);
            }
        }
        self.health_bar.update(self.health);
        self.icon_p1.sprite.update(elapsed);
        self.icon_p2.sprite.update(elapsed);
        self.update_popups(elapsed);
    }

    fn update_popups(&mut self, elapsed: f64) {
        for p in self.popups.iter_mut() {
            p.update(elapsed);
        }
        self.popups.retain_mut(|p| !p.tweens.take_completed(TAG_DESTROY));
    }

    fn beat_hit(&mut self, ctx: &mut Ctx, beat: i64) {
        if self.generated_music {
            self.sort_notes();
        }
        if ctx.data.prefs.camera_zoom {
            if self.song.id == "milf" && (168..200).contains(&beat) && self.cam_zooming && ctx.cameras[CAM_GAME].zoom < 1.35 {
                ctx.camera(CAM_GAME).zoom += 0.015;
                ctx.camera(CAM_HUD).zoom += 0.03;
            }
            if self.cam_zooming && ctx.cameras[CAM_GAME].zoom < 1.35 && beat % 4 == 0 {
                ctx.camera(CAM_GAME).zoom += 0.015;
                ctx.camera(CAM_HUD).zoom += 0.03;
            }
        }
        for icon in [&mut self.icon_p1, &mut self.icon_p2] {
            let size = icon.size + 30.0;
            icon.set_size(size);
        }
        if beat % self.gf_speed == 0 {
            self.gf.dance();
        }
        if beat % 2 == 0 {
            if !self.bf.anim_name().starts_with("sing") {
                self.bf.play_anim("idle", false, false, 0);
            }
            if !self.dad.anim_name().starts_with("sing") {
                self.dad.dance();
            }
        } else if self.dad.name == "spooky" && !self.dad.anim_name().starts_with("sing") {
            self.dad.dance();
        }
        if beat % 8 == 7 && self.song.id == "bopeebo" {
            self.bf.play_anim("hey", true, false, 0);
        }
        if beat % 16 == 15 && self.song.id == "tutorial" && self.dad.name == "gf" && beat > 16 && beat < 48 {
            self.bf.play_anim("hey", true, false, 0);
            self.dad.play_anim("cheer", true, false, 0);
        }
        self.stage.beat_hit(ctx, beat, &mut self.bf, &mut self.gf);
    }

    fn step_hit(&mut self, ctx: &mut Ctx) {
        let pos = ctx.data.conductor.song_position - ctx.data.conductor.offset;
        if (ctx.audio.music.time() - pos).abs() > 20.0 || (self.song.needs_voices && self.vocals.playing() && (self.vocals.time() - pos).abs() > 20.0) {
            if !self.starting_song && ctx.audio.music.playing() {
                self.resync_vocals(ctx);
            }
        }
    }

    fn camera_movement(&mut self, ctx: &mut Ctx) {
        let dad_mid = self.dad.sprite.midpoint();
        if self.cam_follow.x != dad_mid.x + 150.0 && !self.camera_right_side {
            let mut target = Point::new(dad_mid.x + 150.0, dad_mid.y - 100.0);
            match self.dad.name.as_str() {
                "mom" => target.y = dad_mid.y,
                "senpai" | "senpai-angry" => {
                    target.y = dad_mid.y - 430.0;
                    target.x = dad_mid.x - 100.0;
                }
                _ => {}
            }
            self.cam_follow = target;
            if self.dad.name == "mom" {
                self.vocals.set_volume(1.0);
            }
            if self.song.id == "tutorial" {
                self.tween_cam_zoom(ctx, 1.3);
            }
        }
        let bf_mid = self.bf.sprite.midpoint();
        if self.camera_right_side && self.cam_follow.x != bf_mid.x - 100.0 {
            let mut target = Point::new(bf_mid.x - 100.0, bf_mid.y - 100.0);
            match self.stage.kind {
                Kind::Limo => target.x = bf_mid.x - 300.0,
                Kind::Mall => target.y = bf_mid.y - 200.0,
                Kind::School | Kind::SchoolEvil => {
                    target.x = bf_mid.x - 200.0;
                    target.y = bf_mid.y - 200.0;
                }
                _ => {}
            }
            self.cam_follow = target;
            if self.song.id == "tutorial" {
                self.tween_cam_zoom(ctx, 1.0);
            }
        }
        ctx.camera(CAM_GAME).set_target_position(self.cam_follow.x, self.cam_follow.y);
    }

    fn kill_combo(&mut self, ctx: &mut Ctx) {
        if self.combo > 5 && self.gf.has_anim("sad") {
            self.gf.play_anim("sad", false, false, 0);
        }
        if self.combo != 0 {
            self.combo = 0;
            self.display_combo(ctx);
        }
    }

    fn ui_image(&self, ctx: &mut Ctx, name: &str) -> Sprite {
        let path = if self.pixel { format!("weeb/pixelUI/{name}-pixel") } else { name.to_string() };
        Sprite::with_frames(0.0, 0.0, ctx.image(&path))
    }

    fn pop_up_score(&mut self, ctx: &mut Ctx, strum_time: f64) {
        let c = &ctx.data.conductor;
        let note_diff = (strum_time - c.song_position).abs();
        let safe = c.safe_zone_offset;
        let crochet = c.crochet;
        self.vocals.set_volume(1.0);
        let (rating, score) = if note_diff > safe * 0.9 {
            ("shit", 50)
        } else if note_diff > safe * 0.75 {
            ("bad", 100)
        } else if note_diff > safe * 0.2 {
            ("good", 200)
        } else {
            ("sick", 350)
        };
        if !ctx.data.session.practice_mode {
            self.song_score += score;
        }
        let (scroll_x, scroll_y, cam_w, cam_h) = {
            let cam = &ctx.cameras[CAM_GAME];
            (cam.scroll_x, cam.scroll_y, cam.width, cam.height)
        };
        let mut spr = self.ui_image(ctx, rating);
        spr.x = GAME_WIDTH * 0.55 - 40.0;
        if spr.x < scroll_x {
            spr.x = scroll_x;
        } else if spr.x > scroll_x + cam_w - spr.width {
            spr.x = scroll_x + cam_w - spr.width;
        }
        spr.y = scroll_y + cam_h * 0.4 - 60.0;
        spr.acceleration.y = 550.0;
        spr.velocity.y -= ctx.rng.int(140, 175) as f64;
        spr.velocity.x -= ctx.rng.int(0, 10) as f64;
        let w = spr.width;
        if self.pixel {
            spr.set_graphic_size((w * PIXEL_ZOOM * 0.7) as i64, 0);
        } else {
            spr.set_graphic_size((w * 0.7) as i64, 0);
            spr.antialiasing = true;
        }
        spr.update_hitbox();
        spr.tween_tagged(Prop::Alpha, Tween::new(0.0, 0.2).delay(crochet * 0.001), TAG_DESTROY);
        self.popups.push(spr);
        if self.combo >= 10 || self.combo == 0 {
            self.display_combo(ctx);
        }
    }

    fn display_combo(&mut self, ctx: &mut Ctx) {
        let crochet = ctx.data.conductor.crochet;
        let (scroll_x, scroll_y, cam_w, cam_h) = {
            let cam = &ctx.cameras[CAM_GAME];
            (cam.scroll_x, cam.scroll_y, cam.width, cam.height)
        };
        let mut combo_spr = self.ui_image(ctx, "combo");
        combo_spr.y = scroll_y + cam_h * 0.4 + 80.0;
        combo_spr.x = GAME_WIDTH * 0.55;
        if combo_spr.x < scroll_x + 194.0 {
            combo_spr.x = scroll_x + 194.0;
        } else if combo_spr.x > scroll_x + cam_w - combo_spr.width {
            combo_spr.x = scroll_x + cam_w - combo_spr.width;
        }
        combo_spr.acceleration.y = 600.0;
        combo_spr.velocity.y -= 150.0;
        combo_spr.velocity.x += ctx.rng.int(1, 10) as f64;
        let w = combo_spr.width;
        if self.pixel {
            combo_spr.set_graphic_size((w * PIXEL_ZOOM * 0.7) as i64, 0);
        } else {
            combo_spr.set_graphic_size((w * 0.7) as i64, 0);
            combo_spr.antialiasing = true;
        }
        combo_spr.update_hitbox();
        combo_spr.tween_tagged(Prop::Alpha, Tween::new(0.0, 0.2).delay(crochet * 0.001), TAG_DESTROY);
        let base_x = combo_spr.x;
        let base_y = combo_spr.y;
        self.popups.push(combo_spr);
        let mut digits = Vec::new();
        let mut temp = self.combo;
        while temp != 0 {
            digits.push(temp % 10);
            temp /= 10;
        }
        while digits.len() < 3 {
            digits.push(0);
        }
        for (i, d) in digits.iter().enumerate() {
            let mut num = self.ui_image(ctx, &format!("num{d}"));
            num.y = base_y;
            let w = num.width;
            if self.pixel {
                num.set_graphic_size((w * PIXEL_ZOOM) as i64, 0);
            } else {
                num.antialiasing = true;
                num.set_graphic_size((w * 0.5) as i64, 0);
            }
            num.update_hitbox();
            num.x = base_x - 43.0 * (i + 1) as f64;
            num.acceleration.y = ctx.rng.int(200, 300) as f64;
            num.velocity.y -= ctx.rng.int(140, 160) as f64;
            num.velocity.x = ctx.rng.float(-5.0, 5.0);
            num.tween_tagged(Prop::Alpha, Tween::new(0.0, 0.2).delay(crochet * 0.002), TAG_DESTROY);
            self.popups.push(num);
        }
    }

    fn note_miss(&mut self, ctx: &mut Ctx, direction: usize) {
        self.health -= 0.04;
        self.kill_combo(ctx);
        if !ctx.data.session.practice_mode {
            self.song_score -= 10;
        }
        self.vocals.set_volume(0.0);
        let k = ctx.rng.int(1, 3);
        let volume = ctx.rng.float(0.1, 0.2);
        library::play_sound(ctx, &format!("missnote{k}"), volume);
        self.bf.play_anim(&format!("sing{}miss", COLORS[direction]), true, false, 0);
    }

    fn good_note_hit(&mut self, ctx: &mut Ctx, index: usize) {
        if self.notes[index].was_good_hit {
            return;
        }
        if !self.notes[index].is_sustain {
            self.combo += 1;
            let time = self.notes[index].strum_time;
            self.pop_up_score(ctx, time);
        }
        self.health += 0.023;
        let data = self.notes[index].data;
        self.bf.play_anim(&format!("sing{}", COLORS[data]), true, false, 0);
        for s in self.strums.iter_mut().skip(4) {
            if s.id as usize == data {
                s.play("confirm", true);
            }
        }
        self.notes[index].was_good_hit = true;
        self.vocals.set_volume(1.0);
        if !self.notes[index].is_sustain {
            self.remove_note(index);
        }
    }

    fn center_strums(&mut self, player: usize) {
        let pixel = self.pixel;
        for s in self.strums.iter_mut().skip(player * 4).take(4) {
            s.center_offsets(false);
            if s.anim.name() == "confirm" && !pixel {
                s.offset.x -= 13.0;
                s.offset.y -= 13.0;
            }
        }
    }

    fn light_strum(&mut self, data: usize) {
        for s in self.strums.iter_mut().take(4) {
            if s.id as usize == data {
                s.play("confirm", true);
            }
        }
    }

    fn update_opponent_strums(&mut self) {
        for s in self.strums.iter_mut().take(4) {
            if s.anim.name() == "confirm" && s.anim.finished() {
                s.play("static", false);
            }
        }
        self.center_strums(0);
    }

    fn autoplay(&mut self, ctx: &mut Ctx) {
        let pos = ctx.data.conductor.song_position;
        let list: Vec<usize> = self.alive.clone();
        for i in list {
            let n = &self.notes[i];
            if n.must_press && !n.was_good_hit && !n.too_late && n.strum_time <= pos {
                self.bf.hold_timer = 0.0;
                self.good_note_hit(ctx, i);
            }
        }
        if self.bf.hold_timer > ctx.data.conductor.step_crochet * 4.0 * 0.001 {
            let name = self.bf.anim_name();
            if name.starts_with("sing") && !name.ends_with("miss") {
                self.bf.play_anim("idle", false, false, 0);
            }
        }
        for s in self.strums.iter_mut().skip(4) {
            if s.anim.name() == "confirm" && s.anim.finished() {
                s.play("static", false);
            }
        }
        self.center_strums(1);
    }

    fn key_shit(&mut self, ctx: &mut Ctx) {
        let hold = [ctx.pressed(Control::NoteLeft), ctx.pressed(Control::NoteDown), ctx.pressed(Control::NoteUp), ctx.pressed(Control::NoteRight)];
        let press = [
            ctx.just_pressed(Control::NoteLeft),
            ctx.just_pressed(Control::NoteDown),
            ctx.just_pressed(Control::NoteUp),
            ctx.just_pressed(Control::NoteRight),
        ];
        if hold.contains(&true) && self.generated_music {
            let list: Vec<usize> = self.alive.clone();
            for i in list {
                let n = &self.notes[i];
                if n.is_sustain && n.can_be_hit && n.must_press && hold[n.data] {
                    self.good_note_hit(ctx, i);
                }
            }
        }
        if press.contains(&true) && self.generated_music {
            self.bf.hold_timer = 0.0;
            let mut possible: Vec<usize> = Vec::new();
            let mut directions: Vec<usize> = Vec::new();
            let mut dumb: Vec<usize> = Vec::new();
            for &i in &self.alive {
                let n = &self.notes[i];
                if n.can_be_hit && n.must_press && !n.too_late && !n.was_good_hit {
                    if directions.contains(&n.data) {
                        for k in 0..possible.len() {
                            let cool = &self.notes[possible[k]];
                            if cool.data == n.data && (n.strum_time - cool.strum_time).abs() < 10.0 {
                                dumb.push(i);
                                break;
                            } else if cool.data == n.data && n.strum_time < cool.strum_time {
                                possible.remove(k);
                                possible.push(i);
                                break;
                            }
                        }
                    } else {
                        possible.push(i);
                        directions.push(n.data);
                    }
                }
            }
            for i in dumb {
                self.remove_note(i);
            }
            let notes = &self.notes;
            possible.sort_by_key(|&i| (notes[i].strum_time) as i64);
            let ghost = ctx.data.prefs.ghost_tapping;
            if !possible.is_empty() {
                for (d, pressed) in press.iter().enumerate() {
                    if *pressed && !ghost && !directions.contains(&d) {
                        self.note_miss(ctx, d);
                    }
                }
                for i in possible {
                    if press[self.notes[i].data] {
                        self.good_note_hit(ctx, i);
                    }
                }
            } else if !ghost {
                for (d, pressed) in press.iter().enumerate() {
                    if *pressed {
                        self.note_miss(ctx, d);
                    }
                }
            }
        }
        if self.bf.hold_timer > ctx.data.conductor.step_crochet * 4.0 * 0.001 && !hold.contains(&true) {
            let name = self.bf.anim_name();
            if name.starts_with("sing") && !name.ends_with("miss") {
                self.bf.play_anim("idle", false, false, 0);
            }
        }
        for s in self.strums.iter_mut().skip(4) {
            let id = s.id as usize;
            if press[id] && s.anim.name() != "confirm" {
                s.play("pressed", false);
            }
            if !hold[id] {
                s.play("static", false);
            }
        }
        self.center_strums(1);
    }

    fn update_notes(&mut self, ctx: &mut Ctx) {
        let downscroll = ctx.data.prefs.downscroll;
        let speed = round_decimal(self.song.speed, 2);
        let strum_mid = self.strum_line_y + SWAG_WIDTH / 2.0;
        let pos = ctx.data.conductor.song_position;
        let list: Vec<usize> = self.alive.clone();
        for i in list {
            if !self.alive.contains(&i) {
                continue;
            }
            let prev = self.notes[i].prev;
            let prev_good = self.notes[prev].was_good_hit;
            let n = &mut self.notes[i];
            let offscreen = if downscroll { n.sprite.y < -n.sprite.height } else { n.sprite.y > GAME_HEIGHT };
            n.sprite.active = !offscreen;
            n.sprite.visible = !offscreen;
            let clip_ready = !n.must_press || n.was_good_hit || (prev_good && !n.can_be_hit);
            if downscroll {
                n.sprite.y = self.strum_line_y + (pos - n.strum_time) * (0.45 * speed);
                if n.is_sustain {
                    if n.is_end() {
                        n.sprite.y -= n.sprite.height;
                    } else {
                        n.sprite.y += n.shift - n.sprite.height;
                    }
                    if clip_ready && n.sprite.y - n.sprite.offset.y * n.sprite.scale.y + n.sprite.height >= strum_mid {
                        let fw = n.sprite.frame_width();
                        let fh = n.sprite.frame_height();
                        let height = (strum_mid - n.sprite.y) / n.sprite.scale.y;
                        n.sprite.set_clip_rect(Some([0.0, fh - height, fw, height]));
                    }
                }
            } else {
                n.sprite.y = self.strum_line_y - (pos - n.strum_time) * (0.45 * speed);
                if n.is_sustain && !n.is_end() {
                    n.sprite.y += n.shift;
                }
                if n.is_sustain && clip_ready && n.sprite.y + n.sprite.offset.y * n.sprite.scale.y <= strum_mid {
                    let w = n.sprite.width / n.sprite.scale.x;
                    let h = n.sprite.height / n.sprite.scale.y;
                    let y = (strum_mid - n.sprite.y) / n.sprite.scale.y;
                    n.sprite.set_clip_rect(Some([0.0, y, w, h - y]));
                }
            }
            if !self.notes[i].must_press && self.notes[i].was_good_hit {
                if self.song.id != "tutorial" {
                    self.cam_zooming = true;
                }
                let alt = if self.notes[i].alt { "-alt" } else { "" };
                let name = format!("sing{}{}", COLORS[self.notes[i].data], alt);
                let name = if self.dad.has_anim(&name) { name } else { format!("sing{}", COLORS[self.notes[i].data]) };
                self.dad.play_anim(&name, true, false, 0);
                if ctx.data.prefs.opponent_strums {
                    self.light_strum(self.notes[i].data);
                }
                self.dad.hold_timer = 0.0;
                if let (Some(drain), false) = (self.song.drain, self.notes[i].is_sustain) {
                    let floor = 2.0 * drain.floor / 100.0;
                    if self.health > floor {
                        self.health = (self.health - drain.amount).max(floor);
                    }
                }
                if self.song.needs_voices {
                    self.vocals.set_volume(1.0);
                }
                self.remove_note(i);
                continue;
            }
            let n = &self.notes[i];
            if n.is_sustain && n.was_good_hit {
                let gone = if downscroll { n.sprite.y > GAME_HEIGHT } else { n.sprite.y < -n.sprite.height };
                if gone {
                    self.remove_note(i);
                }
            } else if n.too_late || n.was_good_hit {
                if n.too_late {
                    self.health -= 0.0475;
                    self.vocals.set_volume(0.0);
                    self.kill_combo(ctx);
                }
                self.remove_note(i);
            }
        }
    }

    fn end_song(&mut self, ctx: &mut Ctx) {
        if self.ended {
            return;
        }
        self.ended = true;
        if self.playtesting {
            ctx.audio.music.stop();
            self.vocals.stop();
            ctx.switch_state(|ctx| Box::new(super::editor::Editor::resume(ctx)));
            return;
        }
        let session = &mut ctx.data.session;
        session.seen_cutscene = false;
        session.death_counter = 0;
        self.can_pause = false;
        ctx.audio.music.set_volume(0.0);
        self.vocals.set_volume(0.0);
        if self.song.valid_score {
            ctx.data.scores.save_score(&mut ctx.save, &self.song.name, self.song_score, &self.song.difficulty);
        }
        if ctx.data.session.is_story_mode {
            ctx.data.session.campaign_score += self.song_score;
            if !ctx.data.session.story_playlist.is_empty() {
                ctx.data.session.story_playlist.remove(0);
            }
            if ctx.data.session.story_playlist.is_empty() {
                library::play_music(ctx, "freakyMenu", 1.0, true);
                ctx.trans_in = ctx.default_trans_in;
                ctx.trans_out = ctx.default_trans_out;
                let week = ctx.data.session.story_week;
                if self.song.valid_score {
                    let total = ctx.data.session.campaign_score;
                    ctx.data.scores.save_week_score(&mut ctx.save, week, total, &self.song.difficulty);
                }
                ctx.switch_state(|ctx| Box::new(super::story::StoryMenu::new(ctx)));
            } else {
                ctx.skip_next_trans_in = true;
                ctx.skip_next_trans_out = true;
                ctx.audio.music.stop();
                self.vocals.stop();
                let next = ctx.data.session.story_playlist[0].clone();
                let diff = self.song.difficulty.clone();
                ctx.data.session.song = library::chart(ctx, &next, &diff);
                if self.song.id == "eggnog" {
                    let zoom = ctx.cameras[CAM_GAME].zoom;
                    self.black = Some(cover(ctx, -GAME_WIDTH * zoom, -GAME_HEIGHT * zoom, GAME_WIDTH * 3.0, GAME_HEIGHT * 3.0, 0xFF000000));
                    ctx.camera(CAM_HUD).visible = false;
                    self.in_cutscene = true;
                    let data = library::sound(ctx, "Lights_Shut_off");
                    self.cut_sound = ctx.audio.play_tracked(data, 1.0);
                    self.cutscene = Cutscene::LightsOff;
                } else {
                    ctx.data.session.prev_cam_follow = Some(self.cam_follow);
                    ctx.switch_state(|ctx| Box::new(PlayState::new(ctx)));
                }
            }
        } else {
            ctx.switch_state(|ctx| Box::new(super::freeplay::Freeplay::new(ctx)));
        }
    }

    fn open_settings(&mut self, ctx: &mut Ctx) {
        ctx.data.session.resume = Some(Resume { time: ctx.data.conductor.song_position, health: self.health, score: self.song_score, combo: self.combo, cam_follow: self.cam_follow });
        self.vocals.stop();
        ctx.audio.music.stop();
        ctx.switch_state(|ctx| Box::new(super::options::Options::in_song(ctx)));
    }

    fn open_pause(&mut self, ctx: &mut Ctx) {
        self.persistent_update = false;
        self.persistent_draw = true;
        self.paused = true;
        if !self.playtesting && ctx.rng.bool(0.1) {
            ctx.switch_state(|ctx| Box::new(GitarooPause::new(ctx)));
            return;
        }
        ctx.audio.music.pause();
        self.vocals.pause();
        if !self.start_timer.finished {
            self.start_timer.active = false;
        }
        let pos = self.bf.sprite.screen_position(ctx.cameras[CAM_GAME].scroll_x, ctx.cameras[CAM_GAME].scroll_y);
        self.sub = Sub::Pause(PauseMenu::new(ctx, pos.x, pos.y, self.song.clone()));
        self.sub_fresh = true;
    }

    fn close_pause(&mut self, ctx: &mut Ctx) {
        self.sub = Sub::None;
        if self.paused {
            if !self.starting_song {
                self.resync_vocals(ctx);
            }
            if !self.start_timer.finished {
                self.start_timer.active = true;
            }
            self.paused = false;
        }
    }

    fn open_game_over(&mut self, ctx: &mut Ctx) {
        self.persistent_update = false;
        self.persistent_draw = false;
        self.paused = true;
        self.vocals.stop();
        ctx.audio.music.stop();
        ctx.data.session.death_counter += 1;
        let pos = self.bf.sprite.screen_position(ctx.cameras[CAM_GAME].scroll_x, ctx.cameras[CAM_GAME].scroll_y);
        self.sub = Sub::GameOver(GameOver::new(ctx, pos.x, pos.y, self.pixel));
        self.sub_fresh = true;
    }

    fn draw_holds(&self, dl: &mut DrawList, downscroll: bool) {
        let strum_mid = self.strum_line_y + SWAG_WIDTH / 2.0;
        let mut chains: HashMap<usize, HoldSpan> = HashMap::new();
        for &i in &self.alive {
            let n = &self.notes[i];
            if !n.is_sustain || n.is_end() {
                continue;
            }
            let top = n.sprite.y;
            let bottom = n.sprite.y + n.sprite.height;
            let span = chains.entry(n.head).or_insert(HoldSpan { top, top_index: i, bottom, bottom_index: i });
            if top < span.top {
                span.top = top;
                span.top_index = i;
            }
            if bottom > span.bottom {
                span.bottom = bottom;
                span.bottom_index = i;
            }
        }
        for span in chains.values() {
            let (mut top, mut bottom) = (span.top, span.bottom);
            if downscroll {
                if self.notes[span.bottom_index].sprite.clipped() {
                    bottom = bottom.min(strum_mid);
                }
            } else if self.notes[span.top_index].sprite.clipped() {
                top = top.max(strum_mid);
            }
            if bottom > top {
                self.notes[span.top_index].sprite.draw_strip(dl, CAM_HUD, top, bottom - top);
            }
        }
    }

    fn focus_on_player(&self, time: f64) -> bool {
        self.song
            .events
            .iter()
            .filter(|e| e.name == "focus" && e.time <= time)
            .last()
            .map(|e| e.text("target") == Side::Player.name())
            .unwrap_or(false)
    }

    fn character(&mut self, target: &str) -> &mut Character {
        match target {
            "player" | "bf" => &mut self.bf,
            "girlfriend" | "gf" => &mut self.gf,
            _ => &mut self.dad,
        }
    }

    fn fire_event(&mut self, ctx: &mut Ctx, event: &ChartEvent) {
        match event.name.as_str() {
            "play_animation" => {
                let animation = event.text("animation").to_string();
                let character = self.character(event.text("target"));
                if character.has_anim(&animation) {
                    character.play_anim(&animation, true, false, 0);
                    character.hold_timer = 0.0;
                }
            }
            "camera_bump" => {
                if ctx.data.prefs.camera_zoom {
                    ctx.camera(CAM_GAME).zoom += event.number("game", 0.015);
                    ctx.camera(CAM_HUD).zoom += event.number("hud", 0.03);
                }
            }
            "camera_zoom" => self.default_cam_zoom = event.number("zoom", self.default_cam_zoom),
            "gf_speed" => self.gf_speed = (event.number("speed", 1.0) as i64).max(1),
            _ => {}
        }
    }

    fn update_events(&mut self, ctx: &mut Ctx) {
        let song = self.song.clone();
        while let Some(event) = song.events.get(self.event_index) {
            if event.time > ctx.data.conductor.song_position {
                break;
            }
            self.event_index += 1;
            self.fire_event(ctx, event);
        }
    }

    fn update_song_events(&mut self) {
        let beat = self.beat.cur_beat;
        match self.song.id.as_str() {
            "fresh" => match beat {
                16 => {
                    self.cam_zooming = true;
                    self.gf_speed = 2;
                }
                48 | 112 => self.gf_speed = 1,
                80 => self.gf_speed = 2,
                _ => {}
            },
            "bopeebo" if (128..=130).contains(&beat) => self.vocals.set_volume(0.0),
            _ => {}
        }
    }

    fn update_main(&mut self, ctx: &mut Ctx) {
        if ctx.is_exiting() {
            return;
        }
        if let Some(t) = self.cam_zoom_tween.as_mut() {
            let current = ctx.cameras[CAM_GAME].zoom;
            if let Some(v) = t.step(ctx.elapsed, current) {
                ctx.camera(CAM_GAME).zoom = v;
            }
            if t.finished {
                self.cam_zoom_tween = None;
                if std::mem::replace(&mut self.countdown_after_zoom, false) {
                    self.start_countdown(ctx);
                }
            }
        }
        if self.cutscene != Cutscene::None {
            self.update_cutscene(ctx);
        }
        if self.start_timer.update(ctx.elapsed) {
            self.countdown_tick(ctx);
        }
        if self.starting_song {
            if self.started_countdown {
                ctx.data.conductor.song_position += ctx.elapsed * 1000.0;
                if ctx.data.conductor.song_position >= self.start_time {
                    self.start_song(ctx);
                }
            }
        } else if self.silent {
            if !self.paused {
                ctx.data.conductor.song_position += ctx.elapsed * 1000.0;
            }
            let last = self.song.notes.iter().map(|n| n.time + n.length).fold(0.0, f64::max);
            if ctx.data.conductor.song_position > last + 2000.0 {
                self.end_song(ctx);
            }
        } else {
            let pos = self.song_position(ctx);
            ctx.data.conductor.song_position = pos;
        }
        if !self.starting_song && !self.silent && ctx.audio.music.take_completed() {
            self.end_song(ctx);
        }
        if self.vocals.take_completed() {
            self.vocals_finished = true;
        }
        let bpm = self.song.bpm_at(ctx.data.conductor.song_position);
        if bpm != ctx.data.conductor.bpm {
            ctx.data.conductor.change_bpm(bpm);
        }
        self.update_events(ctx);
        if let Some(event) = self.beat.update(&ctx.data.conductor, true) {
            self.step_hit(ctx);
            if let Some(beat) = event.beat {
                self.beat_hit(ctx, beat);
            }
        }
        self.update_members(ctx);
        self.score_txt.set_text(&format!("Score:{}", self.song_score));

        if ctx.just_pressed(Control::Pause) && self.started_countdown && self.can_pause {
            self.open_pause(ctx);
            return;
        }
        if ctx.keys.just_pressed(miniquad::KeyCode::Key7) && !self.playtesting && !ctx.is_exiting() {
            ctx.audio.music.stop();
            self.vocals.stop();
            let id = self.song.id.clone();
            let difficulty = self.song.difficulty.clone();
            ctx.switch_state(move |ctx| Box::new(super::editor::Editor::open(ctx, &id, &difficulty)));
            return;
        }
        if ctx.keys.just_pressed(miniquad::KeyCode::Key9) {
            let player = self.song.player1.clone();
            self.icon_p1.swap_old_icon(ctx, &player);
        }

        for icon in [&mut self.icon_p1, &mut self.icon_p2] {
            let size = 150.0 + (icon.size - 150.0) * decay(0.85, ctx.elapsed);
            icon.set_size(size);
        }
        let icon_offset = 26.0;
        let percent = self.health_bar.percent();
        let along = self.health_bar.width * (remap(percent, 0.0, 100.0, 100.0, 0.0) * 0.01);
        self.icon_p1.sprite.x = self.health_bar.x + (along - icon_offset);
        self.icon_p2.sprite.x = self.health_bar.x + along - (self.icon_p2.sprite.width - icon_offset);
        if self.health > 2.0 {
            self.health = 2.0;
        }
        self.icon_p1.set_losing(percent < 20.0);
        self.icon_p2.set_losing(percent > 80.0);

        if self.generated_music {
            if self.beat.cur_step >= 0 {
                self.camera_right_side = self.focus_on_player(ctx.data.conductor.song_position);
                self.camera_movement(ctx);
            }
        }
        if self.cam_zooming {
            let k = decay(0.95, ctx.elapsed);
            let game = self.default_cam_zoom + (ctx.cameras[CAM_GAME].zoom - self.default_cam_zoom) * k;
            let hud = 1.0 + (ctx.cameras[CAM_HUD].zoom - 1.0) * k;
            ctx.camera(CAM_GAME).zoom = game;
            ctx.camera(CAM_HUD).zoom = hud;
        }
        self.update_song_events();
        if !self.in_cutscene && !ctx.is_exiting() {
            if ctx.just_pressed(Control::Reset) {
                self.health = 0.0;
            }
            if self.health <= 0.0 && !ctx.data.session.practice_mode {
                self.open_game_over(ctx);
                return;
            }
        }
        let speed = self.song.speed;
        while let Some(&next) = self.unspawn.front() {
            if self.notes[next].strum_time - ctx.data.conductor.song_position < 1800.0 / speed {
                self.unspawn.pop_front();
                self.alive.push(next);
            } else {
                break;
            }
        }
        if self.generated_music {
            self.update_notes(ctx);
        }
        if ctx.data.prefs.opponent_strums {
            self.update_opponent_strums();
        }
        if ctx.keys.just_pressed(miniquad::KeyCode::F6) {
            self.botplay = !self.botplay;
        }
        if !self.in_cutscene {
            if self.botplay {
                self.autoplay(ctx);
            } else {
                self.key_shit(ctx);
            }
        }
    }
}

impl State for PlayState {
    fn update(&mut self, ctx: &mut Ctx) {
        if self.persistent_update || matches!(self.sub, Sub::None) {
            self.update_main(ctx);
        } else {
            for s in self.strums.iter_mut() {
                s.update_tweens(ctx.elapsed);
            }
            for p in self.popups.iter_mut() {
                p.update_tweens(ctx.elapsed);
            }
            self.popups.retain_mut(|p| !p.tweens.take_completed(TAG_DESTROY));
        }
        let fresh = std::mem::replace(&mut self.sub_fresh, false);
        match &mut self.sub {
            Sub::None => {}
            Sub::Pause(menu) => {
                if fresh {
                    menu.skip_input();
                }
                match menu.update(ctx) {
                    Some(PauseAction::Resume) => self.close_pause(ctx),
                    Some(PauseAction::Restart) => ctx.switch_state(|ctx| Box::new(PlayState::new(ctx))),
                    Some(PauseAction::Settings) => self.open_settings(ctx),
                    Some(PauseAction::Exit) => {
                        ctx.data.session.seen_cutscene = false;
                        ctx.data.session.death_counter = 0;
                        if self.playtesting {
                            ctx.switch_state(|ctx| Box::new(super::editor::Editor::resume(ctx)));
                        } else if ctx.data.session.is_story_mode {
                            ctx.switch_state(|ctx| Box::new(super::story::StoryMenu::new(ctx)));
                        } else {
                            ctx.switch_state(|ctx| Box::new(super::freeplay::Freeplay::new(ctx)));
                        }
                    }
                    None => {}
                }
            }
            Sub::GameOver(over) => {
                if fresh {
                    over.skip_input();
                }
                over.update(ctx);
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList) {
        if self.persistent_draw || matches!(self.sub, Sub::None) {
            self.stage.draw_back(dl, CAM_GAME);
            self.gf.draw(dl, CAM_GAME);
            self.stage.draw_middle(dl, CAM_GAME);
            self.dad.draw(dl, CAM_GAME);
            self.bf.draw(dl, CAM_GAME);
            for s in &self.strums {
                s.draw(dl, CAM_HUD);
            }
            self.draw_holds(dl, ctx.data.prefs.downscroll);
            for &i in &self.alive {
                if self.notes[i].is_sustain && self.notes[i].is_end() {
                    self.notes[i].sprite.draw(dl, CAM_HUD);
                }
            }
            for &i in &self.alive {
                if !self.notes[i].is_sustain {
                    self.notes[i].sprite.draw(dl, CAM_HUD);
                }
            }
            self.health_bar_bg.draw(dl, CAM_HUD);
            self.health_bar.draw(ctx, dl, CAM_HUD);
            self.score_txt.draw(&mut ctx.gfx, dl, CAM_HUD);
            self.icon_p1.sprite.draw(dl, CAM_HUD);
            self.icon_p2.sprite.draw(dl, CAM_HUD);
            for s in [&self.black, &self.red, &self.senpai_evil].into_iter().flatten() {
                s.draw(dl, CAM_GAME);
            }
            if self.dialogue_shown {
                if let Some(dialogue) = self.dialogue.as_mut() {
                    dialogue.draw(ctx, dl, CAM_HUD);
                }
            }
            for p in &self.popups {
                p.draw(dl, CAM_GAME);
            }
        }
        match &mut self.sub {
            Sub::None => {}
            Sub::Pause(menu) => menu.draw(ctx, dl, CAM_HUD),
            Sub::GameOver(over) => over.draw(ctx, dl, CAM_GAME),
        }
    }
}