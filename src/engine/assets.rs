use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use fontdue::{Font, FontSettings};

use super::atlas::Frames;
use super::audio::Audio;
use super::gfx::{Gfx, Texture};
use super::preload::{self, DecodedImage, Job, Loaded};
use super::text::enable_glyph_cache;
use crate::game::song::{ChartFile, Song};

const UPLOAD_BUDGET: Duration = Duration::from_millis(4);

pub fn find_in(roots: &[PathBuf], rel: &str) -> Option<PathBuf> {
    roots.iter().rev().map(|r| r.join(rel)).find(|p| p.exists())
}

pub fn read_text_in(roots: &[PathBuf], rel: &str) -> Option<String> {
    let path = find_in(roots, rel)?;
    fs::read_to_string(path).ok().map(|s| s.trim_start_matches('\u{feff}').to_string())
}

pub struct Assets {
    pub roots: Vec<PathBuf>,
    textures: HashMap<PathBuf, Texture>,
    frames: HashMap<String, Rc<Frames>>,
    fonts: HashMap<String, Rc<Font>>,
    solids: HashMap<u32, Texture>,
    fallback_font: Option<Rc<Font>>,
    pub charts: HashMap<String, Rc<ChartFile>>,
    pub songs: HashMap<(String, String), Rc<Song>>,
    loads: Option<Receiver<Loaded>>,
    images_pending: usize,
    caching: bool,
}

impl Assets {
    pub fn new(roots: Vec<PathBuf>) -> Assets {
        Assets {
            roots,
            textures: HashMap::new(),
            frames: HashMap::new(),
            fonts: HashMap::new(),
            solids: HashMap::new(),
            fallback_font: None,
            charts: HashMap::new(),
            songs: HashMap::new(),
            loads: None,
            images_pending: 0,
            caching: false,
        }
    }

    pub fn enable_caching(&mut self, charts_dir: &Path) {
        let jobs = preload::jobs(&self.roots, charts_dir);
        self.images_pending = jobs.iter().filter(|job| matches!(job, Job::Image(_))).count();
        self.caching = true;
        enable_glyph_cache();
        self.loads = Some(preload::start(jobs));
    }

    pub fn cached_chart(&self, id: &str) -> Option<Rc<ChartFile>> {
        if self.caching { self.charts.get(id).cloned() } else { None }
    }

    pub fn store_chart(&mut self, id: String, chart: Rc<ChartFile>) {
        if self.caching {
            self.charts.insert(id, chart);
        }
    }

    pub fn cached_song(&self, key: &(String, String)) -> Option<Rc<Song>> {
        if self.caching { self.songs.get(key).cloned() } else { None }
    }

    pub fn store_song(&mut self, key: (String, String), song: Rc<Song>) {
        if self.caching {
            self.songs.insert(key, song);
        }
    }

    pub fn warm(&mut self, gfx: &mut Gfx, audio: &mut Audio) {
        while self.images_pending > 0 {
            let Some(rx) = self.loads.as_ref() else { return };
            let Ok(loaded) = rx.recv() else { return };
            self.accept(gfx, audio, loaded);
        }
    }

    pub fn pump(&mut self, gfx: &mut Gfx, audio: &mut Audio) {
        let start = Instant::now();
        while start.elapsed() < UPLOAD_BUDGET {
            let Some(rx) = self.loads.as_ref() else { return };
            let Ok(loaded) = rx.try_recv() else { return };
            self.accept(gfx, audio, loaded);
        }
    }

    fn accept(&mut self, gfx: &mut Gfx, audio: &mut Audio, loaded: Loaded) {
        match loaded {
            Loaded::Image(path, image) => {
                self.images_pending -= 1;
                if let Some(image) = image {
                    self.upload(gfx, path, image);
                }
            }
            Loaded::Sound(path, data) => audio.store(path, data),
            Loaded::Chart(id, chart) => {
                self.charts.entry(id).or_insert_with(|| Rc::new(chart));
            }
        }
    }

    fn upload(&mut self, gfx: &mut Gfx, path: PathBuf, image: DecodedImage) {
        if self.textures.contains_key(&path) {
            return;
        }
        let texture = gfx.create_texture(image.width, image.height, &image.pixels);
        self.textures.insert(path, texture);
    }

    pub fn find(&self, rel: &str) -> Option<PathBuf> {
        find_in(&self.roots, rel)
    }

    pub fn find_ignore_case(&self, rel: &str) -> Option<PathBuf> {
        let path = Path::new(rel);
        let dir = path.parent()?.to_string_lossy().to_string();
        let wanted = path.file_name()?.to_string_lossy().to_lowercase();
        let name = self.list_dir(&dir).into_iter().find(|n| n.to_lowercase() == wanted)?;
        self.find(&format!("{dir}/{name}"))
    }

    pub fn read_text(&self, rel: &str) -> Option<String> {
        read_text_in(&self.roots, rel)
    }

