use crate::engine::app::{Ctx, State};
use crate::engine::gfx::{DrawList, GAME_WIDTH};
use crate::engine::math::{frame_lerp, haxe_round, interpolate_color, lerp};
use crate::engine::sprite::Sprite;
use crate::engine::text::{Align, Text};

use super::data::{Control, Input};
use super::library;
use super::play::PlayState;
use super::alphabet::Alphabet;
use super::song::song_id;
use super::weeks::{FREEPLAY_COLORS, WEEKS};
use super::widgets::HealthIcon;

struct Entry {
    id: String,
    name: String,
    week: usize,
    icon: String,
    difficulties: Vec<(String, Option<String>)>,
}

pub struct Freeplay {
    songs: Vec<Entry>,
    bg: Sprite,
    items: Vec<Alphabet>,
    icons: Vec<HealthIcon>,
    score_text: Text,
    score_bg: Sprite,
    diff_text: Text,
    selected: usize,
    difficulty: String,
    preview: Option<(String, Option<String>)>,
    lerp_score: f64,
    intended_score: i64,
}

impl Freeplay {
    pub fn new(ctx: &mut Ctx) -> Freeplay {
        let mut songs = Vec::new();
        let listed = ctx.assets.read_text("data/freeplaySonglist.txt").unwrap_or_default();
        for name in listed.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if !WEEKS[1..].iter().any(|w| w.songs.iter().any(|s| s.eq_ignore_ascii_case(name))) {
                songs.push(Entry { id: song_id(name), name: name.to_string(), week: 1, icon: "gf".to_string(), difficulties: Vec::new() });
            }
        }
        for (w, week) in WEEKS.iter().enumerate().skip(1) {
            for (i, song) in week.songs.iter().enumerate() {
                songs.push(Entry { id: song_id(song), name: song.to_string(), week: w, icon: week.icon(i).to_string(), difficulties: Vec::new() });
            }
        }
        for chart in library::custom_songs(ctx) {
            if !songs.iter().any(|s| s.id == chart.id) {
                songs.push(Entry { id: chart.id, name: chart.name, week: 0, icon: chart.opponent, difficulties: Vec::new() });
            }
        }
        for entry in songs.iter_mut() {
            if let Some(chart) = library::chart_file(ctx, &entry.id) {
                entry.difficulties = chart.difficulties.iter().map(|d| (d.name.clone(), d.inst.clone())).collect();
            }
        }
        songs.retain(|s| !s.difficulties.is_empty());
        if !ctx.audio.music.playing() {
            library::play_music(ctx, "freakyMenu", 1.0, true);
        }
        let bg = Sprite::with_frames(0.0, 0.0, ctx.image("menuDesat"));
        let mut items = Vec::new();
        let mut icons = Vec::new();
        for (i, s) in songs.iter().enumerate() {
            let mut item = Alphabet::new(ctx, 0.0, 70.0 * i as f64 + 30.0, &s.name, true);
            item.is_menu_item = true;
            item.target_y = i as f64;
            items.push(item);
            icons.push(HealthIcon::new(ctx, &s.icon, false));
        }
        let font = ctx.data.fonts.ui.clone();
        let mut score_text = Text::new(GAME_WIDTH * 0.7, 5.0, 0.0, "", 38.0, font.clone());
        score_text.set_align(Align::Right);
        let score_bg = Sprite::with_frames(score_text.sprite.x - 6.0, 0.0, ctx.solid(1.0, 66.0, 0x99000000));
        let diff_text = Text::new(score_text.sprite.x, score_text.sprite.y + 36.0, 0.0, "", 28.0, font);
        let mut f = Freeplay {
            songs,
            bg,
            items,
            icons,
            score_text,
            score_bg,
            diff_text,
            selected: 0,
            difficulty: "normal".to_string(),
            preview: None,
            lerp_score: 0.0,
            intended_score: 0,
        };
        if !f.songs.is_empty() {
            f.change_selection(ctx, 0);
        }
        f
    }

    fn change_diff(&mut self, ctx: &mut Ctx, change: i64) {
        let entry = &self.songs[self.selected];
        let names: Vec<&str> = entry.difficulties.iter().map(|(n, _)| n.as_str()).collect();
        let current = names.iter().position(|n| *n == self.difficulty).or_else(|| names.iter().position(|n| *n == "normal")).unwrap_or(0) as i64;
        let next = (current + change).rem_euclid(names.len() as i64) as usize;
        self.difficulty = names[next].to_string();
        let inst = entry.difficulties[next].1.clone();
        let (id, name) = (entry.id.clone(), entry.name.clone());
        self.intended_score = ctx.data.scores.score(&name, &self.difficulty);
        self.diff_text.set_text(&format!("< {} >", self.difficulty.to_uppercase()));
        self.position_highscore();
        let wanted = (id.clone(), inst.clone());
        if self.preview.as_ref() != Some(&wanted) {
            let data = library::inst(ctx, &id, &name, inst.as_deref());
            ctx.audio.play_music(data, 0.0, true);
            self.preview = Some(wanted);
        }
    }

    fn change_selection(&mut self, ctx: &mut Ctx, change: i64) {
        library::play_sound(ctx, "scrollMenu", 0.4);
        let len = self.songs.len() as i64;
        self.selected = ((self.selected as i64 + change).rem_euclid(len)) as usize;
        for icon in self.icons.iter_mut() {
            icon.sprite.set_alpha(0.6);
        }
        self.icons[self.selected].sprite.set_alpha(1.0);
        for (i, item) in self.items.iter_mut().enumerate() {
            item.target_y = i as f64 - self.selected as f64;
            item.set_alpha(if item.target_y == 0.0 { 1.0 } else { 0.6 });
        }
        self.change_diff(ctx, 0);
    }

    fn position_highscore(&mut self) {
        self.score_text.sprite.x = GAME_WIDTH - self.score_text.width() - 6.0;
        self.score_bg.scale.x = GAME_WIDTH - self.score_text.sprite.x + 6.0;
        self.score_bg.x = GAME_WIDTH - self.score_bg.scale.x / 2.0;
        self.diff_text.sprite.x = (self.score_bg.x + self.score_bg.width / 2.0).trunc();
        self.diff_text.sprite.x -= self.diff_text.width() / 2.0;
    }
}

