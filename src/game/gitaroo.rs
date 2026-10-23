use crate::engine::app::{Ctx, State};
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::sprite::{Axes, Sprite};

use super::data::{Control, Input};

pub struct GitarooPause {
    bg: Sprite,
    bf: Sprite,
    replay: Sprite,
    cancel: Sprite,
    replay_select: bool,
}

fn button(ctx: &mut Ctx, x: f64, y: f64, blue: &str, yellow: &str) -> Sprite {
    let frames = ctx.atlas("pauseAlt/pauseUI");
    let mut s = Sprite::with_frames(x, y, frames.clone());
    s.anim.add_by_prefix(&frames, "selected", blue, 0.0, false, false);
    s.anim.append_by_prefix(&frames, "selected", yellow);
    s.play("selected", false);
    s
}

impl GitarooPause {
    pub fn new(ctx: &mut Ctx) -> GitarooPause {
        ctx.audio.music.stop();
        let bg = Sprite::with_frames(0.0, 0.0, ctx.image("pauseAlt/pauseBG"));
        let mut bf = Sprite::with_frames(0.0, 30.0, ctx.atlas("pauseAlt/bfLol"));
        bf.add_by_prefix("lol", "funnyThing", 13.0, true);
        bf.play("lol", false);
        bf.screen_center(Axes::X);
        let replay = button(ctx, GAME_WIDTH * 0.28, GAME_HEIGHT * 0.7, "bluereplay", "yellowreplay");
        let cancel = button(ctx, GAME_WIDTH * 0.58, replay.y, "bluecancel", "cancelyellow");
        let mut pause = GitarooPause { bg, bf, replay, cancel, replay_select: false };
        pause.change_thing();
        pause
    }

    fn change_thing(&mut self) {
        self.replay_select = !self.replay_select;
        let (cancel, replay) = if self.replay_select { (0, 1) } else { (1, 0) };
        self.cancel.anim.set_cur_frame(cancel);
        self.replay.anim.set_cur_frame(replay);
    }
}

impl State for GitarooPause {
    fn update(&mut self, ctx: &mut Ctx) {
        if ctx.just_pressed(Control::UiLeft) || ctx.just_pressed(Control::UiRight) {
            self.change_thing();
        }
        if ctx.just_pressed(Control::Accept) && !ctx.is_exiting() {
            if self.replay_select {
                ctx.switch_state(|ctx| Box::new(super::play::PlayState::new(ctx)));
            } else {
                ctx.switch_state(|ctx| Box::new(super::main_menu::MainMenu::new(ctx)));
            }
        }
        for s in [&mut self.bg, &mut self.bf, &mut self.replay, &mut self.cancel] {
            s.update(ctx.elapsed);
        }
    }

    fn draw(&mut self, _ctx: &mut Ctx, dl: &mut DrawList) {
        for s in [&self.bg, &self.bf, &self.replay, &self.cancel] {
            s.draw(dl, 0);
        }
    }
}