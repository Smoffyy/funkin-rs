use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::sound::PlaybackState;
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Tween};

fn instant() -> Tween {
    Tween { duration: Duration::ZERO, ..Default::default() }
}

fn decibels(amplitude: f64) -> Decibels {
    if amplitude <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * amplitude.log10()) as f32)
    }
}

fn beep() -> StaticSoundData {
    let rate = 44100;
    let duration = 0.08;
    let length = (duration * rate as f32) as usize;
    let frames: Arc<[Frame]> = (0..length)
        .map(|i| {
            let t = i as f32 / rate as f32;
            let square = if (t * 1046.5).fract() < 0.5 { 0.25 } else { -0.25 };
            let v = square * (1.0 - t / duration).max(0.0) * (t * 400.0).min(1.0);
            Frame { left: v, right: v }
        })
        .collect();
    StaticSoundData { sample_rate: rate, frames, settings: StaticSoundSettings::default(), slice: None }
}

#[derive(Clone, Copy, Debug)]
struct Fade {
    from: f64,
    to: f64,
    duration: f64,
    time: f64,
}

pub struct Sound {
    data: Option<StaticSoundData>,
    handle: Option<StaticSoundHandle>,
    volume: f64,
    applied_volume: f64,
    pub looped: bool,
    paused_at: Option<f64>,
    stored_time: f64,
    playing: bool,
    completed: bool,
    fade: Option<Fade>,
}

impl Default for Sound {
    fn default() -> Self {
        Sound::empty()
    }
}

impl Sound {
    pub fn empty() -> Sound {
        Sound {
            data: None,
            handle: None,
            volume: 1.0,
            applied_volume: -1.0,
            looped: false,
            paused_at: None,
            stored_time: 0.0,
            playing: false,
            completed: false,
            fade: None,
        }
    }

    pub fn new(data: Option<StaticSoundData>, volume: f64, looped: bool) -> Sound {
        Sound { data, volume, looped, ..Sound::empty() }
    }

    pub fn length(&self) -> f64 {
        self.data.as_ref().map(|d| d.duration().as_secs_f64() * 1000.0).unwrap_or(0.0)
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn time(&self) -> f64 {
        if let Some(at) = self.paused_at {
            return at;
        }
        match (&self.handle, self.playing) {
            (Some(h), true) => h.position() * 1000.0,
            _ => self.stored_time,
        }
    }

    pub fn play(&mut self, audio: &mut Audio, force_restart: bool, start_ms: f64) {
        if self.playing && !force_restart {
            return;
        }
        if !force_restart {
            if let Some(at) = self.paused_at {
                self.resume_at(at);
                return;
            }
        }
        self.start(audio, start_ms);
    }

    fn resume_at(&mut self, at: f64) {
        self.paused_at = None;
        if let Some(h) = self.handle.as_mut() {
            h.seek_to(at / 1000.0);
            h.resume(instant());
            self.playing = true;
        }
    }

    fn start(&mut self, audio: &mut Audio, start_ms: f64) {
        self.stop();
        let Some(data) = self.data.clone() else { return };
        let mut data = data.start_position((start_ms / 1000.0).max(0.0)).volume(decibels(self.volume));
        if self.looped {
            data = data.loop_region(..);
        }
        self.handle = audio.play_data(data);
        self.applied_volume = self.volume;
        self.playing = self.handle.is_some();
        self.completed = false;
    }

    pub fn pause(&mut self) {
        if !self.playing {
            return;
        }
        let at = self.time();
        if let Some(h) = self.handle.as_mut() {
            h.pause(instant());
        }
        self.paused_at = Some(at);
        self.playing = false;
    }

    pub fn stop(&mut self) {
        if let Some(mut h) = self.handle.take() {
            h.stop(instant());
        }
        self.playing = false;
        self.paused_at = None;
        self.stored_time = 0.0;
    }

    pub fn set_time(&mut self, audio: &mut Audio, ms: f64) {
        if self.playing {
            self.start(audio, ms);
        } else if self.paused_at.is_some() {
            self.paused_at = Some(ms);
        } else {
            self.stored_time = ms;
        }
    }

    pub fn fade_in(&mut self, audio: &mut Audio, duration: f64, from: f64, to: f64) {
        if !self.playing {
            self.play(audio, false, 0.0);
        }
        self.volume = from;
        self.fade = Some(Fade { from, to, duration, time: 0.0 });
    }

    pub fn fade_out(&mut self, duration: f64, to: f64) {
        self.fade = Some(Fade { from: self.volume, to, duration, time: 0.0 });
    }

    pub fn take_completed(&mut self) -> bool {
        std::mem::replace(&mut self.completed, false)
    }

    pub fn update(&mut self, elapsed: f64) {
        if let Some(fade) = self.fade.as_mut() {
            fade.time += elapsed;
            let t = (fade.time / fade.duration.max(1e-9)).min(1.0);
            self.volume = fade.from + (fade.to - fade.from) * t;
            if t >= 1.0 {
                self.fade = None;
            }
        }
        if let Some(h) = self.handle.as_mut() {
            if self.playing && h.state() == PlaybackState::Stopped {
                self.playing = false;
                self.handle = None;
                self.stored_time = 0.0;
                self.completed = true;
                return;
            }
            if self.applied_volume != self.volume {
                h.set_volume(decibels(self.volume), instant());
                self.applied_volume = self.volume;
            }
        }
    }
}

pub struct Audio {
    manager: Option<AudioManager<DefaultBackend>>,
    cache: HashMap<String, Option<StaticSoundData>>,
    one_shots: Vec<Sound>,
    pub volume: f64,
    pub muted: bool,
    applied: f64,
    pub music: Sound,
    beep: StaticSoundData,
}

impl Audio {
    pub fn new() -> Audio {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).ok();
        Audio {
            manager,
            cache: HashMap::new(),
            one_shots: Vec::new(),
            volume: 1.0,
            muted: false,
            applied: -1.0,
            music: Sound::empty(),
            beep: beep(),
        }
    }