impl State for Freeplay {
    fn update(&mut self, ctx: &mut Ctx) {
        for item in self.items.iter_mut() {
            item.update(ctx.elapsed);
        }
        for (icon, item) in self.icons.iter_mut().zip(self.items.iter()) {
            icon.sprite.update(ctx.elapsed);
            icon.sprite.set_position(item.x + item.width() + 10.0, item.y - 30.0);
        }
        if ctx.audio.music.volume() < 0.7 {
            let v = ctx.audio.music.volume() + 0.5 * ctx.elapsed;
            ctx.audio.music.set_volume(v);
        }
        if self.songs.is_empty() {
            if ctx.just_pressed(Control::Back) {
                ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
            }
            return;
        }
        self.lerp_score = lerp(self.lerp_score, self.intended_score as f64, frame_lerp(ctx.elapsed, 0.4));
        let week = self.songs[self.selected].week;
        let target = FREEPLAY_COLORS[week % FREEPLAY_COLORS.len()];
        let current = 0xFF000000 | self.bg.color;
        self.bg.color = interpolate_color(current, target, frame_lerp(ctx.elapsed, 0.045)) & 0xFFFFFF;
        self.score_text.set_text(&format!("PERSONAL BEST:{}", haxe_round(self.lerp_score) as i64));
        self.position_highscore();
        if ctx.just_pressed(Control::UiUp) {
            self.change_selection(ctx, -1);
        }
        if ctx.just_pressed(Control::UiDown) {
            self.change_selection(ctx, 1);
        }
        if ctx.just_pressed(Control::UiLeft) {
            self.change_diff(ctx, -1);
        }
        if ctx.just_pressed(Control::UiRight) {
            self.change_diff(ctx, 1);
        }
        if ctx.just_pressed(Control::Back) {
            library::play_sound(ctx, "cancelMenu", 1.0);
            ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
        }
        if ctx.keys.just_pressed(miniquad::KeyCode::Key7) && !ctx.is_exiting() {
            let id = self.songs[self.selected].id.clone();
            let difficulty = self.difficulty.clone();
            ctx.switch_state(move |ctx| Box::new(super::editor::Editor::open(ctx, &id, &difficulty)));
        }
        if ctx.just_pressed(Control::Accept) && !ctx.is_exiting() {
            let id = self.songs[self.selected].id.clone();
            let difficulty = self.difficulty.clone();
            if let Some(song) = library::chart(ctx, &id, &difficulty) {
                ctx.data.session.song = Some(song);
                ctx.data.session.is_story_mode = false;
                ctx.data.session.story_week = self.songs[self.selected].week;
                ctx.switch_state(|ctx| Box::new(PlayState::new(ctx)));
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList) {
        self.bg.draw(dl, 0);
        for item in self.items.iter_mut() {
            item.draw(dl, 0);
        }
        for icon in &self.icons {
            icon.sprite.draw(dl, 0);
        }
        self.score_bg.draw(dl, 0);
        self.diff_text.draw(&mut ctx.gfx, dl, 0);
        self.score_text.draw(&mut ctx.gfx, dl, 0);
    }
}