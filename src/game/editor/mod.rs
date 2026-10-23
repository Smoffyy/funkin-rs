mod panels;
mod preview;
mod ui;
mod waveform;

use std::rc::Rc;

use miniquad::KeyCode;

use crate::engine::app::{Ctx, State};
use crate::engine::atlas::Frames;
use crate::engine::audio::Sound;
use crate::engine::camera::Camera;
use crate::engine::gfx::{DrawList, GAME_HEIGHT};

use super::character::{character_names, icon_frames};
use super::library;
use super::song::{step_at, time_at, ChartEvent, ChartFile, ChartNote, Difficulty, Side, TimingPoint};
use super::stage::Kind;
use preview::Preview;
use ui::{Rect, Ui, ACCENT, BORDER, PANEL, TEXT, TEXT_DIM};
use waveform::Waveform;

const GRID: f64 = 40.0;
const TOP: f64 = 112.0;
const BOTTOM: f64 = 692.0;
const PLAYHEAD: f64 = 300.0;
const EVENT_X: f64 = 290.0;
const OPPONENT_X: f64 = 340.0;
const PLAYER_X: f64 = 510.0;
const GRID_RIGHT: f64 = PLAYER_X + GRID * 4.0;
const SNAPS: [u32; 10] = [4, 8, 12, 16, 20, 24, 32, 48, 64, 192];
const LANE_COLORS: [u32; 4] = [0xFFC24B99, 0xFF00FFFF, 0xFF12FA05, 0xFFF9393F];
const UNDO_LIMIT: usize = 300;
const AUTOSAVE_SECONDS: f64 = 120.0;
pub const EVENT_KINDS: [&str; 5] = ["focus", "play_animation", "camera_bump", "camera_zoom", "gf_speed"];

#[derive(Clone, Copy, PartialEq)]
pub enum Origin {
    Options,
    Freeplay,
}

#[derive(Clone)]
pub struct Settings {
    pub rate: f64,
    pub inst_volume: f64,
    pub voices_volume: f64,
    pub hit_player: bool,
    pub hit_opponent: bool,
    pub metronome: bool,
    pub waveform: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { rate: 1.0, inst_volume: 0.6, voices_volume: 1.0, hit_player: false, hit_opponent: false, metronome: false, waveform: 2 }
    }
}

pub struct EditorSession {
    chart: ChartFile,
    difficulty: usize,
    time: f64,
    zoom: f64,
    snap: usize,
    tab: usize,
    settings: Settings,
    dirty: bool,
    undo: Vec<ChartFile>,
    redo: Vec<ChartFile>,
    origin: Origin,
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    None,
    Place(usize),
    Move { start_step: f64, start_column: i64, moved: bool },
    Select { anchor_x: f64, anchor_time: f64, button: usize },
    Seek,
}

#[derive(Clone)]
struct Clip {
    step: f64,
    side: Side,
    lane: usize,
    length_steps: f64,
    alt: bool,
}

pub struct Editor {
    ui: Ui,
    chart: ChartFile,
    difficulty: usize,
    time: f64,
    playing: bool,
    inst: Sound,
    voices: Sound,
    inst_wave: Option<Waveform>,
    voices_wave: Option<Waveform>,
    length: f64,
    zoom: f64,
    snap: usize,
    tab: usize,
    settings: Settings,
    selected: Vec<bool>,
    selected_event: Option<usize>,
    drag: Drag,
    move_origin: Vec<(usize, f64, i64)>,
    clipboard: Vec<Clip>,
    undo: Vec<ChartFile>,
    redo: Vec<ChartFile>,
    dirty: bool,
    message: String,
    message_timer: f64,
    event_kind: usize,
    template: ChartEvent,
    characters: Vec<String>,
    stages: Vec<String>,
    songs: Vec<String>,
    open_index: usize,
    copy_from: usize,
    copy_back: f64,
    note_frames: Rc<Frames>,
    note_heads: [usize; 4],
    strums: [usize; 4],
    confirms: [usize; 4],
    icons: Rc<Frames>,
    icon_index: [usize; 2],
    help: bool,
    confirm_exit: f64,
    autosave: f64,
    last_tick: f64,
    flash: [f64; 8],
    origin: Origin,
    previews: Vec<Preview>,
    prev_time: f64,
    loaded_audio: (Option<String>, Option<String>),
    select_base: Vec<bool>,
    scrub_resume: bool,
    new_difficulty: String,
    copy_notes: bool,
    confirm_delete: f64,
}

fn find_frame(frames: &Frames, prefix: &str) -> usize {
    frames.frames.iter().position(|f| f.name.starts_with(prefix)).unwrap_or(0)
}

fn column_of(side: Side, lane: usize) -> i64 {
    match side {
        Side::Opponent => 1 + lane as i64,
        Side::Player => 5 + lane as i64,
    }
}

fn from_column(column: i64) -> (Side, usize) {
    if column <= 4 { (Side::Opponent, (column - 1) as usize) } else { (Side::Player, (column - 5) as usize) }
}

fn column_x(column: i64) -> f64 {
    match column {
        0 => EVENT_X,
        1..=4 => OPPONENT_X + GRID * (column - 1) as f64,
        _ => PLAYER_X + GRID * (column - 5) as f64,
    }
}

fn column_at(x: f64) -> Option<i64> {
    if (EVENT_X..EVENT_X + GRID).contains(&x) {
        Some(0)
    } else if (OPPONENT_X..OPPONENT_X + GRID * 4.0).contains(&x) {
        Some(1 + ((x - OPPONENT_X) / GRID) as i64)
    } else if (PLAYER_X..GRID_RIGHT).contains(&x) {
        Some(5 + ((x - PLAYER_X) / GRID) as i64)
    } else {
        None
    }
}

pub fn format_time(ms: f64) -> String {
    let total = ms.max(0.0);
    let minutes = (total / 60000.0) as u64;
    let seconds = (total / 1000.0) % 60.0;
    format!("{minutes}:{seconds:06.3}")
}

impl Editor {
    pub fn open(ctx: &mut Ctx, id: &str, difficulty: &str) -> Editor {
        let chart = library::chart_file(ctx, id).unwrap_or_else(|| ChartFile::new(id));
        let index = chart.find(difficulty).or_else(|| chart.find("normal")).unwrap_or(0);
        Editor::create(ctx, chart, index, Origin::Freeplay)
    }

    pub fn blank(ctx: &mut Ctx) -> Editor {
        Editor::create(ctx, ChartFile::new("Untitled"), 0, Origin::Options)
    }

    pub fn resume(ctx: &mut Ctx) -> Editor {
        match ctx.data.session.editor.take() {
            Some(session) => {
                let session = *session;
                let mut editor = Editor::create(ctx, session.chart, session.difficulty, session.origin);
                editor.time = session.time;
                editor.prev_time = session.time;
                editor.zoom = session.zoom;
                editor.snap = session.snap;
                editor.tab = session.tab;
                editor.settings = session.settings;
                editor.dirty = session.dirty;
                editor.undo = session.undo;
                editor.redo = session.redo;
                editor.apply_audio_settings();
                editor
            }
            None => Editor::blank(ctx),
        }
    }

