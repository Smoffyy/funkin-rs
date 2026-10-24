use crate::engine::app::Ctx;
use crate::engine::camera::{CameraEvent, FollowStyle, Target};
use crate::engine::gfx::DrawList;
use crate::engine::math::Point;
use crate::engine::timer::Timer;

use super::character::Character;
use super::data::{Control, Input};
use super::library;

pub struct GameOver {
    bf: Character,
    cam_follow: Point,
    is_ending: bool,
    end_timer: Timer,
    fading: bool,
    skip: bool,
    suffix: &'static str,
}

impl GameOver {
    pub fn new(ctx: &mut Ctx, x: f64, y: f64, pixel: bool) -> GameOver {
        let suffix = if pixel { "-pixel" } else { "" };
        let character = if pixel { "bf-pixel-dead" } else { "bf" };
        ctx.data.conductor.song_position = 0.0;
        let bf = Character::new(ctx, x, y, character, true);
        let mid = bf.sprite.graphic_midpoint();
        library::play_sound(ctx, &format!("fnf_loss_sfx{suffix}"), 1.0);
        ctx.data.conductor.change_bpm(100.0);
        let cam = ctx.camera(0);
        cam.scroll_x = 0.0;
        cam.scroll_y = 0.0;
        cam.target = None;
        let mut over = GameOver { bf, cam_follow: mid, is_ending: false, end_timer: Timer::new(), fading: false, skip: false, suffix };
        over.bf.play_anim("firstDeath", false, false, 0);
        over
    }

    pub fn skip_input(&mut self) {
        self.skip = true;
    }

    fn end(&mut self, ctx: &mut Ctx) {
        if self.is_ending {
            return;
        }
        self.is_ending = true;
        self.bf.play_anim("deathConfirm", true, false, 0);
        ctx.audio.music.stop();
        let data = library::music(ctx, &format!("gameOverEnd{}", self.suffix));
        ctx.audio.play(data, 1.0);
        self.end_timer.start(0.7, 1);
    }

    pub fn update(&mut self, ctx: &mut Ctx) {
        self.bf.update(ctx);
        if self.end_timer.update(ctx.elapsed) {
            ctx.camera(0).fade(0xFF000000, 2.0, false, false);
            self.fading = true;
        }
        if self.fading && ctx.cameras[0].take_events().contains(&CameraEvent::FadeDone) {
            self.fading = false;
            ctx.switch_state(|ctx| Box::new(super::play::PlayState::new(ctx)));
        }
        let skip = std::mem::replace(&mut self.skip, false);
        if !skip && ctx.just_pressed(Control::Accept) {
            self.end(ctx);
        }
        if !skip && ctx.just_pressed(Control::Back) && !ctx.is_exiting() {
            ctx.data.session.death_counter = 0;
            ctx.data.session.seen_cutscene = false;
            ctx.audio.music.stop();
            if ctx.data.session.editor.is_some() {
                ctx.switch_state(|ctx| Box::new(super::editor::Editor::resume(ctx)));
            } else if ctx.data.session.is_story_mode {
                ctx.switch_state(|ctx| Box::new(super::story::StoryMenu::new(ctx)));
            } else {
                ctx.switch_state(|ctx| Box::new(super::freeplay::Freeplay::new(ctx)));
            }
        }
        if self.bf.anim_name() == "firstDeath" && self.bf.sprite.anim.cur_frame() == 12 && ctx.cameras[0].target.is_none() {
            let target = Target { x: self.cam_follow.x, y: self.cam_follow.y, width: 1.0, height: 1.0 };
            ctx.camera(0).follow(target, FollowStyle::LockOn, 0.01);
        }
        if self.bf.anim_name() == "firstDeath" && self.bf.sprite.anim.finished() && !self.bf.started_death {
            self.bf.started_death = true;
            if !self.is_ending {
                library::play_music(ctx, &format!("gameOver{}", self.suffix), 1.0, true);
            }
        }
        if ctx.audio.music.playing() {
            ctx.data.conductor.song_position = ctx.audio.music.time();
        }
    }

    pub fn draw(&mut self, _ctx: &mut Ctx, dl: &mut DrawList, camera: usize) {
        self.bf.draw(dl, camera);
    }
}