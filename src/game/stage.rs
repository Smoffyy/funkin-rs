use std::collections::VecDeque;

use crate::engine::app::Ctx;
use crate::engine::audio::Sound;
use crate::engine::gfx::{DrawList, Effect, GAME_WIDTH};
use crate::engine::sprite::Sprite;
use crate::engine::timer::Timer;

use super::character::Character;
use super::library;

pub const PIXEL_ZOOM: f64 = 6.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Stage,
    Spooky,
    Philly,
    Limo,
    Mall,
    MallEvil,
    School,
    SchoolEvil,
}

impl Kind {
    pub const ALL: [Kind; 8] = [Kind::Stage, Kind::Spooky, Kind::Philly, Kind::Limo, Kind::Mall, Kind::MallEvil, Kind::School, Kind::SchoolEvil];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Stage => "stage",
            Kind::Spooky => "spooky",
            Kind::Philly => "philly",
            Kind::Limo => "limo",
            Kind::Mall => "mall",
            Kind::MallEvil => "mallEvil",
            Kind::School => "school",
            Kind::SchoolEvil => "schoolEvil",
        }
    }

    pub fn from_name(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name().eq_ignore_ascii_case(name))
    }

    pub fn for_chart(stage: Option<&str>, song: &str) -> Kind {
        stage.and_then(Kind::from_name).unwrap_or_else(|| Kind::for_song(song))
    }

    pub fn for_song(song: &str) -> Kind {
        match song.to_lowercase().as_str() {
            "spookeez" | "monster" | "south" => Kind::Spooky,
            "pico" | "blammed" | "philly" => Kind::Philly,
            "milf" | "satin-panties" | "high" => Kind::Limo,
            "cocoa" | "eggnog" => Kind::Mall,
            "winter-horrorland" => Kind::MallEvil,
            "senpai" | "roses" => Kind::School,
            "thorns" => Kind::SchoolEvil,
            _ => Kind::Stage,
        }
    }

    pub fn is_pixel(self) -> bool {
        matches!(self, Kind::School | Kind::SchoolEvil)
    }

    pub fn gf_version(self) -> &'static str {
        match self {
            Kind::Limo => "gf-car",
            Kind::Mall | Kind::MallEvil => "gf-christmas",
            Kind::School | Kind::SchoolEvil => "gf-pixel",
            _ => "gf",
        }
    }
}

#[derive(PartialEq)]
enum Role {
    Scenery,
    Halloween,
    Light,
    Train,
    Dancer,
    Boppers,
    Santa,
    Girls,
    FastCar,
}

struct Piece {
    sprite: Sprite,
    role: Role,
}

#[derive(Clone, Copy)]
struct Snapshot {
    x: f64,
    y: f64,
    angle: f64,
    scale: (f64, f64),
    frame: usize,
    flip_x: bool,
    flip_y: bool,
}

struct Trail {
    sprites: Vec<Sprite>,
    recent: VecDeque<Snapshot>,
    timer: f64,
}

impl Trail {
    const LENGTH: usize = 4;
    const DELAY: f64 = 24.0 / 60.0;

    fn new(target: &Sprite) -> Trail {
        let mut sprites = Vec::new();
        let mut alpha = 0.3;
        for _ in 0..Self::LENGTH {
            let mut s = Sprite::new(0.0, 0.0);
            if let Some(frames) = target.frames.clone() {
                s.set_frames(frames);
            }
            s.antialiasing = target.antialiasing;
            s.origin = target.origin;
            s.exists = false;
            s.set_alpha(alpha);
            alpha -= 0.069;
            sprites.push(s);
        }
        Trail { sprites, recent: VecDeque::new(), timer: 0.0 }
    }

    fn update(&mut self, elapsed: f64, target: &Sprite) {
        self.timer += elapsed;
        if self.timer < Self::DELAY {
            return;
        }
        self.timer = 0.0;
        if self.recent.len() == Self::LENGTH {
            self.recent.pop_back();
        }
        self.recent.push_front(Snapshot {
            x: target.x - target.offset.x,
            y: target.y - target.offset.y,
            angle: target.angle,
            scale: (target.scale.x, target.scale.y),
            frame: target.anim.frame_index,
            flip_x: target.flip_x,
            flip_y: target.flip_y,
        });
        for (sprite, snap) in self.sprites.iter_mut().zip(&self.recent) {
            sprite.set_position(snap.x, snap.y);
            sprite.angle = snap.angle;
            sprite.scale.set(snap.scale.0, snap.scale.1);
            sprite.anim.frame_index = snap.frame;
            sprite.flip_x = snap.flip_x;
            sprite.flip_y = snap.flip_y;
            sprite.exists = true;
        }
    }
}

