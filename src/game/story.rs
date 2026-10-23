use crate::engine::app::{Ctx, State};
use crate::engine::gfx::{DrawList, GAME_WIDTH};
use crate::engine::math::{frame_lerp, haxe_round, lerp};
use crate::engine::sprite::{Axes, Sprite};
use crate::engine::text::{Align, Text};
use crate::engine::timer::Timer;
use crate::engine::tween::{Prop, Tween};

use super::data::{Control, Input};
use super::library;
use super::play::PlayState;
use super::weeks::WEEKS;

const WEEK_PREFIXES: [&str; 7] = ["tutorial selected", "WEEK1 select", "week2 select", "Week 3 press", "Week 4 press", "week 5", "Week 6"];
const MENU_CHARACTERS: [(&str, &str); 8] = [
    ("bf", "BF idle dance white"),
    ("gf", "GF Dancing Beat WHITE"),
    ("dad", "Dad idle dance BLACK LINE"),
    ("spooky", "spooky dance idle BLACK LINES"),
    ("pico", "Pico Idle Dance"),
    ("mom", "Mom Idle BLACK LINES"),
    ("parents-christmas", "Parent Christmas Idle"),
    ("senpai", "SENPAI idle Black Lines"),
];

struct WeekItem {
    sprite: Sprite,
    target_y: f64,
    flashing: bool,
    flashing_int: i64,
    fake_framerate: i64,
}

impl WeekItem {
    fn update(&mut self, elapsed: f64) {
        self.sprite.update(elapsed);
        self.sprite.y = lerp(self.sprite.y, self.target_y * 120.0 + 480.0, frame_lerp(elapsed, 0.17));
        if self.flashing {
            self.flashing_int += 1;
        }
        let ff = self.fake_framerate.max(1);
        self.sprite.color = if self.flashing_int % ff >= ff / 2 { 0x33FFFF } else { 0xFFFFFF };
    }
}

fn menu_character(ctx: &mut Ctx, x: f64, character: &str) -> Sprite {
    let frames = ctx.atlas("campaign_menu_UI_characters");
    let mut s = Sprite::with_frames(x, 0.0, frames);
    for (name, prefix) in MENU_CHARACTERS {
        s.add_by_prefix(name, prefix, 24.0, true);
    }
    s.add_by_prefix("bfConfirm", "BF HEY!!", 24.0, false);
    s.play(character, false);
    s.update_hitbox();
    s
}

fn resize(s: &mut Sprite, factor: f64) {
    let w = s.width;
    s.set_graphic_size((w * factor) as i64, 0);
}

pub struct StoryMenu {
    score_text: Text,
    title_text: Text,
    yellow: Sprite,
    black_bar: Sprite,
    weeks: Vec<WeekItem>,
    characters: Vec<Sprite>,
    left_arrow: Sprite,
    right_arrow: Sprite,
    difficulty: Sprite,
    tracklist: Text,
    cur_week: usize,
    cur_difficulty: usize,
    lerp_score: f64,
    intended_score: i64,
    moved_back: bool,
    selected_week: bool,
    stop_spamming: bool,
    start_timer: Timer,
}

