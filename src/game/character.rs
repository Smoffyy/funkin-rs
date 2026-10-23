use std::collections::HashMap;
use std::rc::Rc;

use serde_json::Value;

use crate::engine::app::Ctx;
use crate::engine::gfx::DrawList;
use crate::engine::sprite::Sprite;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Dance {
    Idle,
    LeftRight,
    None,
}

#[derive(Clone, Debug)]
struct AnimDef {
    name: String,
    prefix: String,
    offset: (f64, f64),
    indices: Option<Vec<i64>>,
    looped: bool,
    fps: f64,
}

#[derive(Clone, Debug)]
pub struct CharDef {
    pub image: String,
    pub antialiasing: bool,
    pub scale: f64,
    pub flip_x: bool,
    pub player_swaps: bool,
    pub dance: Dance,
    pub sing_duration: f64,
    pub idle_loop: Option<String>,
    pub finish_to: HashMap<String, String>,
    pub hitbox_shrink: (f64, f64),
    animations: Vec<AnimDef>,
}

fn table(ctx: &mut Ctx) -> Rc<Value> {
    if let Some(v) = ctx.data.characters.clone() {
        return v;
    }
    let text = ctx.assets.read_text("data/characters.json").unwrap_or_else(|| "{}".to_string());
    let value = Rc::new(serde_json::from_str(&text).unwrap_or(Value::Null));
    ctx.data.characters = Some(value.clone());
    value
}

