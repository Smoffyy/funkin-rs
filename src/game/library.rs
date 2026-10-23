use std::rc::Rc;

use kira::sound::static_sound::StaticSoundData;

use crate::engine::app::Ctx;

use super::song::{chart_file, Song};

pub fn chart(ctx: &mut Ctx, song: &str, difficulty: usize) -> Option<Rc<Song>> {
    let id = song.to_lowercase();
    let wanted = chart_file(&id, difficulty);
    let text = ctx
        .assets
        .read_text(&format!("data/{id}/{wanted}.json"))
        .or_else(|| ctx.assets.read_text(&format!("data/{id}/{id}.json")))?;
    Song::parse(&id, &text).map(Rc::new)
}

fn audio_file(ctx: &mut Ctx, rel: &str) -> Option<StaticSoundData> {
    let file = format!("{rel}.ogg");
    let path = ctx.assets.find(&file).or_else(|| ctx.assets.find_ignore_case(&file))?;
    ctx.audio.load(&path)
}

pub fn inst(ctx: &mut Ctx, song: &str) -> Option<StaticSoundData> {
    audio_file(ctx, &format!("music/{song}_Inst"))
}

pub fn voices(ctx: &mut Ctx, song: &str) -> Option<StaticSoundData> {
    audio_file(ctx, &format!("music/{song}_Voices"))
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