pub mod alphabet;
pub mod character;
pub mod conductor;
pub mod data;
pub mod dialogue;
pub mod editor;
pub mod freeplay;
pub mod game_over;
pub mod gitaroo;
pub mod library;
pub mod main_menu;
pub mod note;
pub mod options;
pub mod pause;
pub mod play;
pub mod song;
pub mod stage;
pub mod story;
pub mod title;
pub mod weeks;
pub mod widgets;

use std::path::PathBuf;

pub use data::GameData;

pub fn msaa_samples() -> i32 {
    data::Prefs::load(&crate::engine::save::Save::open("funkin-rs")).msaa as i32
}

use crate::engine::app::{App, Ctx, State};
use crate::engine::assets::Assets;

use conductor::Conductor;
use data::{Controls, Fonts, Highscore, Prefs, Session};

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|exe| exe.parent().map(PathBuf::from))
}

fn charts_dir() -> PathBuf {
    match exe_dir().filter(|dir| dir.join("assets").exists()) {
        Some(dir) => dir.join("charts"),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("charts"),
    }
}

fn content_roots() -> Vec<PathBuf> {
    let mut bases = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))];
    if let Some(dir) = exe_dir() {
        bases.push(dir);
    }
    let mut roots = Vec::new();
    for base in bases {
        for name in ["content", "assets"] {
            let root = base.join(name);
            if root.exists() && !roots.contains(&root) {
                roots.push(root);
            }
        }
    }
    if let Ok(extra) = std::env::var("FUNKIN_CONTENT") {
        for part in std::env::split_paths(&extra) {
            if part.exists() {
                roots.push(part);
            }
        }
    }
    roots
}

pub fn boot() -> App {
    let charts = charts_dir();
    let mut assets = Assets::new(content_roots());
    assets.start_preload(&charts);
    App::new(
        assets,
        |assets| {
            let fonts = Fonts { ui: assets.font("fonts/VT323-Regular.ttf") };
            let save = crate::engine::save::Save::open("funkin-rs");
            let mut controls = Controls::defaults();
            controls.load(&save);
            GameData {
                fonts,
                prefs: Prefs::load(&save),
                controls,
                scores: Highscore::load(&save),
                session: Session::default(),
                conductor: Conductor::default(),
                title_initialized: false,
                characters: None,
                charts_dir: charts,
            }
        },
        Box::new(|ctx: &mut Ctx| {
            ctx.show_fps = ctx.data.prefs.fps_counter;
            Box::new(title::Title::new(ctx)) as Box<dyn State>
        }),
    )
}