    fn create(ctx: &mut Ctx, chart: ChartFile, difficulty: usize, origin: Origin) -> Editor {
        ctx.audio.music.stop();
        let mut camera = Camera::new();
        camera.bg_color = 0xFF101118;
        ctx.cameras = vec![camera];
        let previews = vec![
            Preview::new(ctx, &chart.opponent, false, 1, Rect::new(40.0, 244.0, 240.0, 212.0)),
            Preview::new(ctx, &chart.player, true, 2, Rect::new(40.0, 476.0, 240.0, 212.0)),
        ];
        ctx.data.session.is_story_mode = false;
        let note_frames = ctx.atlas("NOTE_assets");
        let note_heads = ["purple0", "blue0", "green0", "red0"].map(|p| find_frame(&note_frames, p));
        let strums = ["arrowLEFT", "arrowDOWN", "arrowUP", "arrowRIGHT"].map(|p| find_frame(&note_frames, p));
        let confirms = ["left confirm", "down confirm", "up confirm", "right confirm"].map(|p| find_frame(&note_frames, p));
        let icons = ctx.grid("iconGrid", 150, 150);
        let mut stages = vec!["auto".to_string()];
        stages.extend(Kind::ALL.iter().map(|k| k.name().to_string()));
        let mut songs: Vec<String> = super::weeks::WEEKS.iter().flat_map(|w| w.songs.iter().map(|s| super::song::song_id(s))).collect();
        for custom in library::custom_songs(ctx) {
            if !songs.contains(&custom.id) {
                songs.push(custom.id);
            }
        }
        let mut editor = Editor {
            ui: Ui::new(ctx),
            chart,
            difficulty,
            time: 0.0,
            playing: false,
            inst: Sound::empty(),
            voices: Sound::empty(),
            inst_wave: None,
            voices_wave: None,
            length: 0.0,
            zoom: 1.0,
            snap: 3,
            tab: 0,
            settings: Settings::default(),
            selected: Vec::new(),
            selected_event: None,
            drag: Drag::None,
            move_origin: Vec::new(),
            clipboard: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            message: String::new(),
            message_timer: 0.0,
            event_kind: 0,
            template: ChartEvent::new(0.0, "focus"),
            characters: character_names(ctx),
            stages,
            songs,
            open_index: 0,
            copy_from: 1,
            copy_back: 1.0,
            note_frames,
            note_heads,
            strums,
            confirms,
            icons,
            icon_index: [10, 0],
            help: false,
            confirm_exit: 0.0,
            autosave: AUTOSAVE_SECONDS,
            last_tick: 0.0,
            flash: [0.0; 8],
            origin,
            previews,
            prev_time: 0.0,
            loaded_audio: (None, None),
            select_base: Vec::new(),
            scrub_resume: false,
            new_difficulty: String::new(),
            copy_notes: true,
            confirm_delete: 0.0,
        };
        editor.set_template_kind(0);
        editor.ensure_difficulty();
        editor.load_audio(ctx);
        editor
    }

    fn load_audio(&mut self, ctx: &mut Ctx) {
        self.inst.stop();
        self.voices.stop();
        let folder = library::chart_folder(ctx, &self.chart.id);
        for kind in ["Inst", "Voices"] {
            ctx.audio.forget(&folder.join(format!("{kind}.ogg")));
        }
        let files = (self.current().inst.clone(), self.current().voices.clone());
        for file in [&files.0, &files.1].into_iter().flatten() {
            ctx.audio.forget(&folder.join(file));
        }
        let inst = library::inst(ctx, &self.chart.id, &self.chart.name, files.0.as_deref());
        let voices = library::voices(ctx, &self.chart.id, &self.chart.name, files.1.as_deref());
        self.loaded_audio = files;
        self.inst_wave = inst.as_ref().map(Waveform::new);
        self.voices_wave = voices.as_ref().map(Waveform::new);
        self.inst = Sound::new(inst, 1.0, false);
        self.voices = Sound::new(voices, 1.0, false);
        self.apply_audio_settings();
        self.update_length();
    }

    fn switch_difficulty(&mut self, ctx: &mut Ctx, index: usize) {
        self.difficulty = index;
        self.ensure_difficulty();
        self.selected_event = None;
        let files = (self.current().inst.clone(), self.current().voices.clone());
        if files != self.loaded_audio {
            self.load_audio(ctx);
        }
        self.update_length();
    }

    fn apply_audio_settings(&mut self) {
        self.inst.set_volume(self.settings.inst_volume);
        self.voices.set_volume(if self.chart.voices { self.settings.voices_volume } else { 0.0 });
        self.inst.set_rate(self.settings.rate);
        self.voices.set_rate(self.settings.rate);
    }

    fn update_length(&mut self) {
        let audio = self.inst.length().max(self.voices.length());
        let last = self.notes().iter().map(|n| n.time + n.length).fold(0.0, f64::max);
        let fallback = time_at(self.timing(), step_at(self.timing(), last) + 64.0);
        self.length = if audio > 0.0 { audio } else { fallback.max(time_at(self.timing(), 16.0 * 32.0)) };
    }

    fn ensure_difficulty(&mut self) {
        if self.chart.difficulties.is_empty() {
            self.chart.difficulties.push(Difficulty::new("normal", 1.0));
        }
        self.difficulty = self.difficulty.min(self.chart.difficulties.len() - 1);
        let count = self.notes().len();
        self.selected = vec![false; count];
    }

    fn current(&self) -> &Difficulty {
        &self.chart.difficulties[self.difficulty]
    }

    fn current_mut(&mut self) -> &mut Difficulty {
        &mut self.chart.difficulties[self.difficulty]
    }

    fn timing(&self) -> &Vec<TimingPoint> {
        self.current().timing.as_ref().unwrap_or(&self.chart.timing)
    }

    fn timing_mut(&mut self) -> &mut Vec<TimingPoint> {
        let index = self.difficulty;
        match self.chart.difficulties[index].timing {
            Some(ref mut timing) => timing,
            None => &mut self.chart.timing,
        }
    }

    fn events(&self) -> &Vec<ChartEvent> {
        self.current().events.as_ref().unwrap_or(&self.chart.events)
    }

    fn events_mut(&mut self) -> &mut Vec<ChartEvent> {
        let index = self.difficulty;
        match self.chart.difficulties[index].events {
            Some(ref mut events) => events,
            None => &mut self.chart.events,
        }
    }

    fn notes(&self) -> &Vec<ChartNote> {
        &self.current().notes
    }

    fn notes_mut(&mut self) -> &mut Vec<ChartNote> {
        &mut self.current_mut().notes
    }

    fn speed_mut(&mut self) -> &mut f64 {
        &mut self.current_mut().speed
    }

    fn say(&mut self, text: &str) {
        self.message = text.to_string();
        self.message_timer = 4.0;
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.chart.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn restore(&mut self, chart: ChartFile) {
        self.chart = chart;
        self.ensure_difficulty();
        self.selected_event = None;
        self.update_length();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            let current = std::mem::replace(&mut self.chart, previous.clone());
            self.redo.push(current);
            self.restore(previous);
            self.dirty = true;
            self.say("Undo");
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let current = std::mem::replace(&mut self.chart, next.clone());
            self.undo.push(current);
            self.restore(next);
            self.dirty = true;
            self.say("Redo");
        }
    }

