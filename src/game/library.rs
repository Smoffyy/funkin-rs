use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use kira::sound::static_sound::StaticSoundData;

use crate::engine::app::Ctx;

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

pub fn chart_file(ctx: &mut Ctx, song: &str) -> Option<ChartFile> {
    let id = song_id(song);
    let path = chart_folder(ctx, &id).join("chart.json");
    if let Some(chart) = fs::read_to_string(&path).ok().and_then(|text| ChartFile::parse(&id, &text)) {
        return Some(chart);
    }
    let files = [0, 1, 2].map(|d| ctx.assets.read_text(&format!("data/{id}/{}.json", legacy_file(&id, d))));
    ChartFile::from_legacy(&id, files)
}

pub fn save_chart(ctx: &Ctx, chart: &ChartFile) -> std::io::Result<PathBuf> {
    let folder = chart_folder(ctx, &chart.id);
    fs::create_dir_all(&folder)?;
    let path = folder.join("chart.json");
    fs::write(&path, chart.write())?;
    Ok(path)
}

pub fn custom_songs(ctx: &Ctx) -> Vec<ChartFile> {
    let Ok(entries) = fs::read_dir(&ctx.data.charts_dir) else { return vec![] };
    let mut songs: Vec<ChartFile> = entries
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let text = fs::read_to_string(e.path().join("chart.json")).ok()?;
            ChartFile::parse(&id, &text)
        })
        .collect();
    songs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    songs
}

pub fn chart(ctx: &mut Ctx, song: &str, difficulty: &str) -> Option<Rc<Song>> {
    chart_file(ctx, song)?.song(difficulty).map(Rc::new)
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