    pub fn load(&mut self, path: &Path) -> Option<StaticSoundData> {
        let key = path.to_string_lossy().to_string();
        if let Some(cached) = self.cache.get(&key) {
            return cached.clone();
        }
        let data = StaticSoundData::from_file(path).ok();
        self.cache.insert(key, data.clone());
        data
    }

    pub fn play_beep(&mut self) {
        let data = self.beep.clone();
        self.play(Some(data), 1.0);
    }

    fn play_data(&mut self, data: StaticSoundData) -> Option<StaticSoundHandle> {
        self.manager.as_mut()?.play(data).ok()
    }

    pub fn play(&mut self, data: Option<StaticSoundData>, volume: f64) {
        let mut sound = Sound::new(data, volume, false);
        sound.play(self, true, 0.0);
        if sound.playing() {
            self.one_shots.push(sound);
        }
    }

    pub fn play_tracked(&mut self, data: Option<StaticSoundData>, volume: f64) -> Sound {
        let mut sound = Sound::new(data, volume, false);
        sound.play(self, true, 0.0);
        sound
    }

    pub fn play_music(&mut self, data: Option<StaticSoundData>, volume: f64, looped: bool) {
        let mut music = std::mem::take(&mut self.music);
        music.stop();
        music = Sound::new(data, volume, looped);
        music.play(self, true, 0.0);
        self.music = music;
    }

    pub fn fade_in_music(&mut self, duration: f64, from: f64, to: f64) {
        let mut music = std::mem::take(&mut self.music);
        music.fade_in(self, duration, from, to);
        self.music = music;
    }

    pub fn change_volume(&mut self, amount: f64) {
        self.muted = false;
        self.volume = ((self.volume + amount) * 10.0).round().clamp(0.0, 10.0) / 10.0;
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    pub fn effective_volume(&self) -> f64 {
        if self.muted { 0.0 } else { self.volume }
    }

    pub fn update(&mut self, elapsed: f64) {
        let target = self.effective_volume();
        if target != self.applied {
            if let Some(m) = self.manager.as_mut() {
                m.main_track().set_volume(decibels(target), instant());
            }
            self.applied = target;
        }
        self.music.update(elapsed);
        for s in self.one_shots.iter_mut() {
            s.update(elapsed);
        }
        self.one_shots.retain(|s| s.playing());
    }

    pub fn stop_one_shots(&mut self) {
        for s in self.one_shots.iter_mut() {
            s.stop();
        }
        self.one_shots.clear();
    }
}