    fn snap_steps(&self) -> f64 {
        16.0 / SNAPS[self.snap] as f64
    }

    fn snap_time(&self, time: f64) -> f64 {
        let size = self.snap_steps();
        let step = step_at(self.timing(), time);
        time_at(self.timing(), (step / size).round() * size)
    }

    fn floor_time(&self, time: f64) -> f64 {
        let size = self.snap_steps();
        let step = step_at(self.timing(), time);
        time_at(self.timing(), ((step / size) + 1e-6).floor() * size)
    }

    fn step_offset(&self, time: f64, steps: f64) -> f64 {
        time_at(self.timing(), step_at(self.timing(), time) + steps)
    }

    fn pixels_per_step(&self) -> f64 {
        GRID * self.zoom
    }

    fn y_of(&self, time: f64) -> f64 {
        let current = step_at(self.timing(), self.time);
        PLAYHEAD + (step_at(self.timing(), time) - current) * self.pixels_per_step()
    }

    fn time_of(&self, y: f64) -> f64 {
        let current = step_at(self.timing(), self.time);
        time_at(self.timing(), current + (y - PLAYHEAD) / self.pixels_per_step())
    }

    fn section_index(&self, time: f64) -> i64 {
        (step_at(self.timing(), time) / 16.0).floor() as i64
    }

    fn section_start(&self, index: i64) -> f64 {
        time_at(self.timing(), index as f64 * 16.0)
    }

    fn focus_at(&self, time: f64) -> Side {
        self.chart
            .events
            .iter()
            .filter(|e| e.name == "focus" && e.time <= time + 0.5)
            .max_by(|a, b| a.time.total_cmp(&b.time))
            .map(|e| if e.text("target") == "player" { Side::Player } else { Side::Opponent })
            .unwrap_or(Side::Opponent)
    }

    fn seek(&mut self, ctx: &mut Ctx, time: f64) {
        self.time = time.clamp(0.0, self.length.max(0.0));
        self.last_tick = self.time;
        if self.playing {
            self.inst.set_time(&mut ctx.audio, self.time);
            self.voices.set_time(&mut ctx.audio, self.time);
        }
    }

    fn toggle_play(&mut self, ctx: &mut Ctx) {
        if self.playing {
            self.stop();
        } else {
            self.play_from_playhead(ctx);
        }
    }

    fn play_from_playhead(&mut self, ctx: &mut Ctx) {
        if self.time >= self.length - 1.0 {
            self.time = 0.0;
        }
        self.playing = true;
        self.last_tick = self.time;
        self.apply_audio_settings();
        self.inst.play(&mut ctx.audio, true, self.time);
        self.voices.play(&mut ctx.audio, true, self.time);
    }

    fn stop(&mut self) {
        self.playing = false;
        self.inst.pause();
        self.voices.pause();
    }

    fn set_template_kind(&mut self, kind: usize) {
        self.event_kind = kind;
        let mut event = ChartEvent::new(0.0, EVENT_KINDS[kind]);
        match EVENT_KINDS[kind] {
            "focus" => {
                event.params.insert("target".into(), "player".into());
            }
            "play_animation" => {
                event.params.insert("target".into(), "player".into());
                event.params.insert("animation".into(), "hey".into());
            }
            "camera_bump" => {
                event.params.insert("game".into(), 0.015.into());
                event.params.insert("hud".into(), 0.03.into());
            }
            "camera_zoom" => {
                event.params.insert("zoom".into(), 1.05.into());
            }
            _ => {
                event.params.insert("speed".into(), 1.into());
            }
        }
        self.template = event;
    }

    fn note_at(&self, column: i64, y: f64) -> Option<usize> {
        let notes = self.notes();
        (0..notes.len()).rev().find(|&i| {
            let n = &notes[i];
            let ny = self.y_of(n.time);
            column_of(n.side, n.lane) == column && y >= ny && y < ny + GRID
        })
    }

    fn event_at(&self, y: f64) -> Option<usize> {
        (0..self.events().len()).rev().find(|&i| {
            let ey = self.y_of(self.events()[i].time);
            y >= ey && y < ey + GRID
        })
    }

    fn add_note(&mut self, note: ChartNote, select: bool) -> usize {
        self.notes_mut().push(note);
        self.selected.push(select);
        self.selected.len() - 1
    }

    fn remove_notes(&mut self, remove: &[bool]) {
        let mut index = 0;
        self.notes_mut().retain(|_| {
            let keep = !remove[index];
            index += 1;
            keep
        });
        let mut index = 0;
        self.selected.retain(|_| {
            let keep = !remove[index];
            index += 1;
            keep
        });
    }

    fn clear_selection(&mut self) {
        self.selected.iter_mut().for_each(|s| *s = false);
    }

    fn selection_count(&self) -> usize {
        self.selected.iter().filter(|s| **s).count()
    }

    fn delete_selection(&mut self) {
        if self.selection_count() > 0 {
            self.checkpoint();
            let remove = self.selected.clone();
            self.remove_notes(&remove);
            self.say("Deleted notes");
        } else if let Some(index) = self.selected_event.take() {
            self.checkpoint();
            self.events_mut().remove(index);
            self.say("Deleted event");
        }
    }

    fn toggle_note(&mut self, column: i64, time: f64) {
        let (side, lane) = from_column(column);
        self.checkpoint();
        let existing = self.notes().iter().position(|n| n.side == side && n.lane == lane && (n.time - time).abs() < 2.0);
        match existing {
            Some(i) => {
                let mut remove = vec![false; self.selected.len()];
                remove[i] = true;
                self.remove_notes(&remove);
            }
            None => {
                self.add_note(ChartNote { time, side, lane, length: 0.0, alt: false }, false);
                self.flash[(column - 1) as usize] = 0.15;
            }
        }
    }

    fn copy_selection(&mut self) -> bool {
        let timing = self.timing().clone();
        let picked: Vec<&ChartNote> = self.notes().iter().zip(&self.selected).filter(|(_, s)| **s).map(|(n, _)| n).collect();
        if picked.is_empty() {
            return false;
        }
        let first = picked.iter().map(|n| step_at(&timing, n.time)).fold(f64::INFINITY, f64::min);
        self.clipboard = picked
            .iter()
            .map(|n| {
                let start = step_at(&timing, n.time);
                Clip { step: start - first, side: n.side, lane: n.lane, length_steps: step_at(&timing, n.time + n.length) - start, alt: n.alt }
            })
            .collect();
        true
    }

    fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.checkpoint();
        self.clear_selection();
        let base = step_at(self.timing(), self.snap_time(self.time));
        for clip in self.clipboard.clone() {
            let time = time_at(self.timing(), base + clip.step);
            let end = time_at(self.timing(), base + clip.step + clip.length_steps);
            self.add_note(ChartNote { time, side: clip.side, lane: clip.lane, length: (end - time).max(0.0), alt: clip.alt }, true);
        }
        let count = self.clipboard.len();
        self.say(&format!("Pasted {count} notes"));
    }

    fn adjust_sustain(&mut self, steps: f64) {
        if self.selection_count() == 0 {
            return;
        }
        self.checkpoint();
        let timing = self.timing().clone();
        let selected = self.selected.clone();
        for (note, picked) in self.notes_mut().iter_mut().zip(selected) {
            if picked {
                let end = step_at(&timing, note.time + note.length) + steps;
                note.length = (time_at(&timing, end) - note.time).max(0.0);
            }
        }
    }

    fn mirror_selection(&mut self) {
        if self.selection_count() == 0 {
            return;
        }
        self.checkpoint();
        let selected = self.selected.clone();
        for (note, picked) in self.notes_mut().iter_mut().zip(selected) {
            if picked {
                note.side = note.side.other();
            }
        }
        self.say("Swapped sides");
    }

    fn set_focus(&mut self, section: i64, side: Side) {
        let start = self.section_start(section);
        let end = self.section_start(section + 1);
        self.checkpoint();
        self.events_mut().retain(|e| !(e.name == "focus" && e.time >= start - 0.5 && e.time < end - 0.5));
        let mut event = ChartEvent::new(start, "focus");
        event.params.insert("target".into(), side.name().into());
        self.events_mut().push(event);
        self.selected_event = None;
    }

    fn section_notes(&self, section: i64) -> Vec<bool> {
        let start = self.section_start(section);
        let end = self.section_start(section + 1);
        self.notes().iter().map(|n| n.time >= start - 0.5 && n.time < end - 0.5).collect()
    }

    fn clear_section(&mut self, section: i64, side: Option<Side>) {
        let inside = self.section_notes(section);
        let remove: Vec<bool> = self.notes().iter().zip(inside).map(|(n, inside)| inside && side.map(|s| s == n.side).unwrap_or(true)).collect();
        self.checkpoint();
        self.remove_notes(&remove);
    }

    fn copy_section(&mut self, from: i64, to: i64) {
        if from < 0 || from == to {
            return;
        }
        let timing = self.timing().clone();
        let inside = self.section_notes(from);
        let shift = (to - from) as f64 * 16.0;
        let copies: Vec<ChartNote> = self
            .notes()
            .iter()
            .zip(inside)
            .filter(|(_, inside)| *inside)
            .map(|(n, _)| {
                let start = step_at(&timing, n.time) + shift;
                let end = step_at(&timing, n.time + n.length) + shift;
                let time = time_at(&timing, start);
                ChartNote { time, length: (time_at(&timing, end) - time).max(0.0), ..n.clone() }
            })
            .collect();
        self.checkpoint();
        for note in copies {
            self.add_note(note, false);
        }
    }

    fn set_section_bpm(&mut self, section: i64, bpm: Option<f64>) {
        let start = self.section_start(section);
        let timing = self.timing().clone();
        let notes: Vec<(f64, f64)> = self.notes().iter().map(|n| (step_at(&timing, n.time), step_at(&timing, n.time + n.length))).collect();
        let events: Vec<f64> = self.events().iter().map(|e| step_at(&timing, e.time)).collect();
        self.checkpoint();
        if section == 0 {
            if let Some(bpm) = bpm {
                self.timing_mut()[0].bpm = bpm;
            }
        } else {
            self.timing_mut().retain(|t| (t.time - start).abs() > 0.5);
            if let Some(bpm) = bpm {
                self.timing_mut().push(super::song::TimingPoint { time: start, bpm });
            }
            self.timing_mut().sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        self.rebuild_times(&notes, &events);
    }

    fn rebuild_times(&mut self, notes: &[(f64, f64)], events: &[f64]) {
        let timing = self.timing().clone();
        for (note, (start, end)) in self.notes_mut().iter_mut().zip(notes) {
            note.time = time_at(&timing, *start);
            note.length = (time_at(&timing, *end) - note.time).max(0.0);
        }
        for (event, step) in self.events_mut().iter_mut().zip(events) {
            event.time = time_at(&timing, *step);
        }
        self.update_length();
    }

    fn save(&mut self, ctx: &mut Ctx) {
        self.chart.id = super::song::song_id(&self.chart.name);
        if self.chart.id.is_empty() {
            self.say("Give the song a name before saving");
            return;
        }
        match library::save_chart(ctx, &self.chart) {
            Ok(path) => {
                self.dirty = false;
                let _ = std::fs::remove_file(path.with_file_name("autosave.json"));
                self.say(&format!("Saved to {}", path.display()));
                if !self.songs.contains(&self.chart.id) {
                    self.songs.push(self.chart.id.clone());
                }
            }
            Err(e) => self.say(&format!("Save failed: {e}")),
        }
    }

    fn write_autosave(&mut self, ctx: &mut Ctx) {
        let folder = library::chart_folder(ctx, &self.chart.id);
        if self.chart.id.is_empty() || std::fs::create_dir_all(&folder).is_err() {
            return;
        }
        if std::fs::write(folder.join("autosave.json"), self.chart.write()).is_ok() {
            self.say("Autosaved");
        }
    }

    fn session(&self) -> EditorSession {
        EditorSession {
            chart: self.chart.clone(),
            difficulty: self.difficulty,
            time: self.time,
            zoom: self.zoom,
            snap: self.snap,
            tab: self.tab,
            settings: self.settings.clone(),
            dirty: self.dirty,
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            origin: self.origin,
        }
    }

    fn playtest(&mut self, ctx: &mut Ctx, from_here: bool) {
        let name = self.current().name.clone();
        let Some(song) = self.chart.song(&name) else { return };
        self.stop();
        ctx.data.session.song = Some(Rc::new(song));
        ctx.data.session.editor = Some(Box::new(self.session()));
        ctx.data.session.playtest_start = if from_here { self.time } else { 0.0 };
        ctx.data.session.is_story_mode = false;
        ctx.data.session.prev_cam_follow = None;
        ctx.switch_state(|ctx| Box::new(super::play::PlayState::new(ctx)));
    }

    fn leave(&mut self, ctx: &mut Ctx) {
        self.stop();
        ctx.data.session.editor = None;
        library::play_music(ctx, "freakyMenu", 1.0, true);
        match self.origin {
            Origin::Options => ctx.switch_state(|ctx| Box::new(super::options::Options::new(ctx))),
            Origin::Freeplay => ctx.switch_state(|ctx| Box::new(super::freeplay::Freeplay::new(ctx))),
        }
    }

    fn update_playback(&mut self, ctx: &mut Ctx) {
        let elapsed = ctx.elapsed;
        for f in self.flash.iter_mut() {
            *f = (*f - elapsed).max(0.0);
        }
        self.inst.update(elapsed);
        self.voices.update(elapsed);
        if !self.playing {
            return;
        }
        if self.inst.playing() {
            self.time = self.inst.time();
            if self.voices.playing() && (self.voices.time() - self.time).abs() > 30.0 {
                self.voices.set_time(&mut ctx.audio, self.time);
            }
        } else {
            self.time += elapsed * 1000.0 * self.settings.rate;
        }
        if self.time >= self.length {
            self.time = self.length;
            self.stop();
        }
        let (from, to) = (self.last_tick, self.time);
        if to > from {
            let mut hit = false;
            for note in self.chart.difficulties[self.difficulty].notes.iter() {
                if note.time > from && note.time <= to {
                    let wanted = match note.side {
                        Side::Player => self.settings.hit_player,
                        Side::Opponent => self.settings.hit_opponent,
                    };
                    hit |= wanted;
                    self.flash[(column_of(note.side, note.lane) - 1) as usize] = 0.15;
                }
            }
            if hit {
                library::play_sound(ctx, "clickText", 0.7);
            }
            let timing = self.timing();
            if self.settings.metronome && (step_at(timing, to) / 4.0).floor() > (step_at(timing, from) / 4.0).floor() {
                ctx.audio.play_beep();
            }
        }
        self.last_tick = self.time;
    }

    fn update_previews(&mut self, ctx: &mut Ctx) {
        let (opponent, player) = (self.chart.opponent.clone(), self.chart.player.clone());
        self.previews[0].sync(ctx, &opponent);
        self.previews[1].sync(ctx, &player);
        let (from, to) = (self.prev_time.min(self.time), self.prev_time.max(self.time));
        self.prev_time = self.time;
        if to > from {
            let timing = self.timing().clone();
            let crossed_step = step_at(&timing, to).floor() > step_at(&timing, from).floor();
            let notes = self.notes().clone();
            for note in &notes {
                let slot = if note.side == Side::Player { 1 } else { 0 };
                let hit = note.time > from && note.time <= to;
                let held = crossed_step && note.length > 0.0 && note.time < to && to <= note.time + note.length;
                if hit || held {
                    self.previews[slot].sing(note.lane, note.alt);
                }
            }
            let events = self.events().clone();
            for event in events.iter().filter(|e| e.name == "play_animation" && e.time > from && e.time <= to) {
                let slot = if event.text("target") == "player" { 1 } else { 0 };
                self.previews[slot].play(event.text("animation"));
            }
            let first_beat = (step_at(&timing, from) / 4.0).floor() as i64;
            let last_beat = (step_at(&timing, to) / 4.0).floor() as i64;
            if last_beat > first_beat {
                for preview in self.previews.iter_mut() {
                    preview.beat(last_beat);
                }
            }
        }
        for preview in self.previews.iter_mut() {
            preview.update(ctx);
        }
    }

    fn keyboard(&mut self, ctx: &mut Ctx) {
        if self.ui.typing() {
            return;
        }
        let keys = &ctx.keys;
        let ctrl = keys.ctrl();
        let shift = keys.shift();
        if keys.just_pressed(KeyCode::F1) {
            self.help = !self.help;
        }
        if keys.just_pressed(KeyCode::Escape) {
            if self.help {
                self.help = false;
            } else if self.dirty && self.confirm_exit <= 0.0 {
                self.confirm_exit = 3.0;
                self.say("Unsaved changes! Press Escape again to leave without saving.");
            } else {
                self.leave(ctx);
                return;
            }
        }
        let keys = &ctx.keys;
        if keys.just_pressed(KeyCode::Space) {
            self.toggle_play(ctx);
        }
        let keys = &ctx.keys;
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::KpEnter) {
            let here = keys.shift();
            self.playtest(ctx, here);
            return;
        }
        let keys = &ctx.keys;
        if ctrl {
            if keys.just_pressed(KeyCode::Z) && !shift {
                self.undo();
            } else if keys.just_pressed(KeyCode::Y) || (keys.just_pressed(KeyCode::Z) && shift) {
                self.redo();
            }
            let keys = &ctx.keys;
            if keys.just_pressed(KeyCode::S) {
                self.save(ctx);
            }
            let keys = &ctx.keys;
            if keys.just_pressed(KeyCode::C) && self.copy_selection() {
                self.say("Copied");
            }
            if keys.just_pressed(KeyCode::X) && self.copy_selection() {
                self.delete_selection();
                self.say("Cut");
            }
            if keys.just_pressed(KeyCode::V) {
                self.paste();
            }
            if keys.just_pressed(KeyCode::A) {
                self.selected.iter_mut().for_each(|s| *s = true);
            }
            if keys.just_pressed(KeyCode::D) && self.copy_selection() {
                let last = self.notes().iter().zip(&self.selected).filter(|(_, s)| **s).map(|(n, _)| n.time).fold(0.0, f64::max);
                let first = self.notes().iter().zip(&self.selected).filter(|(_, s)| **s).map(|(n, _)| n.time).fold(f64::INFINITY, f64::min);
                let span = step_at(self.timing(), last) - step_at(self.timing(), first) + self.snap_steps();
                let target = self.step_offset(first, span);
                let current = self.time;
                self.time = target;
                self.paste();
                self.time = current;
                self.say("Duplicated");
            }
            return;
        }
        let keys = &ctx.keys;
        let jump = if shift { 4.0 } else { 1.0 };
        if !self.playing {
            if keys.just_pressed_or_repeated(KeyCode::Up) || keys.just_pressed_or_repeated(KeyCode::W) {
                let t = self.step_offset(self.snap_time(self.time), -self.snap_steps() * jump);
                self.seek(ctx, t);
            }
            let keys = &ctx.keys;
            if keys.just_pressed_or_repeated(KeyCode::Down) || keys.just_pressed_or_repeated(KeyCode::S) {
                let t = self.step_offset(self.snap_time(self.time), self.snap_steps() * jump);
                self.seek(ctx, t);
            }
        }
        let keys = &ctx.keys;
        if keys.just_pressed_or_repeated(KeyCode::PageUp) {
            let section = self.section_index(self.time + 1.0) - 1;
            let t = self.section_start(section.max(0));
            self.seek(ctx, t);
        }
        let keys = &ctx.keys;
        if keys.just_pressed_or_repeated(KeyCode::PageDown) {
            let section = self.section_index(self.time + 1.0) + 1;
            let t = self.section_start(section);
            self.seek(ctx, t);
        }
        let keys = &ctx.keys;
        if keys.just_pressed(KeyCode::Home) {
            self.seek(ctx, 0.0);
        }
        let keys = &ctx.keys;
        if keys.just_pressed(KeyCode::End) {
            let end = self.length;
            self.seek(ctx, end);
        }
        let keys = &ctx.keys;
        if keys.just_pressed_or_repeated(KeyCode::Left) && self.snap > 0 {
            self.snap -= 1;
        }
        if keys.just_pressed_or_repeated(KeyCode::Right) && self.snap + 1 < SNAPS.len() {
            self.snap += 1;
        }
        if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
            self.delete_selection();
        }
        let keys = &ctx.keys;
        if keys.just_pressed_or_repeated(KeyCode::Q) {
            let steps = -self.snap_steps();
            self.adjust_sustain(steps);
        }
        let keys = &ctx.keys;
        if keys.just_pressed_or_repeated(KeyCode::E) {
            let steps = self.snap_steps();
            self.adjust_sustain(steps);
        }
        let keys = &ctx.keys;
        if keys.just_pressed(KeyCode::M) {
            self.mirror_selection();
        }
        let lanes = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4, KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8];
        for (i, key) in lanes.iter().enumerate() {
            if ctx.keys.just_pressed(*key) {
                let time = self.snap_time(self.time);
                self.toggle_note(i as i64 + 1, time);
            }
        }
    }

    fn begin_select(&mut self, ctx: &Ctx, button: usize) {
        let keep = ctx.keys.ctrl() || ctx.keys.shift();
        self.select_base = if keep { self.selected.clone() } else { vec![false; self.selected.len()] };
        self.drag = Drag::Select { anchor_x: ctx.mouse.x, anchor_time: self.time_of(ctx.mouse.y), button };
    }

    fn grid_mouse(&mut self, ctx: &mut Ctx) {
        let (mx, my) = (ctx.mouse.x, ctx.mouse.y);
        let in_grid = (TOP..BOTTOM).contains(&my) && mx >= EVENT_X && mx < GRID_RIGHT;
        let minimap = Rect::new(8.0, TOP, 24.0, BOTTOM - TOP);
        let captured = self.ui.mouse_captured();
        if !captured && ctx.mouse.wheel != 0.0 && (in_grid || minimap.contains(mx, my)) {
            if ctx.keys.ctrl() {
                self.zoom = (self.zoom * if ctx.mouse.wheel > 0.0 { 1.25 } else { 0.8 }).clamp(0.25, 4.0);
            } else {
                let amount = if ctx.keys.shift() { 4.0 } else { 1.0 };
                let t = self.step_offset(self.snap_time(self.time), -ctx.mouse.wheel.signum() * self.snap_steps() * amount);
                self.seek(ctx, t);
            }
        }
        match self.drag {
            Drag::Place(index) => {
                if ctx.mouse.pressed(0) {
                    let head = self.notes()[index].time;
                    let end = self.snap_time(self.time_of(my - GRID / 2.0));
                    let length = if end - head > 1.0 { end - head } else { 0.0 };
                    self.notes_mut()[index].length = length;
                } else {
                    self.drag = Drag::None;
                }
                return;
            }
            Drag::Move { start_step, start_column, moved } => {
                if ctx.mouse.pressed(0) {
                    let size = self.snap_steps();
                    let now = step_at(self.timing(), self.time_of(my));
                    let delta = ((now - start_step) / size).round() * size;
                    let column = column_at(mx).filter(|c| *c > 0).unwrap_or(start_column);
                    let lane_delta = column - start_column;
                    if (delta != 0.0 || lane_delta != 0) && !moved {
                        self.checkpoint();
                        self.drag = Drag::Move { start_step, start_column, moved: true };
                    }
                    let timing = self.timing().clone();
                    let origin = self.move_origin.clone();
                    for (index, step, column) in origin {
                        let length_steps = {
                            let n = &self.notes()[index];
                            step_at(&timing, n.time + n.length) - step_at(&timing, n.time)
                        };
                        let new_column = (column + lane_delta).clamp(1, 8);
                        let (side, lane) = from_column(new_column);
                        let time = time_at(&timing, (step + delta).max(0.0));
                        let end = time_at(&timing, (step + delta).max(0.0) + length_steps);
                        let note = &mut self.notes_mut()[index];
                        note.time = time;
                        note.length = (end - time).max(0.0);
                        note.side = side;
                        note.lane = lane;
                    }
                } else {
                    self.drag = Drag::None;
                }
                return;
            }
            Drag::Select { anchor_x, anchor_time, button } => {
                if ctx.mouse.pressed(button) {
                    if my > BOTTOM - 28.0 {
                        self.seek(ctx, self.time + ctx.elapsed * 600.0);
                    } else if my < TOP + 28.0 {
                        self.seek(ctx, self.time - ctx.elapsed * 600.0);
                    }
                    let now = self.time_of(my.clamp(TOP, BOTTOM));
                    let (t0, t1) = (anchor_time.min(now), anchor_time.max(now));
                    let (ca, cb) = (nearest_column(anchor_x), nearest_column(mx));
                    let (c0, c1) = (ca.min(cb), ca.max(cb));
                    let notes = self.notes().clone();
                    for (i, note) in notes.iter().enumerate() {
                        let column = column_of(note.side, note.lane);
                        let inside = note.time >= t0 - 0.5 && note.time <= t1 + 0.5 && (c0..=c1).contains(&column);
                        let base = self.select_base.get(i).copied().unwrap_or(false);
                        if let Some(s) = self.selected.get_mut(i) {
                            *s = base || inside;
                        }
                    }
                } else {
                    self.drag = Drag::None;
                }
                return;
            }
            Drag::Seek => {
                if ctx.mouse.pressed(0) {
                    let t = ((my - TOP) / (BOTTOM - TOP)).clamp(0.0, 1.0) * self.length;
                    self.time = t.clamp(0.0, self.length.max(0.0));
                    self.last_tick = self.time;
                } else {
                    self.drag = Drag::None;
                    if self.scrub_resume {
                        self.scrub_resume = false;
                        self.play_from_playhead(ctx);
                    }
                }
                return;
            }
            Drag::None => {}
        }
        if captured {
            return;
        }
        if ctx.mouse.just_pressed(0) && minimap.contains(mx, my) {
            self.scrub_resume = self.playing;
            self.stop();
            self.drag = Drag::Seek;
            return;
        }
        if !in_grid {
            let body = (40.0..720.0).contains(&mx) && (TOP..BOTTOM).contains(&my);
            if body && (ctx.mouse.just_pressed(1) || (ctx.mouse.just_pressed(0) && ctx.keys.shift())) {
                self.begin_select(ctx, if ctx.mouse.just_pressed(0) { 0 } else { 1 });
            }
            return;
        }
        let Some(column) = column_at(mx) else { return };
        let time = self.floor_time(self.time_of(my));
        if column == 0 {
            if ctx.mouse.just_pressed(0) {
                match self.event_at(my) {
                    Some(i) => self.selected_event = Some(i),
                    None => {
                        self.checkpoint();
                        let mut event = self.template.clone();
                        event.time = time.max(0.0);
                        self.events_mut().push(event);
                        self.selected_event = Some(self.events().len() - 1);
                        self.tab = 4;
                    }
                }
            } else if ctx.mouse.just_pressed(1) {
                if let Some(i) = self.event_at(my) {
                    self.checkpoint();
                    self.events_mut().remove(i);
                    self.selected_event = None;
                }
            }
            return;
        }
        if ctx.mouse.just_pressed(0) {
            match self.note_at(column, my) {
                Some(i) => {
                    if ctx.keys.ctrl() || ctx.keys.shift() {
                        self.selected[i] = !self.selected[i];
                        return;
                    }
                    if !self.selected[i] {
                        self.clear_selection();
                        self.selected[i] = true;
                    }
                    let timing = self.timing().clone();
                    self.move_origin = self
                        .notes()
                        .iter()
                        .zip(&self.selected)
                        .enumerate()
                        .filter(|(_, (_, s))| **s)
                        .map(|(index, (n, _))| (index, step_at(&timing, n.time), column_of(n.side, n.lane)))
                        .collect();
                    self.drag = Drag::Move { start_step: step_at(&timing, self.time_of(my)), start_column: column, moved: false };
                    self.tab = 3;
                }
                None => {
                    if ctx.keys.shift() {
                        self.begin_select(ctx, 0);
                        return;
                    }
                    if !ctx.keys.ctrl() {
                        self.clear_selection();
                    }
                    self.checkpoint();
                    let (side, lane) = from_column(column);
                    let index = self.add_note(ChartNote { time: time.max(0.0), side, lane, length: 0.0, alt: false }, true);
                    self.drag = Drag::Place(index);
                }
            }
        } else if ctx.mouse.just_pressed(1) {
            match self.note_at(column, my) {
                Some(i) => {
                    self.checkpoint();
                    let mut remove = vec![false; self.selected.len()];
                    remove[i] = true;
                    self.remove_notes(&remove);
                }
                None => self.begin_select(ctx, 1),
            }
        }
    }

    fn draw_grid(&mut self, ctx: &Ctx) {
        let pps = self.pixels_per_step();
        let top_step = step_at(self.timing(), self.time_of(TOP)).floor() as i64 - 1;
        let bottom_step = step_at(self.timing(), self.time_of(BOTTOM)).ceil() as i64 + 1;
        let start_y = self.y_of(0.0).max(TOP);
        let end_y = self.y_of(self.length).min(BOTTOM);
        for column in [0i64, 1, 5] {
            let width = if column == 0 { GRID } else { GRID * 4.0 };
            self.ui.fill(Rect::new(column_x(column), TOP, width, BOTTOM - TOP), 0xFF161822);
            if end_y > start_y {
                self.ui.fill(Rect::new(column_x(column), start_y, width, end_y - start_y), 0xFF1E2130);
            }
        }
        let first_section = (top_step as f64 / 16.0).floor() as i64;
        let last_section = (bottom_step as f64 / 16.0).ceil() as i64;
        for section in first_section.max(0)..=last_section {
            let y0 = self.y_of(self.section_start(section)).max(TOP);
            let y1 = self.y_of(self.section_start(section + 1)).min(BOTTOM);
            if y1 <= y0 {
                continue;
            }
            let focus = self.focus_at(self.section_start(section));
            let x = if focus == Side::Player { PLAYER_X } else { OPPONENT_X };
            self.ui.fill(Rect::new(x, y0, GRID * 4.0, y1 - y0), 0x1A7C5CFF);
        }
        let wave = match self.settings.waveform {
            1 => self.inst_wave.as_ref(),
            2 => self.voices_wave.as_ref(),
            _ => None,
        };
        if let Some(wave) = wave {
            let mut y = start_y;
            let mut rows = Vec::new();
            while y < end_y {
                let peak = wave.peak(self.time_of(y), self.time_of(y + 2.0)) as f64;
                rows.push((y, peak));
                y += 2.0;
            }
            for (y, peak) in rows {
                let half = peak * GRID * 2.0;
                for base in [OPPONENT_X, PLAYER_X] {
                    self.ui.fill(Rect::new(base + GRID * 2.0 - half, y, half * 2.0, 2.0), 0x553D6BFF);
                }
            }
        }
        if pps >= 6.0 {
            for step in top_step.max(0)..=bottom_step {
                let y = self.y_of(time_at(self.timing(), step as f64));
                if !(TOP..BOTTOM).contains(&y) {
                    continue;
                }
                let (color, h) = if step % 16 == 0 {
                    (0xFF6A6F8E, 2.0)
                } else if step % 4 == 0 {
                    (0xFF3A3E55, 1.0)
                } else {
                    (0xFF262938, 1.0)
                };
                for column in [0i64, 1, 5] {
                    let width = if column == 0 { GRID } else { GRID * 4.0 };
                    self.ui.fill(Rect::new(column_x(column), y, width, h), color);
                }
                if step % 16 == 0 {
                    self.ui.text(&format!("{}", step / 16), GRID_RIGHT + 6.0, y - 1.0, 16.0, TEXT_DIM);
                }
            }
        }
        for column in 1..8 {
            if column == 4 {
                continue;
            }
            let x = column_x(column) + GRID;
            self.ui.fill(Rect::new(x, TOP, 1.0, BOTTOM - TOP), 0x30FFFFFF);
        }
        let mut hovered_event = None;
        let events = self.events().clone();
        for (i, event) in events.iter().enumerate() {
            let y = self.y_of(event.time);
            if y + GRID < TOP || y > BOTTOM {
                continue;
            }
            let selected = self.selected_event == Some(i);
            let rect = Rect::new(EVENT_X + 6.0, y + 6.0, GRID - 12.0, GRID - 12.0);
            self.ui.fill(rect, if selected { 0xFFFFFFFF } else { 0xFFFFB347 });
            self.ui.fill(Rect::new(rect.x + 4.0, rect.y + 4.0, rect.w - 8.0, rect.h - 8.0), 0xFF3A2A10);
            let letter = match event.name.as_str() {
                "focus" => "F",
                "play_animation" => "A",
                "camera_bump" => "B",
                "camera_zoom" => "Z",
                "gf_speed" => "G",
                _ => "?",
            };
            self.ui.text_centered(letter, rect, 16.0, if selected { 0xFFFFFFFF } else { 0xFFFFC870 });
            hovered_event = hovered_event.or(rect.contains(ctx.mouse.x, ctx.mouse.y).then(|| event_label(event)));
        }
        let notes = self.notes().clone();
        for (i, note) in notes.iter().enumerate() {
            let y = self.y_of(note.time);
            let end = self.y_of(note.time + note.length);
            if end + GRID < TOP || y > BOTTOM {
                continue;
            }
            let x = column_x(column_of(note.side, note.lane));
            if note.length > 0.0 {
                let top = (y + GRID / 2.0).max(TOP);
                let bottom = (end + GRID / 2.0).min(BOTTOM);
                if bottom > top {
                    self.ui.fill(Rect::new(x + GRID / 2.0 - 5.0, top, 10.0, bottom - top), LANE_COLORS[note.lane] & 0x00FFFFFF | 0xC0000000);
                }
            }
            if y + GRID < TOP {
                continue;
            }
            let rect = Rect::new(x + 1.0, y, GRID - 2.0, GRID);
            let frame = self.note_heads[note.lane];
            let frames = self.note_frames.clone();
            self.ui.image(&frames, frame, rect, 1.0);
            if self.selected.get(i).copied().unwrap_or(false) {
                self.ui.outline(Rect::new(x, y, GRID, GRID), 3.0, 0xFFFFFFFF);
            }
            if note.alt {
                self.ui.text("A", x + 2.0, y, 16.0, 0xFFFFFFFF);
            }
        }
        let strum_y = PLAYHEAD - GRID / 2.0;
        for column in 1..=8 {
            let lane = ((column - 1) % 4) as usize;
            let lit = self.flash[(column - 1) as usize] > 0.0;
            let frame = if lit { self.confirms[lane] } else { self.strums[lane] };
            let frames = self.note_frames.clone();
            let x = column_x(column);
            self.ui.image(&frames, frame, Rect::new(x + 1.0, strum_y, GRID - 2.0, GRID), if lit { 1.0 } else { 0.35 });
        }
        self.ui.fill(Rect::new(EVENT_X - 4.0, PLAYHEAD - 1.0, GRID_RIGHT - EVENT_X + 8.0, 2.0), ACCENT);
        let (mx, my) = (ctx.mouse.x, ctx.mouse.y);
        if !self.ui.mouse_captured() && self.drag == Drag::None && (TOP..BOTTOM).contains(&my) {
            if let Some(column) = column_at(mx) {
                let time = self.floor_time(self.time_of(my));
                let y = self.y_of(time);
                self.ui.fill(Rect::new(column_x(column), y, GRID, GRID), 0x30FFFFFF);
                let step = step_at(self.timing(), time);
                self.ui.tooltip = Some(format!("{}  step {:.2}", format_time(time), step));
            }
        }
        if let Some(label) = hovered_event {
            self.ui.tooltip = Some(label);
        }
        if let Drag::Select { anchor_x, anchor_time, .. } = self.drag {
            let top = self.y_of(anchor_time);
            let area = Rect::new(anchor_x.min(mx), top.min(my), (anchor_x - mx).abs(), (top - my).abs());
            self.ui.fill(area, 0x307C5CFF);
            self.ui.outline(area, 1.0, ACCENT);
        }
        self.ui.fill(Rect::new(EVENT_X - 4.0, TOP - 2.0, GRID_RIGHT - EVENT_X + 40.0, 2.0), BORDER);
    }

    fn draw_minimap(&mut self) {
        let area = Rect::new(8.0, TOP, 24.0, BOTTOM - TOP);
        self.ui.fill(area, 0xFF161822);
        if self.length <= 0.0 {
            return;
        }
        let scale = area.h / self.length;
        let notes = self.notes().clone();
        for note in &notes {
            let y = area.y + note.time * scale;
            let x = if note.side == Side::Opponent { area.x + 2.0 } else { area.x + 13.0 };
            self.ui.fill(Rect::new(x, y, 9.0, 1.0), LANE_COLORS[note.lane] & 0x00FFFFFF | 0xB0000000);
        }
        let view_top = self.time_of(TOP).max(0.0) * scale;
        let view_bottom = self.time_of(BOTTOM).min(self.length) * scale;
        self.ui.outline(Rect::new(area.x, area.y + view_top, area.w, (view_bottom - view_top).max(2.0)), 1.0, 0xA0FFFFFF);
        self.ui.fill(Rect::new(area.x - 3.0, area.y + self.time * scale - 1.0, area.w + 6.0, 2.0), ACCENT);
    }

    fn draw_header(&mut self) {
        let icons = self.icons.clone();
        for (frame, x) in [(self.icon_index[0], OPPONENT_X), (self.icon_index[1], PLAYER_X)] {
            let rect = Rect::new(x + GRID * 2.0 - 32.0, 44.0, 64.0, 64.0);
            self.ui.image(&icons, frame, rect, 1.0);
        }
        self.ui.text_centered("EVENTS", Rect::new(EVENT_X - 10.0, 84.0, GRID + 20.0, 20.0), 14.0, TEXT_DIM);
    }


    fn draw_help(&mut self) {
        let lines = [
            ("Space", "Play / pause"),
            ("Enter / Shift+Enter", "Playtest from start / from playhead"),
            ("Left click", "Place note, drag down for a hold"),
            ("Drag a note", "Move selection in time and lanes"),
            ("Right click", "Delete note or event"),
            ("Right drag", "Box select (Ctrl adds)"),
            ("Ctrl+click", "Toggle note selection"),
            ("1-8", "Toggle a note at the playhead (1-4 opponent, 5-8 player)"),
            ("Q / E", "Shorten / lengthen selected holds"),
            ("M", "Swap selected notes to the other side"),
            ("Up/Down, W/S, wheel", "Move by snap (Shift: x4)"),
            ("PageUp / PageDown", "Previous / next section"),
            ("Home / End", "Jump to start / end"),
            ("Left / Right", "Change snap"),
            ("Ctrl+wheel", "Zoom"),
            ("Ctrl+C / X / V / D", "Copy / cut / paste / duplicate"),
            ("Ctrl+A", "Select all notes"),
            ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
            ("Ctrl+S", "Save"),
            ("Delete", "Delete selection"),
            ("Escape", "Leave the editor"),
        ];
        let panel = Rect::new(240.0, 40.0, 800.0, 650.0);
        self.ui.fill(Rect::new(0.0, 0.0, 1280.0, GAME_HEIGHT), 0xB0000000);
        self.ui.fill(panel, PANEL);
        self.ui.outline(panel, 2.0, ACCENT);
        self.ui.text("Shortcuts", panel.x + 24.0, panel.y + 16.0, 28.0, TEXT);
        for (i, (key, action)) in lines.iter().enumerate() {
            let y = panel.y + 60.0 + i as f64 * 25.5;
            self.ui.text(key, panel.x + 24.0, y, 18.0, 0xFFB9A8FF);
            self.ui.text(action, panel.x + 280.0, y, 18.0, TEXT);
        }
        self.ui.text("Press F1 or Escape to close", panel.x + 24.0, panel.y + panel.h - 34.0, 18.0, TEXT_DIM);
    }
}