impl CharDef {
    pub fn load(ctx: &mut Ctx, name: &str) -> CharDef {
        let all = table(ctx);
        let v = all.get(name).or_else(|| all.get("bf")).cloned().unwrap_or(Value::Null);
        let flag = |key: &str, default: bool| v.get(key).and_then(|x| x.as_bool()).unwrap_or(default);
        let string = |key: &str| v.get(key).and_then(|x| x.as_str()).map(str::to_string);
        let animations = v
            .get("animations")
            .and_then(|a| a.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|a| a.as_array())
                    .map(|a| AnimDef {
                        name: a.first().and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        prefix: a.get(1).and_then(|x| x.as_str()).unwrap_or("").to_string(),
                        offset: (a.get(2).and_then(|x| x.as_f64()).unwrap_or(0.0), a.get(3).and_then(|x| x.as_f64()).unwrap_or(0.0)),
                        indices: a.get(4).and_then(|x| x.as_array()).map(|i| i.iter().filter_map(|n| n.as_i64()).collect()),
                        looped: a.get(5).and_then(|x| x.as_bool()).unwrap_or(false),
                        fps: a.get(6).and_then(|x| x.as_f64()).unwrap_or(24.0),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let dance = match v.get("dance").and_then(|x| x.as_str()).unwrap_or("idle") {
            "leftright" => Dance::LeftRight,
            "none" => Dance::None,
            _ => Dance::Idle,
        };
        let shrink = v.get("hitbox_shrink").and_then(|x| x.as_array());
        let shrink_at = |i: usize| shrink.and_then(|s| s.get(i)).and_then(|x| x.as_f64()).unwrap_or(0.0);
        CharDef {
            image: string("image").unwrap_or_else(|| "BOYFRIEND".to_string()),
            antialiasing: flag("antialiasing", true),
            scale: v.get("scale").and_then(|x| x.as_f64()).unwrap_or(1.0),
            flip_x: flag("flip_x", false),
            player_swaps: flag("player_swaps", true),
            dance,
            sing_duration: v.get("sing_duration").and_then(|x| x.as_f64()).unwrap_or(4.0),
            idle_loop: string("idle_loop"),
            finish_to: v
                .get("finish_to")
                .and_then(|x| x.as_object())
                .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
                .unwrap_or_default(),
            hitbox_shrink: (shrink_at(0), shrink_at(1)),
            animations,
        }
    }
}

pub struct Character {
    pub sprite: Sprite,
    pub name: String,
    pub def: CharDef,
    offsets: HashMap<String, (f64, f64)>,
    pub is_player: bool,
    pub hold_timer: f64,
    pub debug_mode: bool,
    pub started_death: bool,
    danced: bool,
}

impl Character {
    pub fn new(ctx: &mut Ctx, x: f64, y: f64, name: &str, is_player: bool) -> Character {
        let def = CharDef::load(ctx, name);
        let frames = ctx.atlas(&def.image);
        let mut sprite = Sprite::with_frames(x, y, frames.clone());
        sprite.antialiasing = true;
        let mut offsets = HashMap::new();
        for a in &def.animations {
            match &a.indices {
                Some(indices) => sprite.anim.add_by_indices(&frames, &a.name, &a.prefix, indices, "", a.fps, a.looped, false),
                None => sprite.anim.add_by_prefix(&frames, &a.name, &a.prefix, a.fps, a.looped, false),
            }
            offsets.insert(a.name.clone(), a.offset);
        }
        let mut c = Character {
            sprite,
            name: name.to_string(),
            def,
            offsets,
            is_player,
            hold_timer: 0.0,
            debug_mode: false,
            started_death: false,
            danced: false,
        };
        if c.def.scale != 1.0 {
            let w = c.sprite.width;
            c.sprite.set_graphic_size((w * c.def.scale) as i64, 0);
            c.sprite.update_hitbox();
        }
        c.sprite.antialiasing = c.def.antialiasing;
        if c.def.dance == Dance::None {
            if let Some(first) = c.def.animations.first().map(|a| a.name.clone()) {
                c.play_anim(&first, false, false, 0);
            }
        }
        c.sprite.width -= c.def.hitbox_shrink.0;
        c.sprite.height -= c.def.hitbox_shrink.1;
        c.sprite.flip_x = c.def.flip_x;
        c.dance();
        c.sprite.anim.finish();
        if is_player {
            c.sprite.flip_x = !c.sprite.flip_x;
            if c.def.player_swaps {
                c.sprite.anim.swap_frames("singRIGHT", "singLEFT");
                if c.sprite.anim.exists("singRIGHTmiss") {
                    c.sprite.anim.swap_frames("singRIGHTmiss", "singLEFTmiss");
                }
            }
        }
        c.sprite.anim.clear_finished();
        c
    }

    pub fn anim_name(&self) -> &str {
        self.sprite.anim.name()
    }

    pub fn has_anim(&self, name: &str) -> bool {
        self.sprite.anim.exists(name)
    }

    pub fn update(&mut self, ctx: &Ctx) {
        let elapsed = ctx.elapsed;
        if self.is_player {
            if !self.debug_mode {
                if self.anim_name().starts_with("sing") {
                    self.hold_timer += elapsed;
                } else {
                    self.hold_timer = 0.0;
                }
                if self.anim_name().ends_with("miss") && self.sprite.anim.finished() {
                    self.play_anim("idle", true, false, 10);
                }
                if self.anim_name() == "firstDeath" && self.sprite.anim.finished() && self.started_death {
                    self.play_anim("deathLoop", false, false, 0);
                }
            }
        } else if !self.name.starts_with("bf") {
            if self.anim_name().starts_with("sing") {
                self.hold_timer += elapsed;
            }
            if self.hold_timer >= ctx.data.conductor.step_crochet * self.def.sing_duration * 0.001 {
                self.dance();
                self.hold_timer = 0.0;
            }
        }
        if let Some(idle) = self.def.idle_loop.clone() {
            if !self.anim_name().starts_with("sing") && self.sprite.anim.finished() {
                self.play_anim(&idle, false, false, 0);
            }
        }
        if let Some(next) = self.def.finish_to.get(self.anim_name()).cloned() {
            if self.sprite.anim.finished() {
                self.play_anim(&next, false, false, 0);
            }
        }
        self.sprite.update(elapsed);
    }

    pub fn dance(&mut self) {
        if self.debug_mode {
            return;
        }
        match self.def.dance {
            Dance::LeftRight => {
                if !self.anim_name().starts_with("hair") {
                    self.danced = !self.danced;
                    if self.danced {
                        self.play_anim("danceRight", false, false, 0);
                    } else {
                        self.play_anim("danceLeft", false, false, 0);
                    }
                }
            }
            Dance::Idle => self.play_anim("idle", false, false, 0),
            Dance::None => {}
        }
    }

    pub fn play_anim(&mut self, name: &str, force: bool, reversed: bool, frame: i64) {
        self.sprite.anim.play(name, force, reversed, frame);
        let offset = self.offsets.get(name).copied().unwrap_or((0.0, 0.0));
        self.sprite.offset.set(offset.0, offset.1);
        if self.name == "gf" {
            match name {
                "singLEFT" => self.danced = true,
                "singRIGHT" => self.danced = false,
                "singUP" | "singDOWN" => self.danced = !self.danced,
                _ => {}
            }
        }
    }

    pub fn draw(&self, dl: &mut DrawList, camera: usize) {
        self.sprite.draw(dl, camera);
    }
}

pub fn icon_frames(ctx: &mut Ctx, name: &str) -> Vec<usize> {
    if name == "bf-old" {
        return vec![14, 15];
    }
    table(ctx)
        .get(name)
        .and_then(|v| v.get("icon_frames"))
        .and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|n| n.as_u64()).map(|n| n as usize).collect())
        .unwrap_or_else(|| vec![10, 11])
}

pub fn character_names(ctx: &mut Ctx) -> Vec<String> {
    table(ctx).as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default()
}