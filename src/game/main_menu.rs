use crate::engine::app::{Ctx, State};
use crate::engine::camera::{FollowStyle, Target};
use crate::engine::gfx::{DrawList, GAME_HEIGHT, GAME_WIDTH};
use crate::engine::sprite::{Axes, Sprite};
use crate::engine::text::{Align, Border, Text};
use crate::engine::timer::Timer;
use crate::engine::tween::{Ease, Prop, Tween};

use super::data::{Control, Input};
use super::library;
use super::widgets::Flicker;

const ITEMS: [&str; 4] = ["story mode", "freeplay", "donate", "options"];
const DONATE_URL: &str = "https://www.kickstarter.com/projects/funkin/friday-night-funkin-the-full-ass-game/";

pub struct MainMenu {
    bg: Sprite,
    magenta: Option<Sprite>,
    magenta_flicker: Flicker,
    items: Vec<Sprite>,
    selected: usize,
    enabled: bool,
    busy: bool,
    item_flicker: Flicker,
    exit_timer: Timer,
    version: Text,
}

fn change_anim(item: &mut Sprite, anim: &str) {
    item.play(anim, false);
    item.update_hitbox();
    item.center_origin();
    item.offset = item.origin;
}

fn open_url(url: &str) {
    let program = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let _ = std::process::Command::new(program).arg(url).spawn();
}

impl MainMenu {
    pub fn new(ctx: &mut Ctx) -> MainMenu {
        ctx.trans_in = ctx.default_trans_in;
        ctx.trans_out = ctx.default_trans_out;
        if !ctx.audio.music.playing() {
            library::play_music(ctx, "freakyMenu", 1.0, true);
        }
        let mut bg = Sprite::with_frames(0.0, 0.0, ctx.image("menuBG"));
        bg.scroll.set(0.0, 0.17);
        let w = bg.width;
        bg.set_graphic_size((w * 1.2) as i64, 0);
        bg.update_hitbox();
        bg.screen_center(Axes::XY);
        bg.antialiasing = true;

        let magenta = if ctx.data.prefs.flashing_menu {
            let mut m = Sprite::with_frames(0.0, 0.0, ctx.image("menuDesat"));
            m.scroll = bg.scroll;
            m.set_graphic_size(bg.width as i64, 0);
            m.update_hitbox();
            m.x = bg.x;
            m.y = bg.y;
            m.visible = false;
            m.antialiasing = true;
            m.color = 0xFD719B;
            Some(m)
        } else {
            None
        };

        let atlas = ctx.atlas("FNF_main_menu_assets");
        let spacing = 160.0;
        let top = (GAME_HEIGHT - spacing * (ITEMS.len() as f64 - 1.0)) / 2.0;
        let mut items = Vec::new();
        for (i, name) in ITEMS.iter().enumerate() {
            let mut item = Sprite::with_frames(0.0, 0.0, atlas.clone());
            item.add_by_prefix("idle", &format!("{name} basic"), 24.0, true);
            item.add_by_prefix("selected", &format!("{name} white"), 24.0, true);
            item.antialiasing = true;
            item.id = i as i32;
            item.scroll.set(0.0, 0.0);
            change_anim(&mut item, if i == 0 { "selected" } else { "idle" });
            item.set_alpha(1.0);
            item.x = GAME_WIDTH / 2.0;
            item.y = top + spacing * i as f64;
            items.push(item);
        }

        let first = items[0].graphic_midpoint();
        let cam = ctx.camera(0);
        cam.follow(Target { x: first.x, y: first.y, width: 1.0, height: 1.0 }, FollowStyle::LockOn, 0.06);

        let mut version = Text::new(5.0, GAME_HEIGHT - 22.0, 0.0, concat!("funkin-rs v", env!("CARGO_PKG_VERSION")), 20.0, ctx.data.fonts.ui.clone());
        version.set_format(ctx.data.fonts.ui.clone(), 20.0, 0xFFFFFFFF, Align::Left, Border::Outline, 0xFF000000);
        version.sprite.scroll.set(0.0, 0.0);

        MainMenu {
            bg,
            magenta,
            magenta_flicker: Flicker::default(),
            items,
            selected: 0,
            enabled: false,
            busy: false,
            item_flicker: Flicker::default(),
            exit_timer: Timer::new(),
            version,
        }
    }