pub struct Stage {
    pub kind: Kind,
    pub zoom: f64,
    pieces: Vec<Piece>,
    limo: Option<Sprite>,
    trail: Option<Trail>,
    dance_right: bool,
    light_fade: f64,
    train_sound: Sound,
    train_moving: bool,
    train_timing: f64,
    train_cars: i64,
    train_finishing: bool,
    train_cooldown: i64,
    train_started: bool,
    lightning_beat: i64,
    lightning_offset: i64,
    fast_car_can_drive: bool,
    fast_car_timer: Timer,
}

fn image(ctx: &mut Ctx, x: f64, y: f64, key: &str) -> Sprite {
    Sprite::with_frames(x, y, ctx.image(key))
}

fn animated(ctx: &mut Ctx, x: f64, y: f64, key: &str, name: &str, prefix: &str, fps: f64, looped: bool) -> Sprite {
    let mut s = Sprite::with_frames(x, y, ctx.atlas(key));
    s.add_by_prefix(name, prefix, fps, looped);
    s
}

fn playing(mut s: Sprite, name: &str) -> Sprite {
    s.play(name, false);
    s
}

fn sized(mut s: Sprite, factor: f64) -> Sprite {
    let w = s.width;
    s.set_graphic_size((w * factor) as i64, 0);
    s.update_hitbox();
    s
}

fn scrolled(mut s: Sprite, x: f64, y: f64, antialiasing: bool) -> Sprite {
    s.scroll.set(x, y);
    s.antialiasing = antialiasing;
    s
}

fn dancer(ctx: &mut Ctx, x: f64, y: f64, key: &str, prefix: &str) -> Sprite {
    let mut s = Sprite::with_frames(x, y, ctx.atlas(key));
    let left: Vec<i64> = (0..15).collect();
    let right: Vec<i64> = (15..30).collect();
    s.add_by_indices("danceLeft", prefix, &left, 24.0, false);
    s.add_by_indices("danceRight", prefix, &right, 24.0, false);
    s.play("danceLeft", false);
    s.anim.finish();
    s
}