    pub fn list_dir(&self, rel: &str) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for root in &self.roots {
            if let Ok(entries) = fs::read_dir(root.join(rel)) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
        }
        names.sort();
        names
    }

    fn load_texture_file(&mut self, gfx: &mut Gfx, path: &Path) -> Option<Texture> {
        if let Some(t) = self.textures.get(path) {
            return Some(*t);
        }
        let image = preload::decode_image(path)?;
        let texture = gfx.create_texture(image.width, image.height, &image.pixels);
        self.textures.insert(path.to_path_buf(), texture);
        Some(texture)
    }

    pub fn registered(&self, key: &str) -> Option<Rc<Frames>> {
        self.frames.get(key).cloned()
    }

    pub fn register(&mut self, key: &str, frames: Frames) -> Rc<Frames> {
        let rc = Rc::new(frames);
        self.frames.insert(key.to_string(), rc.clone());
        rc
    }

    pub fn missing(&mut self, gfx: &mut Gfx) -> Rc<Frames> {
        if let Some(f) = self.frames.get("@missing") {
            return f.clone();
        }
        let frames = self.solid(gfx, 32.0, 32.0, 0xFFFF00FF);
        self.frames.insert("@missing".to_string(), frames.clone());
        frames
    }

    pub fn image(&mut self, gfx: &mut Gfx, key: &str) -> Rc<Frames> {
        if let Some(f) = self.frames.get(key) {
            return f.clone();
        }
        let rel = format!("images/{key}.png");
        match self.find(&rel).and_then(|p| self.load_texture_file(gfx, &p)) {
            Some(t) => self.register(key, Frames::single(t)),
            None => self.missing(gfx),
        }
    }

    pub fn sparrow(&mut self, gfx: &mut Gfx, key: &str) -> Rc<Frames> {
        if let Some(f) = self.frames.get(key) {
            return f.clone();
        }
        let png = format!("images/{key}.png");
        let xml = format!("images/{key}.xml");
        let texture = self.find(&png).and_then(|p| self.load_texture_file(gfx, &p));
        let text = self.read_text(&xml);
        match (texture, text) {
            (Some(t), Some(x)) => match Frames::sparrow(t, &x) {
                Ok(frames) => self.register(key, frames),
                Err(_) => self.missing(gfx),
            },
            (Some(t), None) => self.register(key, Frames::single(t)),
            _ => self.missing(gfx),
        }
    }

    pub fn packer(&mut self, gfx: &mut Gfx, key: &str) -> Rc<Frames> {
        if let Some(f) = self.frames.get(key) {
            return f.clone();
        }
        let png = format!("images/{key}.png");
        let txt = format!("images/{key}.txt");
        let texture = self.find(&png).and_then(|p| self.load_texture_file(gfx, &p));
        match (texture, self.read_text(&txt)) {
            (Some(t), Some(text)) => self.register(key, Frames::packer(t, &text)),
            (Some(t), None) => self.register(key, Frames::single(t)),
            _ => self.missing(gfx),
        }
    }

    pub fn atlas(&mut self, gfx: &mut Gfx, key: &str) -> Rc<Frames> {
        if let Some(f) = self.frames.get(key) {
            return f.clone();
        }
        if self.find(&format!("images/{key}.xml")).is_some() {
            self.sparrow(gfx, key)
        } else if self.find(&format!("images/{key}.txt")).is_some() {
            self.packer(gfx, key)
        } else {
            self.image(gfx, key)
        }
    }

    pub fn grid(&mut self, gfx: &mut Gfx, key: &str, tile_width: u32, tile_height: u32) -> Rc<Frames> {
        let grid_key = format!("{key}@{tile_width}x{tile_height}");
        if let Some(f) = self.frames.get(&grid_key) {
            return f.clone();
        }
        let single = self.image(gfx, key);
        let frames = Frames::grid(single.texture, tile_width, tile_height);
        self.register(&grid_key, frames)
    }

    pub fn solid(&mut self, gfx: &mut Gfx, width: f64, height: f64, color: u32) -> Rc<Frames> {
        let texture = match self.solids.get(&color) {
            Some(t) => *t,
            None => {
                let a = (color >> 24) & 0xFF;
                let pm = |c: u32| ((c * a + 127) / 255) as u8;
                let bytes = [pm((color >> 16) & 0xFF), pm((color >> 8) & 0xFF), pm(color & 0xFF), a as u8];
                let t = gfx.create_texture(1, 1, &bytes);
                self.solids.insert(color, t);
                t
            }
        };
        Rc::new(Frames::solid(texture, width.trunc(), height.trunc()))
    }

    pub fn font(&mut self, rel: &str) -> Rc<Font> {
        if let Some(f) = self.fonts.get(rel) {
            return f.clone();
        }
        let loaded = self
            .find(rel)
            .or_else(|| Some(PathBuf::from(rel)).filter(|p| p.exists()))
            .and_then(|p| fs::read(p).ok())
            .and_then(|bytes| Font::from_bytes(bytes, FontSettings::default()).ok())
            .map(Rc::new);
        match loaded {
            Some(f) => {
                self.fonts.insert(rel.to_string(), f.clone());
                f
            }
            None => self.fallback_font(),
        }
    }

    pub fn fallback_font(&mut self) -> Rc<Font> {
        if let Some(f) = &self.fallback_font {
            return f.clone();
        }
        let candidates = [
            "C:/Windows/Fonts/arialbd.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ];
        let bundled = self.find("fonts/VT323-Regular.ttf");
        for path in candidates.into_iter().map(PathBuf::from).chain(bundled) {
            if let Some(f) = fs::read(path).ok().and_then(|b| Font::from_bytes(b, FontSettings::default()).ok()) {
                let f = Rc::new(f);
                self.fallback_font = Some(f.clone());
                return f;
            }
        }
        panic!("no usable font found; put a .ttf in content/fonts");
    }
}