pub fn event_label(event: &ChartEvent) -> String {
    match event.name.as_str() {
        "focus" => format!("focus {}", event.text("target")),
        "play_animation" => format!("{} {}", event.text("target"), event.text("animation")),
        "camera_bump" => "camera bump".to_string(),
        "camera_zoom" => format!("zoom {}", event.number("zoom", 1.0)),
        "gf_speed" => format!("gf speed {}", event.number("speed", 1.0)),
        other => other.to_string(),
    }
}

impl State for Editor {
    fn update(&mut self, ctx: &mut Ctx) {
        self.ui.begin(ctx);
        self.update_playback(ctx);
        self.confirm_exit = (self.confirm_exit - ctx.elapsed).max(0.0);
        self.confirm_delete = (self.confirm_delete - ctx.elapsed).max(0.0);
        self.message_timer = (self.message_timer - ctx.elapsed).max(0.0);
        if self.dirty {
            self.autosave -= ctx.elapsed;
            if self.autosave <= 0.0 {
                self.autosave = AUTOSAVE_SECONDS;
                self.write_autosave(ctx);
            }
        }
        self.icon_index = [&self.chart.opponent, &self.chart.player].map(|c| icon_frames(ctx, c).first().copied().unwrap_or(10));
        if !self.help {
            self.draw_grid(ctx);
            self.draw_minimap();
            self.draw_header();
            self.panels(ctx);
            self.grid_mouse(ctx);
        } else {
            self.draw_help();
        }
        if !ctx.is_exiting() {
            self.keyboard(ctx);
        }
        let bpm = super::song::bpm_at(self.timing(), self.time);
        ctx.data.conductor.change_bpm(bpm);
        self.update_previews(ctx);
        for camera in 1..ctx.cameras.len() {
            ctx.cameras[camera].visible = !self.help;
        }
        self.ui.finish();
    }

    fn draw(&mut self, ctx: &mut Ctx, dl: &mut DrawList) {
        self.ui.render(&mut ctx.gfx, dl, 0);
        for preview in &self.previews {
            preview.draw(dl);
        }
    }
}
fn nearest_column(x: f64) -> i64 {
    column_at(x).unwrap_or(if x < OPPONENT_X { 1 } else { 8 }).max(1)
}