impl Stage {
    pub fn new(ctx: &mut Ctx, kind: Kind, song: &str) -> Stage {
        let mut stage = Stage {
            kind,
            zoom: 1.05,
            pieces: Vec::new(),
            limo: None,
            trail: None,
            dance_right: false,
            light_fade: 0.0,
            train_sound: Sound::empty(),
            train_moving: false,
            train_timing: 0.0,
            train_cars: 8,
            train_finishing: false,
            train_cooldown: 0,
            train_started: false,
            lightning_beat: 0,
            lightning_offset: 8,
            fast_car_can_drive: true,
            fast_car_timer: Timer::new(),
        };
        match kind {
            Kind::Spooky => {
                let mut bg = Sprite::with_frames(-200.0, -100.0, ctx.atlas("halloween_bg"));
                bg.add_by_prefix("idle", "halloweem bg0", 30.0, true);
                bg.add_by_prefix("lightning", "halloweem bg lightning strike", 24.0, false);
                bg.play("idle", false);
                bg.antialiasing = true;
                stage.add(bg, Role::Halloween);
            }
            Kind::Philly => {
                stage.add(scrolled(image(ctx, -100.0, 0.0, "philly/sky"), 0.1, 0.1, false), Role::Scenery);
                let city = sized(scrolled(image(ctx, -10.0, 0.0, "philly/city"), 0.3, 0.3, false), 0.85);
                let city_x = city.x;
                stage.add(city, Role::Scenery);
                for i in 0..5 {
                    let mut light = sized(scrolled(image(ctx, city_x, 0.0, &format!("philly/win{i}")), 0.3, 0.3, true), 0.85);
                    light.visible = false;
                    stage.add(light, Role::Light);
                }
                let behind = image(ctx, -40.0, 50.0, "philly/behindTrain");
                let street_y = behind.y;
                stage.add(behind, Role::Scenery);
                stage.add(image(ctx, 2000.0, 360.0, "philly/train"), Role::Train);
                let data = library::sound(ctx, "train_passes");
                stage.train_sound = Sound::new(data, 1.0, false);
                stage.add(image(ctx, -40.0, street_y, "philly/street"), Role::Scenery);
            }
            Kind::Limo => {
                stage.zoom = 0.9;
                stage.add(scrolled(image(ctx, -120.0, -50.0, "limo/limoSunset"), 0.1, 0.1, false), Role::Scenery);
                let bg_limo = scrolled(playing(animated(ctx, -200.0, 480.0, "limo/bgLimo", "drive", "background limo pink", 24.0, true), "drive"), 0.4, 0.4, false);
                let dancer_y = bg_limo.y - 400.0;
                stage.add(bg_limo, Role::Scenery);
                for i in 0..5 {
                    let mut d = dancer(ctx, 370.0 * i as f64 + 130.0, dancer_y, "limo/limoDancer", "bg dancer sketch PINK");
                    d.antialiasing = true;
                    d.scroll.set(0.4, 0.4);
                    stage.add(d, Role::Dancer);
                }
                let mut limo = playing(animated(ctx, -120.0, 550.0, "limo/limoDrive", "drive", "Limo stage", 24.0, true), "drive");
                limo.antialiasing = true;
                stage.limo = Some(limo);
                stage.add(image(ctx, -300.0, 160.0, "limo/fastCarLol"), Role::FastCar);
                stage.reset_fast_car(ctx);
            }
            Kind::Mall => {
                stage.zoom = 0.8;
                stage.add(sized(scrolled(image(ctx, -1000.0, -500.0, "christmas/bgWalls"), 0.2, 0.2, true), 0.8), Role::Scenery);
                let upper = animated(ctx, -240.0, -90.0, "christmas/upperBop", "bop", "Upper Crowd Bob", 24.0, false);
                stage.add(sized(scrolled(upper, 0.33, 0.33, true), 0.85), Role::Boppers);
                stage.add(sized(scrolled(image(ctx, -1100.0, -600.0, "christmas/bgEscalator"), 0.3, 0.3, true), 0.9), Role::Scenery);
                stage.add(scrolled(image(ctx, 370.0, -250.0, "christmas/christmasTree"), 0.4, 0.4, true), Role::Scenery);
                let bottom = animated(ctx, -300.0, 140.0, "christmas/bottomBop", "bop", "Bottom Level Boppers", 24.0, false);
                stage.add(sized(scrolled(bottom, 0.9, 0.9, true), 1.0), Role::Boppers);
                stage.add(scrolled(image(ctx, -600.0, 700.0, "christmas/fgSnow"), 1.0, 1.0, true), Role::Scenery);
                let santa = animated(ctx, -840.0, 150.0, "christmas/santa", "idle", "santa idle in fear", 24.0, false);
                stage.add(scrolled(santa, 1.0, 1.0, true), Role::Santa);
            }
            Kind::MallEvil => {
                stage.add(sized(scrolled(image(ctx, -400.0, -500.0, "christmas/evilBG"), 0.2, 0.2, true), 0.8), Role::Scenery);
                stage.add(scrolled(image(ctx, 300.0, -300.0, "christmas/evilTree"), 0.2, 0.2, true), Role::Scenery);
                stage.add(scrolled(image(ctx, -200.0, 700.0, "christmas/evilSnow"), 1.0, 1.0, true), Role::Scenery);
            }
            Kind::School => {
                let shift = -200.0;
                let sky = scrolled(image(ctx, 0.0, 0.0, "weeb/weebSky"), 0.1, 0.1, false);
                let wide = (sky.width * 6.0) as i64;
                let school = scrolled(image(ctx, shift, 0.0, "weeb/weebSchool"), 0.6, 0.9, false);
                let street = scrolled(image(ctx, shift, 0.0, "weeb/weebStreet"), 0.95, 0.95, false);
                let back_trees = scrolled(image(ctx, shift + 170.0, 130.0, "weeb/weebTreesBack"), 0.9, 0.9, false);
                let mut trees = Sprite::with_frames(shift - 380.0, -800.0, ctx.atlas("weeb/weebTrees"));
                trees.add_anim("treeLoop", &(0..19).collect::<Vec<usize>>(), 12.0, true, false);
                trees.play("treeLoop", false);
                trees.scroll.set(0.85, 0.85);
                let petals = scrolled(playing(animated(ctx, shift, -40.0, "weeb/petals", "leaves", "PETALS ALL", 24.0, true), "leaves"), 0.85, 0.85, false);
                let resize = |mut s: Sprite, width: i64| {
                    s.set_graphic_size(width, 0);
                    s.update_hitbox();
                    s
                };
                let wide_f = wide as f64;
                stage.add(resize(sky, wide), Role::Scenery);
                stage.add(resize(school, wide), Role::Scenery);
                stage.add(resize(street, wide), Role::Scenery);
                stage.add(resize(back_trees, (wide_f * 0.8) as i64), Role::Scenery);
                stage.add(resize(trees, (wide_f * 1.4) as i64), Role::Scenery);
                stage.add(resize(petals, wide), Role::Scenery);
                let prefix = if song.eq_ignore_ascii_case("roses") { "BG fangirls dissuaded" } else { "BG girls group" };
                let mut girls = dancer(ctx, -100.0, 190.0, "weeb/bgFreaks", prefix);
                girls.scroll.set(0.9, 0.9);
                if song.eq_ignore_ascii_case("roses") {
                    stage.dance_right = true;
                    girls.play("danceRight", true);
                    girls.anim.finish();
                }
                stage.add(sized(girls, PIXEL_ZOOM), Role::Girls);
            }
            Kind::SchoolEvil => {
                let mut bg = scrolled(playing(animated(ctx, 400.0, 200.0, "weeb/animatedEvilSchool", "idle", "background 2", 24.0, true), "idle"), 0.8, 0.9, false);
                bg.scale.set(6.0, 6.0);
                stage.add(bg, Role::Scenery);
            }
            Kind::Stage => {
                stage.zoom = 0.9;
                stage.add(scrolled(image(ctx, -600.0, -200.0, "stageback"), 0.9, 0.9, true), Role::Scenery);
                stage.add(sized(scrolled(image(ctx, -650.0, 600.0, "stagefront"), 0.9, 0.9, true), 1.1), Role::Scenery);
                stage.add(sized(scrolled(image(ctx, -500.0, -300.0, "stagecurtains"), 1.3, 1.3, true), 0.9), Role::Scenery);
            }
        }
        stage
    }

