use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use kira::sound::static_sound::StaticSoundData;
use kira::Frame;

use crate::engine::app::Ctx;
use crate::engine::assets::read_text_in;

use super::song::{song_id, ChartFile, Song};

pub fn chart_folder(ctx: &Ctx, id: &str) -> PathBuf {
    ctx.data.charts_dir.join(song_id(id))
}

fn legacy_file(id: &str, difficulty: usize) -> String {
    match difficulty {
        0 => format!("{id}-easy"),
        2 => format!("{id}-hard"),
        _ => id.to_string(),
    }
}

pub fn load_chart(roots: &[PathBuf], folder: &Path, id: &str) -> Option<ChartFile> {
    if let Some(chart) = fs::read_to_string(folder.join("chart.json")).ok().and_then(|text| ChartFile::parse(id, &text)) {
        return Some(chart);
    }
    let files = [0, 1, 2].map(|d| read_text_in(roots, &format!("data/{id}/{}.json", legacy_file(id, d))));
    ChartFile::from_legacy(id, files)
}

pub fn chart_file(ctx: &mut Ctx, song: &str) -> Option<Rc<ChartFile>> {
    let id = song_id(song);
    if let Some(chart) = ctx.assets.cached_chart(&id) {
        return Some(chart);
    }
    let chart = Rc::new(load_chart(&ctx.assets.roots, &chart_folder(ctx, &id), &id)?);
    ctx.assets.store_chart(id, chart.clone());
    Some(chart)
}

pub fn save_chart(ctx: &mut Ctx, chart: &ChartFile) -> std::io::Result<PathBuf> {
    let id = song_id(&chart.id);
    let folder = chart_folder(ctx, &id);
    fs::create_dir_all(&folder)?;
    let path = folder.join("chart.json");
    fs::write(&path, chart.write())?;
    ctx.assets.songs.retain(|(song, _), _| *song != id);
    ctx.assets.store_chart(id, Rc::new(chart.clone()));
    Ok(path)
}

pub fn custom_songs(ctx: &mut Ctx) -> Vec<Rc<ChartFile>> {
    let Ok(entries) = fs::read_dir(&ctx.data.charts_dir) else { return vec![] };
    let ids: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().join("chart.json").exists())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let mut songs: Vec<Rc<ChartFile>> = ids.iter().filter_map(|id| chart_file(ctx, id)).collect();
    songs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    songs
}

pub fn chart(ctx: &mut Ctx, song: &str, difficulty: &str) -> Option<Rc<Song>> {
    let key = (song_id(song), difficulty.to_string());
    if let Some(built) = ctx.assets.cached_song(&key) {
        return Some(built);
    }
    let built = Rc::new(chart_file(ctx, song)?.song(difficulty)?);
    ctx.assets.store_song(key, built.clone());
    Some(built)
}

fn audio_file(ctx: &mut Ctx, rel: &str) -> Option<StaticSoundData> {
    let file = format!("{rel}.ogg");
    let path = ctx.assets.find(&file).or_else(|| ctx.assets.find_ignore_case(&file))?;
    ctx.audio.load(&path)
}

fn song_audio(ctx: &mut Ctx, id: &str, name: &str, kind: &str, file: Option<&str>) -> Option<StaticSoundData> {
    let folder = chart_folder(ctx, id);
    if let Some(path) = file.map(|f| folder.join(f)).filter(|p| p.exists()) {
        return ctx.audio.load(&path);
    }
    let custom = folder.join(format!("{kind}.ogg"));
    if custom.exists() {
        return ctx.audio.load(&custom);
    }
    audio_file(ctx, &format!("music/{name}_{kind}"))
}

pub fn inst(ctx: &mut Ctx, id: &str, name: &str, file: Option<&str>) -> Option<StaticSoundData> {
    song_audio(ctx, id, name, "Inst", file)
}

pub fn voices(ctx: &mut Ctx, id: &str, name: &str, file: Option<&str>) -> Option<StaticSoundData> {
    song_audio(ctx, id, name, "Voices", file)
}

pub fn split_voices(ctx: &mut Ctx, id: &str, file: Option<&str>) -> Option<StaticSoundData> {
    let path = chart_folder(ctx, id).join(file?);
    ctx.audio.load(&path)
}

pub fn mixed_voices(ctx: &mut Ctx, id: &str, player: Option<&str>, opponent: Option<&str>) -> Option<StaticSoundData> {
    let folder = chart_folder(ctx, id);
    let mut tracks = [player, opponent].into_iter().flatten().filter_map(|file| ctx.audio.load(&folder.join(file)));
    let first = tracks.next()?;
    Some(tracks.fold(first, |mix, track| add_tracks(mix, &track)))
}

fn add_tracks(mix: StaticSoundData, track: &StaticSoundData) -> StaticSoundData {
    let silent = Frame { left: 0.0, right: 0.0 };
    let len = mix.frames.len().max(track.frames.len());
    let frames: Arc<[Frame]> = (0..len)
        .map(|i| {
            let a = mix.frames.get(i).copied().unwrap_or(silent);
            let b = track.frames.get(i).copied().unwrap_or(silent);
            Frame { left: a.left + b.left, right: a.right + b.right }
        })
        .collect();
    StaticSoundData { frames, slice: None, ..mix }
}

pub fn sound(ctx: &mut Ctx, name: &str) -> Option<StaticSoundData> {
    audio_file(ctx, &format!("sounds/{name}"))
}

pub fn music(ctx: &mut Ctx, name: &str) -> Option<StaticSoundData> {
    audio_file(ctx, &format!("music/{name}"))
}

pub fn play_sound(ctx: &mut Ctx, name: &str, volume: f64) {
    let data = sound(ctx, name);
    ctx.audio.play(data, volume);
}

pub fn play_music(ctx: &mut Ctx, name: &str, volume: f64, looped: bool) {
    let data = music(ctx, name);
    ctx.audio.play_music(data, volume, looped);
}