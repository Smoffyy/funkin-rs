use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use fontdue::Font;
use miniquad::KeyCode;
use serde_json::{json, Map, Value};

use crate::engine::app::Ctx;
use crate::engine::input::{key_from_name, key_name};
use crate::engine::math::Point;
use crate::engine::save::Save;

use super::conductor::Conductor;
use super::song::Song;

pub struct Fonts {
    pub ui: Rc<Font>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Control {
    NoteLeft,
    NoteDown,
    NoteUp,
    NoteRight,
    UiUp,
    UiLeft,
    UiRight,
    UiDown,
    Reset,
    Accept,
    Back,
    Pause,
}

impl Control {
    pub const ALL: [Control; 12] = [
        Control::NoteLeft,
        Control::NoteDown,
        Control::NoteUp,
        Control::NoteRight,
        Control::UiUp,
        Control::UiLeft,
        Control::UiRight,
        Control::UiDown,
        Control::Reset,
        Control::Accept,
        Control::Back,
        Control::Pause,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Control::NoteLeft => "NOTE_LEFT",
            Control::NoteDown => "NOTE_DOWN",
            Control::NoteUp => "NOTE_UP",
            Control::NoteRight => "NOTE_RIGHT",
            Control::UiUp => "UI_UP",
            Control::UiLeft => "UI_LEFT",
            Control::UiRight => "UI_RIGHT",
            Control::UiDown => "UI_DOWN",
            Control::Reset => "RESET",
            Control::Accept => "ACCEPT",
            Control::Back => "BACK",
            Control::Pause => "PAUSE",
        }
    }
}

pub struct Controls {
    binds: HashMap<Control, Vec<KeyCode>>,
}

impl Controls {
    pub fn defaults() -> Controls {
        use KeyCode::*;
        let mut binds = HashMap::new();
        binds.insert(Control::UiUp, vec![W, Up]);
        binds.insert(Control::UiDown, vec![S, Down]);
        binds.insert(Control::UiLeft, vec![A, Left]);
        binds.insert(Control::UiRight, vec![D, Right]);
        binds.insert(Control::NoteUp, vec![W, Up]);
        binds.insert(Control::NoteDown, vec![S, Down]);
        binds.insert(Control::NoteLeft, vec![A, Left]);
        binds.insert(Control::NoteRight, vec![D, Right]);
        binds.insert(Control::Accept, vec![Z, Space, Enter]);
        binds.insert(Control::Back, vec![X, Backspace, Escape]);
        binds.insert(Control::Pause, vec![P, Enter, Escape]);
        binds.insert(Control::Reset, vec![R]);
        Controls { binds }
    }

    pub fn keys(&self, control: Control) -> &[KeyCode] {
        self.binds.get(&control).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn replace(&mut self, control: Control, old: Option<KeyCode>, new: KeyCode) {
        let list = self.binds.entry(control).or_default();
        match old.and_then(|o| list.iter().position(|k| *k == o)) {
            Some(i) => list[i] = new,
            None => list.push(new),
        }
    }

    pub fn load(&mut self, save: &Save) {
        let Some(map) = save.get("controls").and_then(|v| v.as_object()) else { return };
        for control in Control::ALL {
            if let Some(list) = map.get(control.name()).and_then(|v| v.as_array()) {
                let keys: Vec<KeyCode> = list.iter().filter_map(|k| k.as_str().and_then(key_from_name)).collect();
                if !keys.is_empty() {
                    self.binds.insert(control, keys);
                }
            }
        }
    }

    pub fn store(&self, save: &mut Save) {
        let mut map = Map::new();
        for control in Control::ALL {
            let names: Vec<Value> = self.keys(control).iter().map(|k| json!(key_name(*k))).collect();
            map.insert(control.name().to_string(), Value::Array(names));
        }
        save.set("controls", Value::Object(map));
        save.flush();
    }
}

#[derive(Clone, Debug)]
pub struct Prefs {
    pub downscroll: bool,
    pub flashing_menu: bool,
    pub camera_zoom: bool,
    pub fps_counter: bool,
    pub auto_pause: bool,
    pub engine_features: bool,
    pub opponent_strums: bool,
    pub ghost_tapping: bool,
    pub quick_transitions: bool,
    pub preload_assets: bool,
    pub offset: f64,
    pub msaa: u32,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { downscroll: false, flashing_menu: true, camera_zoom: true, fps_counter: true, auto_pause: false, engine_features: true, opponent_strums: false, ghost_tapping: false, quick_transitions: false, preload_assets: false, offset: 0.0, msaa: 1 }
    }
}