    fn add(&mut self, sprite: Sprite, role: Role) {
        self.pieces.push(Piece { sprite, role });
    }

    fn each(&mut self, role: Role) -> impl Iterator<Item = &mut Sprite> {
        self.pieces.iter_mut().filter(move |p| p.role == role).map(|p| &mut p.sprite)
    }

    pub fn attach_trail(&mut self, target: &Sprite) {
        if self.kind == Kind::SchoolEvil {
            self.trail = Some(Trail::new(target));
        }
    }

    pub fn reposition(&self, bf: &mut Character, gf: &mut Character, dad: &mut Character) {
        match self.kind {
            Kind::Limo => {
                bf.sprite.y -= 220.0;
                bf.sprite.x += 260.0;
            }
            Kind::Mall => bf.sprite.x += 200.0,
            Kind::MallEvil => {
                bf.sprite.x += 320.0;
                dad.sprite.y -= 80.0;
            }
            Kind::School | Kind::SchoolEvil => {
                bf.sprite.x += 200.0;
                bf.sprite.y += 220.0;
                gf.sprite.x += 180.0;
                gf.sprite.y += 300.0;
            }
            _ => {}
        }
    }

    fn reset_fast_car(&mut self, ctx: &mut Ctx) {
        let y = ctx.rng.int(140, 250) as f64;
        for car in self.each(Role::FastCar) {
            car.x = -12600.0;
            car.y = y;
            car.velocity.x = 0.0;
        }
        self.fast_car_can_drive = true;
    }

    fn fast_car_drive(&mut self, ctx: &mut Ctx) {
        let k = ctx.rng.int(0, 1);
        library::play_sound(ctx, &format!("carPass{k}"), 0.7);
        let speed = (ctx.rng.int(170, 220) as f64 / ctx.elapsed) * 3.0;
        for car in self.each(Role::FastCar) {
            car.velocity.x = speed;
        }
        self.fast_car_can_drive = false;
        self.fast_car_timer.start(2.0, 1);
    }

    fn train_start(&mut self, ctx: &mut Ctx) {
        self.train_moving = true;
        self.train_sound.play(&mut ctx.audio, true, 0.0);
    }

    fn update_train_pos(&mut self, gf: &mut Character) {
        if self.train_sound.time() >= 4700.0 {
            self.train_started = true;
            gf.play_anim("hairBlow", false, false, 0);
        }
        if !self.train_started {
            return;
        }
        let mut reset = false;
        let finishing = self.train_finishing;
        let mut lost_car = false;
        for train in self.each(Role::Train) {
            train.x -= 400.0;
            if train.x < -2000.0 && !finishing {
                train.x = -1150.0;
                lost_car = true;
            }
            if train.x < -4000.0 && finishing {
                reset = true;
            }
        }
        if lost_car {
            self.train_cars -= 1;
            if self.train_cars <= 0 {
                self.train_finishing = true;
            }
        }
        if reset {
            self.train_reset(gf);
        }
    }