    fn select(&mut self, ctx: &mut Ctx, index: usize) {
        change_anim(&mut self.items[self.selected], "idle");
        self.selected = index;
        change_anim(&mut self.items[index], "selected");
        let mid = self.items[index].graphic_midpoint();
        ctx.camera(0).set_target_position(mid.x, mid.y);
    }

    fn start_exit(&mut self) {
        self.enabled = false;
        for item in self.items.iter_mut() {
            if item.id as usize != self.selected {
                item.tween(Prop::Alpha, Tween::new(0.0, 0.4).ease(Ease::QuadOut));
            } else {
                item.visible = false;
            }
        }
        self.exit_timer.start(0.4, 1);
    }
}

impl State for MainMenu {
    fn persistent_update(&self) -> bool {
        true
    }

    fn trans_in_done(&mut self, _ctx: &mut Ctx) {
        self.enabled = true;
    }

    fn update(&mut self, ctx: &mut Ctx) {
        if ctx.audio.music.volume() < 0.8 {
            let v = ctx.audio.music.volume() + 0.5 * ctx.elapsed;
            ctx.audio.music.set_volume(v);
        }
        if ctx.is_exiting() {
            self.enabled = false;
        }
        if ctx.just_pressed(Control::Back) && self.enabled && !self.busy {
            library::play_sound(ctx, "cancelMenu", 1.0);
            ctx.switch_state(|ctx| Box::new(super::title::Title::new(ctx)));
        }
        if self.exit_timer.update(ctx.elapsed) {
            match self.selected {
                0 => ctx.switch_state(|ctx| Box::new(super::story::StoryMenu::new(ctx))),
                1 => ctx.switch_state(|ctx| Box::new(super::freeplay::Freeplay::new(ctx))),
                _ => ctx.switch_state(|ctx| Box::new(super::options::Options::new(ctx))),
            }
        }
        if let Some(m) = self.magenta.as_mut() {
            let mut visible = m.visible;
            self.magenta_flicker.update(ctx.elapsed, &mut visible);
            m.visible = visible;
        }
        let selected = self.selected;
        let mut visible = self.items[selected].visible;
        if self.item_flicker.update(ctx.elapsed, &mut visible) {
            self.items[selected].visible = visible;
            self.busy = false;
            if ITEMS[selected] == "donate" {
                open_url(DONATE_URL);
            } else {
                self.start_exit();
            }
        } else {
            self.items[selected].visible = visible;
        }
        if self.enabled && !self.busy {
            let len = self.items.len();
            let up = ctx.just_pressed(Control::UiUp);
            let down = ctx.just_pressed(Control::UiDown);
            if up != down {
                let next = if up { (self.selected + len - 1) % len } else { (self.selected + 1) % len };
                library::play_sound(ctx, "scrollMenu", 1.0);
                self.select(ctx, next);
            }
            if ctx.just_pressed(Control::Accept) {
                self.magenta_flicker = Flicker::start(1.1, 0.15, false);
                self.busy = true;
                library::play_sound(ctx, "confirmMenu", 1.0);
                self.item_flicker = Flicker::start(1.0, 0.06, true);
            }
        }
        self.bg.update(ctx.elapsed);
        for item in self.items.iter_mut() {
            item.update(ctx.elapsed);
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList) {
        self.bg.draw(dl, 0);
        if let Some(m) = &self.magenta {
            m.draw(dl, 0);
        }
        for item in &self.items {
            item.draw(dl, 0);
        }
        self.version.draw(&mut ctx.gfx, dl, 0);
    }
}