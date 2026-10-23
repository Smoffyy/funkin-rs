use std::rc::Rc;

use crate::engine::app::Ctx;
use crate::engine::audio::Sound;
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::sprite::Sprite;
use crate::engine::text::Text;
use crate::engine::tween::{Ease, Prop, Tween};

use super::data::{Control, Input};
use super::library;
use super::song::Song;
use super::alphabet::Alphabet;

pub enum PauseAction {
    Resume,
    Restart,
    Settings,
    Exit,
}

const MAIN: [&str; 6] = ["Resume", "Restart Song", "Change Difficulty", "Toggle Practice Mode", "Settings", "Exit to menu"];

pub struct PauseMenu {
    song: Rc<Song>,
    music: Sound,
    bg: Sprite,
    infos: Vec<Text>,
    practice: Text,
    items: Vec<Alphabet>,
    labels: Vec<String>,
    choosing: bool,
    selected: usize,
    skip: bool,
}

impl PauseMenu {
    pub fn new(ctx: &mut Ctx, _x: f64, _y: f64, song: Rc<Song>) -> PauseMenu {
        let data = library::music(ctx, "breakfast");
        let mut music = Sound::new(data, 0.0, true);
        let half = (music.length() / 2.0) as i64;
        let start = ctx.rng.int(0, half.max(0)) as f64;
        music.play(&mut ctx.audio, false, start);
        music.set_volume(0.0);

        let mut bg = Sprite::with_frames(0.0, 0.0, ctx.solid(GAME_WIDTH, GAME_HEIGHT, 0xFF000000));
        bg.set_alpha(0.0);
        bg.scroll.set(0.0, 0.0);
        bg.tween(Prop::Alpha, Tween::new(0.6, 0.4).ease(Ease::QuartInOut));

        let font = ctx.data.fonts.ui.clone();
        let lines = [song.name.clone(), song.difficulty.to_uppercase(), format!("Deaths: {}", ctx.data.session.death_counter)];
        let mut infos = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let mut t = Text::new(20.0, 15.0 + 32.0 * i as f64, 0.0, line, 32.0, font.clone());
            t.set_size(32.0);
            t.sprite.scroll.set(0.0, 0.0);
            t.sprite.set_alpha(0.0);
            t.sprite.x = GAME_WIDTH - (t.width() + 20.0);
            let target = if i == 0 { 20.0 } else { t.sprite.y + 5.0 };
            let delay = 0.3 + 0.2 * i as f64;
            t.sprite.tween(Prop::Alpha, Tween::new(1.0, 0.4).ease(Ease::QuartInOut).delay(delay));
            t.sprite.tween(Prop::Y, Tween::new(target, 0.4).ease(Ease::QuartInOut).delay(delay));
            infos.push(t);
        }
        let mut practice = Text::new(20.0, 15.0 + 96.0, 0.0, "PRACTICE MODE", 32.0, font);
        practice.sprite.scroll.set(0.0, 0.0);
        practice.sprite.x = GAME_WIDTH - (practice.width() + 20.0);
        practice.sprite.visible = ctx.data.session.practice_mode;

        let mut menu = PauseMenu { song, music, bg, infos, practice, items: Vec::new(), labels: Vec::new(), choosing: false, selected: 0, skip: false };
        menu.regen(ctx, MAIN.iter().map(|s| s.to_string()).collect());
        menu
    }

    pub fn skip_input(&mut self) {
        self.skip = true;
    }

    fn regen(&mut self, ctx: &mut Ctx, labels: Vec<String>) {
        self.items.clear();
        self.labels = labels.clone();
        for (i, label) in labels.iter().enumerate() {
            let mut item = Alphabet::new(ctx, 0.0, 70.0 * i as f64 + 30.0, label, true);
            item.is_menu_item = true;
            item.target_y = i as f64;
            item.scroll.set(0.0, 0.0);
            self.items.push(item);
        }
        self.selected = 0;
        self.change_selection(ctx, 0);
    }

    fn change_selection(&mut self, ctx: &mut Ctx, change: i64) {
        library::play_sound(ctx, "scrollMenu", 0.4);
        let len = self.items.len() as i64;
        self.selected = ((self.selected as i64 + change).rem_euclid(len)) as usize;
        for (i, item) in self.items.iter_mut().enumerate() {
            item.target_y = i as f64 - self.selected as f64;
            item.set_alpha(if item.target_y == 0.0 { 1.0 } else { 0.6 });
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx) -> Option<PauseAction> {
        if self.music.volume() < 0.5 {
            let v = self.music.volume() + 0.01 * ctx.elapsed;
            self.music.set_volume(v);
        }
        self.music.update(ctx.elapsed);
        self.bg.update(ctx.elapsed);
        for t in self.infos.iter_mut() {
            t.sprite.update(ctx.elapsed);
        }
        for item in self.items.iter_mut() {
            item.update(ctx.elapsed);
        }
        if std::mem::replace(&mut self.skip, false) {
            return None;
        }
        if ctx.just_pressed(Control::UiUp) {
            self.change_selection(ctx, -1);
        }
        if ctx.just_pressed(Control::UiDown) {
            self.change_selection(ctx, 1);
        }
        if !ctx.just_pressed(Control::Accept) {
            return None;
        }
        let label = self.labels[self.selected].clone();
        if self.choosing {
            if label == "BACK" {
                self.choosing = false;
                self.regen(ctx, MAIN.iter().map(|s| s.to_string()).collect());
                return None;
            }
            if let Some(song) = library::chart(ctx, &self.song.id, &label.to_lowercase()) {
                ctx.data.session.song = Some(song);
            }
            self.music.stop();
            return Some(PauseAction::Restart);
        }
        let action = match label.as_str() {
            "Resume" => Some(PauseAction::Resume),
            "Toggle Practice Mode" => {
                ctx.data.session.practice_mode = !ctx.data.session.practice_mode;
                self.practice.sprite.visible = ctx.data.session.practice_mode;
                None
            }
            "Change Difficulty" => {
                let mut names: Vec<String> = library::chart_file(ctx, &self.song.id).map(|c| c.names()).unwrap_or_default().iter().map(|n| n.to_uppercase()).collect();
                names.push("BACK".to_string());
                self.choosing = true;
                self.regen(ctx, names);
                None
            }
            "Restart Song" => Some(PauseAction::Restart),
            "Settings" => Some(PauseAction::Settings),
            _ => Some(PauseAction::Exit),
        };
        if action.is_some() {
            self.music.stop();
        }
        action
    }

    pub fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList, camera: usize) {
        self.bg.draw(dl, camera);
        for t in self.infos.iter_mut() {
            t.draw(&mut ctx.gfx, dl, camera);
        }
        self.practice.draw(&mut ctx.gfx, dl, camera);
        for item in self.items.iter_mut() {
            item.draw(dl, camera);
        }
    }
}

impl Drop for PauseMenu {
    fn drop(&mut self) {
        self.music.stop();
    }
}