    fn train_reset(&mut self, gf: &mut Character) {
        gf.play_anim("hairFall", false, false, 0);
        for train in self.each(Role::Train) {
            train.x = GAME_WIDTH + 200.0;
        }
        self.train_moving = false;
        self.train_cars = 8;
        self.train_finishing = false;
        self.train_started = false;
    }

    pub fn update(&mut self, ctx: &mut Ctx, gf: &mut Character, dad: &Sprite) {
        let elapsed = ctx.elapsed;
        if self.kind == Kind::Philly {
            if self.train_moving {
                self.train_timing += elapsed;
                if self.train_timing >= 1.0 / 24.0 {
                    self.update_train_pos(gf);
                    self.train_timing = 0.0;
                }
            }
            self.light_fade += (ctx.data.conductor.crochet / 1000.0) * elapsed * 1.5;
            let fade = Effect::Fade(self.light_fade as f32);
            for light in self.each(Role::Light) {
                light.effect = fade;
            }
            self.train_sound.update(elapsed);
        }
        if self.fast_car_timer.update(elapsed) {
            self.reset_fast_car(ctx);
        }
        for piece in self.pieces.iter_mut() {
            piece.sprite.update(elapsed);
        }
        if let Some(limo) = self.limo.as_mut() {
            limo.update(elapsed);
        }
        if let Some(trail) = self.trail.as_mut() {
            trail.update(elapsed, dad);
        }
    }

    pub fn beat_hit(&mut self, ctx: &mut Ctx, beat: i64, bf: &mut Character, gf: &mut Character) {
        match self.kind {
            Kind::School => {
                self.dance_right = !self.dance_right;
                let name = if self.dance_right { "danceRight" } else { "danceLeft" };
                for girls in self.each(Role::Girls) {
                    girls.play(name, true);
                }
            }
            Kind::Mall => {
                for bopper in self.each(Role::Boppers) {
                    bopper.play("bop", true);
                }
                for santa in self.each(Role::Santa) {
                    santa.play("idle", true);
                }
            }
            Kind::Limo => {
                self.dance_right = !self.dance_right;
                let name = if self.dance_right { "danceRight" } else { "danceLeft" };
                for dancer in self.each(Role::Dancer) {
                    dancer.play(name, true);
                }
                if ctx.rng.bool(10.0) && self.fast_car_can_drive {
                    self.fast_car_drive(ctx);
                }
            }
            Kind::Philly => {
                if !self.train_moving {
                    self.train_cooldown += 1;
                }
                if beat % 4 == 0 {
                    self.light_fade = 0.0;
                    let pick = ctx.rng.int(0, 4) as usize;
                    for (i, light) in self.each(Role::Light).enumerate() {
                        light.visible = i == pick;
                    }
                }
                if beat % 8 == 4 && ctx.rng.bool(30.0) && !self.train_moving && self.train_cooldown > 8 {
                    self.train_cooldown = ctx.rng.int(-4, 0);
                    self.train_start(ctx);
                }
            }
            Kind::Spooky => {
                if ctx.rng.bool(10.0) && beat > self.lightning_beat + self.lightning_offset {
                    let k = ctx.rng.int(1, 2);
                    library::play_sound(ctx, &format!("thunder_{k}"), 1.0);
                    for bg in self.each(Role::Halloween) {
                        bg.play("lightning", false);
                    }
                    self.lightning_beat = beat;
                    self.lightning_offset = ctx.rng.int(8, 24);
                    bf.play_anim("scared", true, false, 0);
                    gf.play_anim("scared", true, false, 0);
                }
            }
            _ => {}
        }
    }

    pub fn draw_back(&self, dl: &mut DrawList, camera: usize) {
        for piece in &self.pieces {
            piece.sprite.draw(dl, camera);
        }
        if let Some(trail) = &self.trail {
            for s in &trail.sprites {
                s.draw(dl, camera);
            }
        }
    }

    pub fn draw_middle(&self, dl: &mut DrawList, camera: usize) {
        if let Some(limo) = &self.limo {
            limo.draw(dl, camera);
        }
    }
}