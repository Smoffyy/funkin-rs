use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use kira::sound::static_sound::StaticSoundData;

use crate::game::library::load_chart;
use crate::game::song::ChartFile;

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct ChartJob {
    id: String,
    folder: PathBuf,
    roots: Vec<PathBuf>,
}

pub enum Job {
    Image(PathBuf),
    Sound(PathBuf),
    Chart(ChartJob),
}

pub enum Loaded {
    Image(PathBuf, Option<DecodedImage>),
    Sound(PathBuf, StaticSoundData),
    Chart(String, ChartFile),
}

fn premultiply(mut bytes: Vec<u8>) -> Vec<u8> {
    for px in bytes.chunks_exact_mut(4) {
        let a = px[3] as u32;
        px[0] = ((px[0] as u32 * a + 127) / 255) as u8;
        px[1] = ((px[1] as u32 * a + 127) / 255) as u8;
        px[2] = ((px[2] as u32 * a + 127) / 255) as u8;
    }
    bytes
}

pub fn decode_image(path: &Path) -> Option<DecodedImage> {
    let image = image::open(path).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    Some(DecodedImage { width, height, pixels: premultiply(image.into_raw()) })
}

fn run(job: Job) -> Option<Loaded> {
    match job {
        Job::Image(path) => {
            let image = decode_image(&path);
            Some(Loaded::Image(path, image))
        }
        Job::Sound(path) => StaticSoundData::from_file(&path).ok().map(|data| Loaded::Sound(path, data)),
        Job::Chart(job) => load_chart(&job.roots, &job.folder, &job.id).map(|chart| Loaded::Chart(job.id, chart)),
    }
}

pub fn start(jobs: Vec<Job>) -> Receiver<Loaded> {
    let queue = Arc::new(Mutex::new(VecDeque::from(jobs)));
    let (tx, rx) = channel();
    let workers = thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1).clamp(1, 4));
    for _ in 0..workers {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        thread::spawn(move || loop {
            let job = queue.lock().unwrap().pop_front();
            let Some(job) = job else { return };
            if let Some(loaded) = run(job) {
                if tx.send(loaded).is_err() {
                    return;
                }
            }
        });
    }
    rx
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, ext, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
            out.push(path);
        }
    }
}

fn folder_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
    entries.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect()
}

pub fn jobs(roots: &[PathBuf], charts_dir: &Path) -> Vec<Job> {
    let mut ids = folder_names(charts_dir);
    let mut sounds = Vec::new();
    let mut images = Vec::new();
    let mut music = Vec::new();
    for root in roots {
        ids.extend(folder_names(&root.join("data")));
        files(&root.join("sounds"), "ogg", &mut sounds);
        files(&root.join("images"), "png", &mut images);
        files(&root.join("music"), "ogg", &mut music);
    }
    let mut song_audio = Vec::new();
    files(charts_dir, "ogg", &mut song_audio);
    ids.sort();
    ids.dedup();
    let mut jobs = Vec::new();
    jobs.extend(ids.into_iter().map(|id| Job::Chart(ChartJob { folder: charts_dir.join(&id), id, roots: roots.to_vec() })));
    jobs.extend(sounds.into_iter().map(Job::Sound));
    jobs.extend(images.into_iter().map(Job::Image));
    jobs.extend(music.into_iter().map(Job::Sound));
    jobs.extend(song_audio.into_iter().map(Job::Sound));
    jobs
}