impl StoryMenu {
    pub fn new(ctx: &mut Ctx) -> StoryMenu {
        ctx.trans_in = ctx.default_trans_in;
        ctx.trans_out = ctx.default_trans_out;
        if !ctx.audio.music.playing() {
            library::play_music(ctx, "freakyMenu", 1.0, true);
        }
        let font = ctx.data.fonts.ui.clone();
        let score_text = Text::new(10.0, 10.0, 0.0, "SCORE: 49324858", 38.0, font.clone());
        let mut title_text = Text::new(GAME_WIDTH * 0.7, 10.0, 0.0, "", 38.0, font.clone());
        title_text.set_align(Align::Right);
        title_text.sprite.set_alpha(0.7);

        let ui = ctx.atlas("campaign_menu_UI_assets");
        let yellow = Sprite::with_frames(0.0, 56.0, ctx.solid(GAME_WIDTH, 400.0, 0xFFF9CF51));
        let black_bar = Sprite::with_frames(0.0, 0.0, ctx.solid(GAME_WIDTH, 56.0, 0xFF000000));

        let fake_framerate = haxe_round((1.0 / ctx.elapsed.max(1.0 / 1000.0)) / 10.0) as i64;
        let mut weeks = Vec::new();
        for (i, prefix) in WEEK_PREFIXES.iter().enumerate() {
            let mut sprite = Sprite::with_frames(0.0, yellow.y + yellow.height + 10.0, ui.clone());
            sprite.add_by_prefix("week", prefix, 24.0, true);
            sprite.play("week", false);
            sprite.anim.pause();
            sprite.update_hitbox();
            sprite.y += (sprite.height + 20.0) * i as f64;
            sprite.screen_center(Axes::X);
            sprite.antialiasing = true;
            weeks.push(WeekItem { sprite, target_y: i as f64, flashing: false, flashing_int: 0, fake_framerate });
        }

        let mut characters = Vec::new();
        for (slot, name) in WEEKS[0].characters.iter().enumerate() {
            let mut c = menu_character(ctx, GAME_WIDTH * 0.25 * (1 + slot) as f64 - 150.0, name);
            c.y += 70.0;
            c.antialiasing = true;
            match *name {
                "dad" | "gf" => {
                    resize(&mut c, 0.5);
                    c.update_hitbox();
                }
                "bf" => {
                    resize(&mut c, 0.9);
                    c.update_hitbox();
                    c.x -= 80.0;
                }
                _ => {}
            }
            characters.push(c);
        }

        let first = &weeks[0].sprite;
        let mut left_arrow = Sprite::with_frames(first.x + first.width + 10.0, first.y + 10.0, ui.clone());
        left_arrow.add_by_prefix("idle", "arrow left", 30.0, true);
        left_arrow.add_by_prefix("press", "arrow push left", 30.0, true);
        left_arrow.play("idle", false);
        let mut difficulty = Sprite::with_frames(left_arrow.x + 130.0, left_arrow.y, ui.clone());
        difficulty.add_by_prefix("easy", "EASY", 30.0, true);
        difficulty.add_by_prefix("normal", "NORMAL", 30.0, true);
        difficulty.add_by_prefix("hard", "HARD", 30.0, true);
        difficulty.play("easy", false);

        let tracklist_y = yellow.x + yellow.height + 100.0;
        let mut tracklist = Text::new(GAME_WIDTH * 0.05, tracklist_y, 0.0, "Tracks", 38.0, font);
        tracklist.set_align(Align::Center);
        tracklist.set_color(0xFFE55777);

        let mut menu = StoryMenu {
            score_text,
            title_text,
            yellow,
            black_bar,
            weeks,
            characters,
            left_arrow,
            right_arrow: Sprite::new(0.0, 0.0),
            difficulty,
            tracklist,
            cur_week: 0,
            cur_difficulty: 1,
            lerp_score: 0.0,
            intended_score: 0,
            moved_back: false,
            selected_week: false,
            stop_spamming: false,
            start_timer: Timer::new(),
        };
        menu.change_difficulty(ctx, 0);
        let mut right_arrow = Sprite::with_frames(menu.difficulty.x + menu.difficulty.width + 50.0, menu.left_arrow.y, ui);
        right_arrow.add_by_prefix("idle", "arrow right", 30.0, true);
        right_arrow.add_by_prefix("press", "arrow push right", 24.0, false);
        right_arrow.play("idle", false);
        menu.right_arrow = right_arrow;
        menu.update_text(ctx);
        menu
    }

    fn update_text(&mut self, ctx: &mut Ctx) {
        let week = &WEEKS[self.cur_week];
        for (c, name) in self.characters.iter_mut().zip(week.characters) {
            c.play(name, false);
        }
        let opponent = &mut self.characters[0];
        let (offset, factor) = match week.characters[0] {
            "parents-christmas" => ((200.0, 200.0), 0.99),
            "senpai" => ((130.0, 0.0), 1.4),
            "mom" => ((100.0, 200.0), 1.0),
            "dad" => ((120.0, 200.0), 1.0),
            _ => ((100.0, 100.0), 1.0),
        };
        opponent.offset.set(offset.0, offset.1);
        resize(opponent, factor);
        let mut text = String::from("Tracks\n");
        for song in week.songs {
            text.push('\n');
            text.push_str(song);
        }
        self.tracklist.set_text(&text.to_uppercase());
        self.tracklist.sprite.screen_center(Axes::X);
        self.tracklist.sprite.x -= GAME_WIDTH * 0.35;
        self.intended_score = ctx.data.scores.week_score(self.cur_week, self.cur_difficulty);
    }

    fn change_week(&mut self, ctx: &mut Ctx, change: i64) {
        let len = self.weeks.len() as i64;
        self.cur_week = ((self.cur_week as i64 + change).rem_euclid(len)) as usize;
        for (i, item) in self.weeks.iter_mut().enumerate() {
            item.target_y = i as f64 - self.cur_week as f64;
            item.sprite.set_alpha(if item.target_y == 0.0 { 1.0 } else { 0.6 });
        }
        library::play_sound(ctx, "scrollMenu", 1.0);
        self.update_text(ctx);
    }