impl Prefs {
    pub fn load(save: &Save) -> Prefs {
        let mut p = Prefs::default();
        if let Some(map) = save.get("prefs").and_then(|v| v.as_object()) {
            let flag = |key: &str, default: bool| map.get(key).and_then(|v| v.as_bool()).unwrap_or(default);
            p.downscroll = flag("downscroll", p.downscroll);
            p.flashing_menu = flag("flashing-menu", p.flashing_menu);
            p.camera_zoom = flag("camera-zoom", p.camera_zoom);
            p.fps_counter = flag("fps-counter", p.fps_counter);
            p.auto_pause = flag("auto-pause", p.auto_pause);
            p.engine_features = flag("engine-features", p.engine_features);
            p.opponent_strums = flag("opponent-strums", p.opponent_strums);
            p.ghost_tapping = flag("ghost-tapping", p.ghost_tapping);
            p.quick_transitions = flag("quick-transitions", p.quick_transitions);
            p.preload_assets = flag("preload-assets", p.preload_assets);
            p.offset = map.get("offset").and_then(|v| v.as_f64()).unwrap_or(p.offset);
            p.msaa = map.get("msaa").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(p.msaa);
        }
        p
    }

    pub fn store(&self, save: &mut Save) {
        save.set(
            "prefs",
            json!({
                "downscroll": self.downscroll,
                "flashing-menu": self.flashing_menu,
                "camera-zoom": self.camera_zoom,
                "fps-counter": self.fps_counter,
                "auto-pause": self.auto_pause,
                "engine-features": self.engine_features,
                "opponent-strums": self.opponent_strums,
                "ghost-tapping": self.ghost_tapping,
                "quick-transitions": self.quick_transitions,
                "preload-assets": self.preload_assets,
                "offset": self.offset,
                "msaa": self.msaa,
            }),
        );
        save.flush();
    }

    pub fn opponent_strums_active(&self) -> bool {
        self.engine_features && self.opponent_strums
    }

    pub fn ghost_tapping_active(&self) -> bool {
        self.engine_features && self.ghost_tapping
    }

    pub fn quick_transitions_active(&self) -> bool {
        self.engine_features && self.quick_transitions
    }

    pub fn preload_assets_active(&self) -> bool {
        self.engine_features && self.preload_assets
    }

    pub fn msaa_samples(&self) -> u32 {
        if self.engine_features { self.msaa } else { 1 }
    }
}

#[derive(Default)]
pub struct Highscore {
    scores: HashMap<String, i64>,
}

impl Highscore {
    pub fn load(save: &Save) -> Highscore {
        let mut scores = HashMap::new();
        if let Some(map) = save.get("songScores").and_then(|v| v.as_object()) {
            for (k, v) in map {
                if let Some(n) = v.as_i64() {
                    scores.insert(k.clone(), n);
                }
            }
        }
        Highscore { scores }
    }

    pub fn format_song(song: &str, diff: &str) -> String {
        if diff.is_empty() || diff == "normal" { song.to_string() } else { format!("{song}-{diff}") }
    }

    fn set(&mut self, save: &mut Save, key: String, score: i64) {
        self.scores.insert(key, score);
        let map: Map<String, Value> = self.scores.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        save.set("songScores", Value::Object(map));
        save.flush();
    }

    pub fn save_score(&mut self, save: &mut Save, song: &str, score: i64, diff: &str) {
        let key = Self::format_song(&song.to_lowercase(), diff);
        if self.scores.get(&key).map(|s| *s < score).unwrap_or(true) {
            self.set(save, key, score);
        }
    }

    pub fn save_week_score(&mut self, save: &mut Save, week: usize, score: i64, diff: &str) {
        let key = Self::format_song(&format!("week{week}"), diff);
        if self.scores.get(&key).map(|s| *s < score).unwrap_or(true) {
            self.set(save, key, score);
        }
    }

    pub fn score(&self, song: &str, diff: &str) -> i64 {
        self.scores.get(&Self::format_song(&song.to_lowercase(), diff)).copied().unwrap_or(0)
    }

    pub fn week_score(&self, week: usize, diff: &str) -> i64 {
        self.scores.get(&Self::format_song(&format!("week{week}"), diff)).copied().unwrap_or(0)
    }
}

#[derive(Default)]
pub struct Session {
    pub song: Option<Rc<Song>>,
    pub is_story_mode: bool,
    pub story_week: usize,
    pub story_playlist: Vec<String>,
    pub story_difficulty: usize,
    pub death_counter: u32,
    pub practice_mode: bool,
    pub seen_cutscene: bool,
    pub campaign_score: i64,
    pub prev_cam_follow: Option<Point>,
    pub resume: Option<super::play::Resume>,
    pub editor: Option<Box<super::editor::EditorSession>>,
    pub playtest_start: f64,
}

pub struct GameData {
    pub fonts: Fonts,
    pub prefs: Prefs,
    pub controls: Controls,
    pub scores: Highscore,
    pub session: Session,
    pub conductor: Conductor,
    pub title_initialized: bool,
    pub characters: Option<Rc<Value>>,
    pub charts_dir: PathBuf,
}


pub trait Input {
    fn pressed(&self, control: Control) -> bool;
    fn just_pressed(&self, control: Control) -> bool;
}

impl Input for Ctx {
    fn pressed(&self, control: Control) -> bool {
        self.data.controls.keys(control).iter().any(|k| self.keys.pressed(*k))
    }

    fn just_pressed(&self, control: Control) -> bool {
        self.data.controls.keys(control).iter().any(|k| self.keys.just_pressed(*k))
    }

}