    fn change_difficulty(&mut self, ctx: &mut Ctx, change: i64) {
        self.cur_difficulty = ((self.cur_difficulty as i64 + change).rem_euclid(3)) as usize;
        let (name, offset) = [("easy", 20.0), ("normal", 70.0), ("hard", 20.0)][self.cur_difficulty];
        self.difficulty.play(name, false);
        self.difficulty.offset.x = offset;
        self.difficulty.set_alpha(0.0);
        self.difficulty.y = self.left_arrow.y - 15.0;
        self.intended_score = ctx.data.scores.week_score(self.cur_week, self.cur_difficulty);
        self.difficulty.tween(Prop::Y, Tween::new(self.left_arrow.y + 15.0, 0.07));
        self.difficulty.tween(Prop::Alpha, Tween::new(1.0, 0.07));
    }

    fn select_week(&mut self, ctx: &mut Ctx) {
        if !self.stop_spamming {
            library::play_sound(ctx, "confirmMenu", 1.0);
            self.weeks[self.cur_week].flashing = true;
            self.characters[1].play("bfConfirm", false);
            self.stop_spamming = true;
        }
        let week = &WEEKS[self.cur_week];
        let session = &mut ctx.data.session;
        session.story_playlist = week.songs.iter().map(|s| s.to_string()).collect();
        session.is_story_mode = true;
        session.story_difficulty = self.cur_difficulty;
        session.story_week = self.cur_week;
        session.campaign_score = 0;
        self.selected_week = true;
        ctx.data.session.song = library::chart(ctx, week.songs[0], self.cur_difficulty);
        self.start_timer.start(1.0, 1);
    }
}

impl State for StoryMenu {
    fn persistent_update(&self) -> bool {
        true
    }

    fn update(&mut self, ctx: &mut Ctx) {
        self.lerp_score = lerp(self.lerp_score, self.intended_score as f64, frame_lerp(ctx.elapsed, 0.5));
        self.score_text.set_text(&format!("WEEK SCORE:{}", haxe_round(self.lerp_score) as i64));
        self.title_text.set_text(&WEEKS[self.cur_week].name.to_uppercase());
        self.title_text.sprite.x = GAME_WIDTH - (self.title_text.width() + 10.0);
        if self.start_timer.update(ctx.elapsed) && ctx.data.session.song.is_some() {
            ctx.switch_state(|ctx| Box::new(PlayState::new(ctx)));
        }
        if !self.moved_back {
            if !self.selected_week {
                if ctx.just_pressed(Control::UiUp) {
                    self.change_week(ctx, -1);
                }
                if ctx.just_pressed(Control::UiDown) {
                    self.change_week(ctx, 1);
                }
                self.right_arrow.play(if ctx.pressed(Control::UiRight) { "press" } else { "idle" }, false);
                self.left_arrow.play(if ctx.pressed(Control::UiLeft) { "press" } else { "idle" }, false);
                if ctx.just_pressed(Control::UiRight) {
                    self.change_difficulty(ctx, 1);
                }
                if ctx.just_pressed(Control::UiLeft) {
                    self.change_difficulty(ctx, -1);
                }
            }
            if ctx.just_pressed(Control::Accept) {
                self.select_week(ctx);
            }
        }
        if ctx.just_pressed(Control::Back) && !self.moved_back && !self.selected_week {
            library::play_sound(ctx, "cancelMenu", 1.0);
            self.moved_back = true;
            ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
        }
        let elapsed = ctx.elapsed;
        for w in self.weeks.iter_mut() {
            w.update(elapsed);
        }
        for s in self.characters.iter_mut().chain([&mut self.left_arrow, &mut self.right_arrow, &mut self.difficulty]) {
            s.update(elapsed);
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList) {
        for w in &self.weeks {
            w.sprite.draw(dl, 0);
        }
        self.black_bar.draw(dl, 0);
        self.left_arrow.draw(dl, 0);
        self.difficulty.draw(dl, 0);
        self.right_arrow.draw(dl, 0);
        self.yellow.draw(dl, 0);
        for c in &self.characters {
            c.draw(dl, 0);
        }
        self.tracklist.draw(&mut ctx.gfx, dl, 0);
        self.score_text.draw(&mut ctx.gfx, dl, 0);
        self.title_text.draw(&mut ctx.gfx, dl, 0);
    }
}