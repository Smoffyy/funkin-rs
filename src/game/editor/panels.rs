use serde_json::Value;

use crate::engine::app::Ctx;

use super::super::library;
use super::super::song::{difficulty_id, step_at, ChartFile, Difficulty, Drain, Side};
use super::ui::{Rect, BORDER, PANEL, PANEL_LIGHT, TEXT, TEXT_DIM};
use super::{event_label, format_time, Editor, EVENT_KINDS, SNAPS};

const TABS: [&str; 6] = ["Song", "Diff", "Section", "Notes", "Events", "Editor"];
const X: f64 = 736.0;
const W: f64 = 520.0;
const ROW: f64 = 30.0;
const LABEL: f64 = 150.0;

fn row(y: f64) -> Rect {
    Rect::new(X, y, W, ROW - 4.0)
}

fn field(y: f64) -> Rect {
    Rect::new(X + LABEL, y, W - LABEL, ROW - 4.0)
}

fn half(y: f64, index: usize) -> Rect {
    let w = (W - 8.0) / 2.0;
    Rect::new(X + (w + 8.0) * index as f64, y, w, ROW - 4.0)
}

fn third(y: f64, index: usize) -> Rect {
    let w = (W - 16.0) / 3.0;
    Rect::new(X + (w + 8.0) * index as f64, y, w, ROW - 4.0)
}

fn options(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

impl Editor {
    pub(super) fn panels(&mut self, ctx: &mut Ctx) {
        self.top_bar(ctx);
        self.info();
        let panel = Rect::new(720.0, 48.0, 552.0, 638.0);
        self.ui.fill(panel, PANEL);
        self.ui.outline(panel, 1.0, BORDER);
        let mut tab = self.tab;
        self.ui.tabs("tabs", Rect::new(X, 60.0, W, 30.0), &TABS, &mut tab);
        self.tab = tab;
        let y = 104.0;
        match self.tab {
            0 => self.song_tab(ctx, y),
            1 => self.difficulty_tab(ctx, y),
            2 => self.section_tab(y),
            3 => self.notes_tab(y),
            4 => self.events_tab(y),
            _ => self.editor_tab(ctx, y),
        }
        self.status_bar();
    }

    fn top_bar(&mut self, ctx: &mut Ctx) {
        self.ui.fill(Rect::new(0.0, 0.0, 1280.0, 40.0), PANEL);
        self.ui.fill(Rect::new(0.0, 40.0, 1280.0, 1.0), BORDER);
        let title = format!("{}{}", self.chart.name, if self.dirty { " *" } else { "" });
        self.ui.text(&title, 96.0, 9.0, 22.0, TEXT);
        let names: Vec<String> = self.chart.difficulties.iter().map(|d| d.name.to_uppercase()).collect();
        let mut difficulty = self.difficulty;
        let area = Rect::new(290.0, 6.0, 266.0, 28.0);
        let changed = if names.len() <= 3 {
            let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            self.ui.tabs("difficulty", area, &refs, &mut difficulty)
        } else {
            self.ui.dropdown("difficulty", area, &names, &mut difficulty)
        };
        if changed {
            self.switch_difficulty(ctx, difficulty);
        }
        let add = Rect::new(560.0, 6.0, 30.0, 28.0);
        if self.ui.button("adddiff", add, "+") {
            self.tab = 1;
            self.say("Name the new difficulty in the Diff tab");
        }
        self.ui.tip(add, "Add a difficulty");
        let play = if self.playing { "Pause" } else { "Play" };
        if self.ui.button("play", Rect::new(600.0, 6.0, 80.0, 28.0), play) {
            self.toggle_play(ctx);
        }
        if self.ui.button("test", Rect::new(688.0, 6.0, 110.0, 28.0), "Playtest") {
            self.playtest(ctx, false);
            return;
        }
        self.ui.tip(Rect::new(688.0, 6.0, 110.0, 28.0), "Enter: from start, Shift+Enter: from playhead");
        if self.ui.button("testhere", Rect::new(806.0, 6.0, 130.0, 28.0), "Test from here") {
            self.playtest(ctx, true);
            return;
        }
        if self.ui.button("save", Rect::new(944.0, 6.0, 80.0, 28.0), "Save") {
            self.save(ctx);
        }
        if self.ui.button("help", Rect::new(1032.0, 6.0, 40.0, 28.0), "?") {
            self.help = true;
        }
        let time = format!("{} / {}", format_time(self.time), format_time(self.length));
        self.ui.text_right(&time, 1268.0, 10.0, 20.0, TEXT_DIM);
    }

    fn info(&mut self) {
        let x = 44.0;
        let mut y = 120.0;
        let step = step_at(self.timing(), self.time);
        let bpm = super::super::song::bpm_at(self.timing(), self.time);
        let opponent = self.notes().iter().filter(|n| n.side == Side::Opponent).count();
        let player = self.notes().len() - opponent;
        let lines = [
            ("Section", format!("{}", (step / 16.0).floor() as i64)),
            ("Beat", format!("{:.2}", step / 4.0)),
            ("Step", format!("{step:.2}")),
            ("BPM", format!("{bpm}")),
            ("Snap", format!("1/{}", SNAPS[self.snap])),
            ("Zoom", format!("{:.2}x", self.zoom)),
            ("Opp notes", format!("{opponent}")),
            ("Player", format!("{player}")),
            ("Events", format!("{}", self.events().len())),
            ("Selected", format!("{}", self.selection_count())),
        ];
        for (i, (label, value)) in lines.iter().enumerate() {
            let column = x + (i % 2) as f64 * 124.0;
            self.ui.text(label, column, y, 16.0, TEXT_DIM);
            self.ui.text(value, column + 62.0, y, 16.0, TEXT);
            if i % 2 == 1 {
                y += 22.0;
            }
        }
        let cards = [(self.previews[0].area, "OPPONENT"), (self.previews[1].area, "PLAYER")];
        for (area, label) in cards {
            self.ui.fill(area, PANEL_LIGHT);
            self.ui.outline(area, 1.0, BORDER);
            self.ui.text(label, area.x + 6.0, area.y + 4.0, 14.0, TEXT_DIM);
        }
    }

    fn status_bar(&mut self) {
        self.ui.fill(Rect::new(0.0, 692.0, 1280.0, 28.0), PANEL_LIGHT);
        let text = if self.message_timer > 0.0 {
            self.message.clone()
        } else {
            "Click to place notes, drag down for holds, right click to delete. Space plays, Enter playtests, F1 lists shortcuts.".to_string()
        };
        self.ui.text(&text, 10.0, 697.0, 16.0, if self.message_timer > 0.0 { TEXT } else { TEXT_DIM });
    }

    fn song_tab(&mut self, ctx: &mut Ctx, mut y: f64) {
        let mut chart = self.chart.clone();
        let mut changed = false;
        self.ui.label("Song name", row(y));
        changed |= self.ui.text_field("name", field(y), &mut chart.name);
        y += ROW;
        self.ui.label("Artist", row(y));
        changed |= self.ui.text_field("artist", field(y), &mut chart.artist);
        y += ROW;
        self.ui.label("Charter", row(y));
        changed |= self.ui.text_field("charter", field(y), &mut chart.charter);
        y += ROW;
        self.ui.label("BPM", row(y));
        let mut bpm = self.timing()[0].bpm;
        if self.ui.stepper("bpm", field(y), &mut bpm, 1.0, 10.0, 999.0, 2) {
            self.chart = chart.clone();
            self.set_section_bpm(0, Some(bpm));
            chart = self.chart.clone();
        }
        y += ROW;
        self.ui.label("Offset (ms)", row(y));
        changed |= self.ui.stepper("offset", field(y), &mut chart.offset, 1.0, -5000.0, 5000.0, 0);
        y += ROW;
        self.ui.label("Player", row(y));
        let mut index = self.characters.iter().position(|c| *c == chart.player).unwrap_or(0);
        let characters = self.characters.clone();
        if self.ui.dropdown("player", field(y), &characters, &mut index) {
            chart.player = characters[index].clone();
            changed = true;
        }
        y += ROW;
        self.ui.label("Opponent", row(y));
        let mut index = self.characters.iter().position(|c| *c == chart.opponent).unwrap_or(0);
        if self.ui.dropdown("opponent", field(y), &characters, &mut index) {
            chart.opponent = characters[index].clone();
            changed = true;
        }
        y += ROW;
        self.ui.label("Girlfriend", row(y));
        let mut gfs = vec!["auto".to_string()];
        gfs.extend(characters.iter().filter(|c| c.starts_with("gf")).cloned());
        let mut index = chart.girlfriend.as_ref().and_then(|g| gfs.iter().position(|c| c == g)).unwrap_or(0);
        if self.ui.dropdown("girlfriend", field(y), &gfs, &mut index) {
            chart.girlfriend = if index == 0 { None } else { Some(gfs[index].clone()) };
            changed = true;
        }
        y += ROW;
        self.ui.label("Stage", row(y));
        let stages = self.stages.clone();
        let mut index = chart.stage.as_ref().and_then(|s| stages.iter().position(|c| c == s)).unwrap_or(0);
        if self.ui.dropdown("stage", field(y), &stages, &mut index) {
            chart.stage = if index == 0 { None } else { Some(stages[index].clone()) };
            changed = true;
        }
        y += ROW;
        let mut voices = chart.voices;
        if self.ui.checkbox("voices", row(y), "Song has a vocals track", &mut voices) {
            chart.voices = voices;
            changed = true;
        }
        y += ROW + 6.0;
        if changed {
            self.checkpoint();
            self.chart = chart;
            self.apply_audio_settings();
        }
        let folder = library::chart_folder(ctx, &super::super::song::song_id(&self.chart.name));
        let inst = if self.inst.length() > 0.0 { "loaded" } else { "missing" };
        let vocals = if self.voices.length() > 0.0 { "loaded" } else { "missing" };
        self.ui.text(&format!("Inst: {inst}    Vocals: {vocals}"), X, y, 16.0, TEXT_DIM);
        y += 20.0;
        self.ui.text(&format!("Custom audio: {}", folder.join("Inst.ogg / Voices.ogg").display()), X, y, 14.0, TEXT_DIM);
        y += 24.0;
        if self.ui.button("reload", half(y, 0), "Reload audio") {
            self.load_audio(ctx);
            self.say("Audio reloaded");
        }
        if self.ui.button("folder", half(y, 1), "Open song folder") {
            let _ = std::fs::create_dir_all(&folder);
            let program = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
            let _ = std::process::Command::new(program).arg(&folder).spawn();
        }
        y += ROW + 6.0;
        self.ui.label("Open song", row(y));
        let songs = self.songs.clone();
        let mut open = self.open_index.min(songs.len().saturating_sub(1));
        if self.ui.dropdown("open", Rect::new(X + LABEL, y, W - LABEL - 88.0, ROW - 4.0), &songs, &mut open) {
            self.open_index = open;
        }
        if self.ui.button("openbtn", Rect::new(X + W - 80.0, y, 80.0, ROW - 4.0), "Open") {
            if let Some(id) = songs.get(open).cloned() {
                if let Some(chart) = library::chart_file(ctx, &id) {
                    self.open_chart(ctx, chart);
                    self.say(&format!("Opened {id}"));
                }
            }
        }
        y += ROW;
        if self.ui.button("new", half(y, 0), "New song") {
            self.open_chart(ctx, ChartFile::new("Untitled"));
            self.say("New song");
        }
        let autosave = folder.join("autosave.json");
        if self.ui.button("restore", half(y, 1), "Load autosave") {
            match std::fs::read_to_string(&autosave).ok().and_then(|t| ChartFile::parse(&self.chart.id, &t)) {
                Some(chart) => {
                    self.checkpoint();
                    self.restore(chart);
                    self.say("Autosave loaded");
                }
                None => self.say("No autosave found for this song"),
            }
        }
    }

    fn open_chart(&mut self, ctx: &mut Ctx, chart: ChartFile) {
        self.stop();
        self.difficulty = chart.find("normal").unwrap_or(0);
        self.chart = chart;
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
        self.time = 0.0;
        self.selected_event = None;
        self.ensure_difficulty();
        self.load_audio(ctx);
    }

    fn difficulty_tab(&mut self, ctx: &mut Ctx, mut y: f64) {
        let current = self.current().clone();
        self.ui.text(&format!("Difficulty: {}", current.name.to_uppercase()), X, y, 22.0, TEXT);
        y += ROW + 4.0;
        self.ui.label("Name", row(y));
        let mut name = current.name.clone();
        if self.ui.text_field("diffname", field(y), &mut name) {
            let id = difficulty_id(&name);
            if id.is_empty() || self.chart.find(&id).is_some() {
                self.say("That difficulty name is empty or already used");
            } else {
                self.checkpoint();
                self.current_mut().name = id;
            }
        }
        y += ROW;
        self.ui.label("Scroll speed", row(y));
        let mut speed = current.speed;
        if self.ui.stepper("speed", field(y), &mut speed, 0.1, 0.1, 10.0, 2) {
            self.checkpoint();
            *self.speed_mut() = speed;
        }
        y += ROW + 6.0;
        let mut own_audio = current.inst.is_some() || current.voices.is_some();
        if self.ui.checkbox("ownaudio", row(y), "Use its own music", &mut own_audio) {
            self.checkpoint();
            let tag = self.current().name.clone();
            let d = self.current_mut();
            if own_audio {
                d.inst = Some(format!("Inst-{tag}.ogg"));
                d.voices = Some(format!("Voices-{tag}.ogg"));
            } else {
                d.inst = None;
                d.voices = None;
            }
            self.load_audio(ctx);
        }
        y += ROW;
        let folder = library::chart_folder(ctx, &self.chart.id);
        if own_audio {
            for (key, label, file) in [("instfile", "Inst file", current.inst.clone()), ("voicefile", "Vocals file", current.voices.clone())] {
                self.ui.label(label, row(y));
                let mut value = file.clone().unwrap_or_default();
                let found = file.map(|f| folder.join(f).exists()).unwrap_or(false);
                let input = Rect::new(X + LABEL, y, W - LABEL - 80.0, ROW - 4.0);
                if self.ui.text_field(key, input, &mut value) {
                    self.checkpoint();
                    let value = if value.trim().is_empty() { None } else { Some(value.trim().to_string()) };
                    if key == "instfile" {
                        self.current_mut().inst = value;
                    } else {
                        self.current_mut().voices = value;
                    }
                    self.load_audio(ctx);
                }
                let status = if found { "found" } else { "missing" };
                self.ui.text(status, X + W - 70.0, y + 3.0, 18.0, if found { TEXT } else { 0xFFFF7A7A });
                y += ROW;
            }
            self.ui.text(&format!("Put the files in {}", folder.display()), X, y, 14.0, TEXT_DIM);
            y += 20.0;
            if self.ui.button("reloaddiff", half(y, 0), "Reload audio") {
                self.load_audio(ctx);
                self.say("Audio reloaded");
            }
            y += ROW;
        } else {
            self.ui.text("Uses the same music as the song.", X, y, 16.0, TEXT_DIM);
            y += 24.0;
        }
        let mut own_timing = current.timing.is_some() || current.events.is_some();
        if self.ui.checkbox("owntiming", row(y), "Use its own BPM and events", &mut own_timing) {
            self.checkpoint();
            let (timing, events) = (self.chart.timing.clone(), self.chart.events.clone());
            let d = self.current_mut();
            if own_timing {
                d.timing = Some(timing);
                d.events = Some(events);
            } else {
                d.timing = None;
                d.events = None;
            }
            self.update_length();
        }
        y += ROW + 6.0;
        let mut drain_on = current.drain.is_some();
        if self.ui.checkbox("draintoggle", row(y), "Opponent drains player health", &mut drain_on) {
            self.checkpoint();
            self.current_mut().drain = if drain_on { Some(Drain { amount: 0.023, floor: 25.0 }) } else { None };
        }
        y += ROW;
        if let Some(drain) = current.drain {
            self.ui.label("Drain per hit", row(y));
            let mut amount = drain.amount;
            if self.ui.stepper("drainamount", field(y), &mut amount, 0.005, 0.0, 0.5, 3) {
                self.checkpoint();
                if let Some(d) = self.current_mut().drain.as_mut() {
                    d.amount = amount;
                }
            }
            y += ROW;
            self.ui.label("Stops at (%)", row(y));
            let mut floor = drain.floor;
            if self.ui.stepper("drainfloor", field(y), &mut floor, 5.0, 0.0, 100.0, 0) {
                self.checkpoint();
                if let Some(d) = self.current_mut().drain.as_mut() {
                    d.floor = floor;
                }
            }
            y += ROW;
        }
        y += 6.0;
        let names: Vec<String> = self.chart.difficulties.iter().map(|d| d.name.to_uppercase()).collect();
        self.ui.label("Copy notes from", row(y));
        let mut from = self.copy_from.min(names.len() - 1);
        let picker = Rect::new(X + LABEL, y, W - LABEL - 88.0, ROW - 4.0);
        if self.ui.dropdown("copyfrom", picker, &names, &mut from) {
            self.copy_from = from;
        }
        if self.ui.button("copydiff", Rect::new(X + W - 80.0, y, 80.0, ROW - 4.0), "Copy") && from != self.difficulty {
            self.checkpoint();
            let notes = self.chart.difficulties[from].notes.clone();
            *self.notes_mut() = notes;
            self.ensure_difficulty();
            self.say(&format!("Copied notes from {}", names[from]));
        }
        y += ROW;
        if self.chart.difficulties.len() > 1 {
            let label = if self.confirm_delete > 0.0 { "Click again to delete" } else { "Delete this difficulty" };
            if self.ui.button("deldiff", row(y), label) {
                if self.confirm_delete > 0.0 {
                    self.checkpoint();
                    self.chart.difficulties.remove(self.difficulty);
                    self.confirm_delete = 0.0;
                    let index = self.difficulty.saturating_sub(1);
                    self.switch_difficulty(ctx, index);
                    self.say("Difficulty deleted");
                } else {
                    self.confirm_delete = 3.0;
                }
            }
            y += ROW;
        }
        y += 10.0;
        self.ui.fill(Rect::new(X, y, W, 1.0), BORDER);
        y += 12.0;
        self.ui.text("Add a difficulty", X, y, 22.0, TEXT);
        y += ROW + 4.0;
        self.ui.label("Name", row(y));
        let mut new_name = self.new_difficulty.clone();
        self.ui.text_field("newdiff", field(y), &mut new_name);
        self.new_difficulty = new_name;
        y += ROW;
        let mut copy = self.copy_notes;
        self.ui.checkbox("copynotes", row(y), "Start with the current notes", &mut copy);
        self.copy_notes = copy;
        y += ROW;
        if self.ui.button("adddifficulty", half(y, 0), "Add difficulty") {
            let id = difficulty_id(&self.new_difficulty);
            if id.is_empty() || self.chart.find(&id).is_some() {
                self.say("Pick a new, unused difficulty name");
            } else {
                self.checkpoint();
                let mut difficulty = Difficulty::new(&id, current.speed);
                if self.copy_notes {
                    difficulty.notes = current.notes.clone();
                }
                self.chart.difficulties.push(difficulty);
                self.new_difficulty.clear();
                let index = self.chart.difficulties.len() - 1;
                self.switch_difficulty(ctx, index);
                self.say(&format!("Added {}", id.to_uppercase()));
            }
        }
    }

    fn section_tab(&mut self, mut y: f64) {
        let section = self.section_index(self.time + 0.5).max(0);
        let start = self.section_start(section);
        self.ui.text(&format!("Section {section}  ({})", format_time(start)), X, y, 22.0, TEXT);
        y += ROW + 4.0;
        self.ui.label("Camera focus", row(y));
        let focus = self.focus_at(start);
        let w = (W - LABEL - 8.0) / 2.0;
        if self.ui.button_colored("focusopp", Rect::new(X + LABEL, y, w, ROW - 4.0), "Opponent", focus == Side::Opponent) && focus != Side::Opponent {
            self.set_focus(section, Side::Opponent);
        }
        if self.ui.button_colored("focusplayer", Rect::new(X + LABEL + w + 8.0, y, w, ROW - 4.0), "Player", focus == Side::Player) && focus != Side::Player {
            self.set_focus(section, Side::Player);
        }
        y += ROW;
        let point = self.timing().iter().find(|t| (t.time - start).abs() < 0.5).copied();
        let mut changes = point.is_some() && section > 0;
        let bpm_here = super::super::song::bpm_at(self.timing(), start);
        if section > 0 && self.ui.checkbox("changebpm", row(y), "Change BPM at this section", &mut changes) {
            self.set_section_bpm(section, if changes { Some(bpm_here) } else { None });
        }
        if section > 0 {
            y += ROW;
        }
        self.ui.label("BPM", row(y));
        let mut bpm = bpm_here;
        if self.ui.stepper("sectionbpm", field(y), &mut bpm, 1.0, 10.0, 999.0, 2) {
            let target = if section == 0 || changes { section } else { self.timing().iter().rev().find(|t| t.time <= start + 0.5).map(|t| self.section_index(t.time + 0.5)).unwrap_or(0) };
            self.set_section_bpm(target, Some(bpm));
        }
        y += ROW + 8.0;
        self.ui.text("Notes in this section", X, y, 16.0, TEXT_DIM);
        y += 22.0;
        if self.ui.button("selsection", third(y, 0), "Select") {
            self.selected = self.section_notes(section);
        }
        if self.ui.button("clearall", third(y, 1), "Clear all") {
            self.clear_section(section, None);
        }
        if self.ui.button("swapsection", third(y, 2), "Swap sides") {
            self.selected = self.section_notes(section);
            self.mirror_selection();
        }
        y += ROW;
        if self.ui.button("clearopp", half(y, 0), "Clear opponent") {
            self.clear_section(section, Some(Side::Opponent));
        }
        if self.ui.button("clearplayer", half(y, 1), "Clear player") {
            self.clear_section(section, Some(Side::Player));
        }
        y += ROW;
        if self.ui.button("altsection", half(y, 0), "Opponent alt anims") {
            let inside = self.section_notes(section);
            self.checkpoint();
            let all = self.notes().iter().zip(&inside).filter(|(n, i)| **i && n.side == Side::Opponent).all(|(n, _)| n.alt);
            for (note, inside) in self.notes_mut().iter_mut().zip(inside) {
                if inside && note.side == Side::Opponent {
                    note.alt = !all;
                }
            }
        }
        if self.ui.button("dupsection", half(y, 1), "Duplicate to next") {
            self.copy_section(section, section + 1);
            self.say("Section duplicated");
        }
        y += ROW + 8.0;
        self.ui.label("Copy from back", row(y));
        let mut back = self.copy_back;
        self.ui.stepper("copyback", Rect::new(X + LABEL, y, 150.0, ROW - 4.0), &mut back, 1.0, 1.0, 64.0, 0);
        self.copy_back = back;
        if self.ui.button("copyfromback", Rect::new(X + LABEL + 160.0, y, W - LABEL - 160.0, ROW - 4.0), "Copy here") {
            self.copy_section(section - back as i64, section);
            self.say(&format!("Copied section {}", section - back as i64));
        }
    }

    fn notes_tab(&mut self, mut y: f64) {
        let count = self.selection_count();
        self.ui.text(&format!("{count} selected"), X, y, 22.0, TEXT);
        y += ROW + 4.0;
        if count == 0 {
            self.ui.text("Click a note to select it, Ctrl+click to add more,", X, y, 16.0, TEXT_DIM);
            self.ui.text("or right drag on the grid to box select.", X, y + 20.0, 16.0, TEXT_DIM);
            return;
        }
        let timing = self.timing().clone();
        let first = self.notes().iter().zip(&self.selected).find(|(_, s)| **s).map(|(n, _)| n.clone());
        if let Some(note) = first {
            self.ui.label("Hold (steps)", row(y));
            let mut steps = step_at(&timing, note.time + note.length) - step_at(&timing, note.time);
            let before = steps;
            if self.ui.stepper("hold", field(y), &mut steps, self.snap_steps(), 0.0, 1024.0, 2) {
                self.adjust_sustain(steps - before);
            }
            y += ROW;
            let mut alt = note.alt;
            if self.ui.checkbox("alt", row(y), "Alt animation (opponent)", &mut alt) {
                self.checkpoint();
                let selected = self.selected.clone();
                for (n, picked) in self.notes_mut().iter_mut().zip(selected) {
                    if picked {
                        n.alt = alt;
                    }
                }
            }
            y += ROW + 6.0;
        }
        if self.ui.button("mirror", third(y, 0), "Swap sides") {
            self.mirror_selection();
        }
        if self.ui.button("flip", third(y, 1), "Flip lanes") {
            self.checkpoint();
            let selected = self.selected.clone();
            for (n, picked) in self.notes_mut().iter_mut().zip(selected) {
                if picked {
                    n.lane = 3 - n.lane;
                }
            }
        }
        if self.ui.button("delete", third(y, 2), "Delete") {
            self.delete_selection();
        }
        y += ROW;
        if self.ui.button("copy", third(y, 0), "Copy") && self.copy_selection() {
            self.say("Copied");
        }
        if self.ui.button("paste", third(y, 1), "Paste at playhead") {
            self.paste();
        }
        if self.ui.button("deselect", third(y, 2), "Deselect") {
            self.clear_selection();
        }
    }

    fn events_tab(&mut self, mut y: f64) {
        self.ui.text("New event", X, y, 22.0, TEXT);
        y += ROW + 4.0;
        self.ui.label("Type", row(y));
        let kinds = options(&EVENT_KINDS);
        let mut kind = self.event_kind;
        if self.ui.dropdown("eventkind", field(y), &kinds, &mut kind) {
            self.set_template_kind(kind);
        }
        y += ROW;
        let mut template = self.template.clone();
        if self.event_fields("template", &mut template, &mut y) {
            self.template = template;
        }
        self.ui.text("Click the events column to place this event.", X, y + 4.0, 16.0, TEXT_DIM);
        y += ROW + 16.0;
        self.ui.fill(Rect::new(X, y, W, 1.0), BORDER);
        y += 12.0;
        let Some(index) = self.selected_event.filter(|i| *i < self.events().len()) else {
            self.ui.text("No event selected", X, y, 18.0, TEXT_DIM);
            return;
        };
        let mut event = self.events()[index].clone();
        self.ui.text(&format!("Selected: {} at {}", event_label(&event), format_time(event.time)), X, y, 18.0, TEXT);
        y += ROW;
        if self.event_fields("selected", &mut event, &mut y) {
            self.checkpoint();
            self.events_mut()[index] = event;
        }
        if self.ui.button("delevent", half(y, 0), "Delete event") {
            self.checkpoint();
            self.events_mut().remove(index);
            self.selected_event = None;
        }
        if self.ui.button("jumpevent", half(y, 1), "Jump to event") {
            self.time = self.events()[index].time;
        }
    }

    fn event_fields(&mut self, id: &str, event: &mut super::super::song::ChartEvent, y: &mut f64) -> bool {
        let mut changed = false;
        match event.name.as_str() {
            "focus" | "play_animation" => {
                self.ui.label("Target", row(*y));
                let targets = if event.name == "focus" { options(&["player", "opponent"]) } else { options(&["player", "opponent", "girlfriend"]) };
                let mut index = targets.iter().position(|t| t == event.text("target")).unwrap_or(0);
                if self.ui.dropdown(&format!("{id}target"), field(*y), &targets, &mut index) {
                    event.params.insert("target".into(), Value::String(targets[index].clone()));
                    changed = true;
                }
                *y += ROW;
                if event.name == "play_animation" {
                    self.ui.label("Animation", row(*y));
                    let mut animation = event.text("animation").to_string();
                    if self.ui.text_field(&format!("{id}anim"), field(*y), &mut animation) {
                        event.params.insert("animation".into(), Value::String(animation));
                        changed = true;
                    }
                    *y += ROW;
                }
            }
            "camera_bump" => {
                for (key, label) in [("game", "Game zoom"), ("hud", "HUD zoom")] {
                    self.ui.label(label, row(*y));
                    let mut value = event.number(key, 0.015);
                    if self.ui.stepper(&format!("{id}{key}"), field(*y), &mut value, 0.005, 0.0, 1.0, 3) {
                        event.params.insert(key.into(), value.into());
                        changed = true;
                    }
                    *y += ROW;
                }
            }
            "camera_zoom" => {
                self.ui.label("Zoom", row(*y));
                let mut value = event.number("zoom", 1.05);
                if self.ui.stepper(&format!("{id}zoom"), field(*y), &mut value, 0.05, 0.1, 4.0, 2) {
                    event.params.insert("zoom".into(), value.into());
                    changed = true;
                }
                *y += ROW;
            }
            _ => {
                self.ui.label("GF speed", row(*y));
                let mut value = event.number("speed", 1.0);
                if self.ui.stepper(&format!("{id}speed"), field(*y), &mut value, 1.0, 1.0, 8.0, 0) {
                    event.params.insert("speed".into(), (value as i64).into());
                    changed = true;
                }
                *y += ROW;
            }
        }
        changed
    }

    fn editor_tab(&mut self, ctx: &mut Ctx, mut y: f64) {
        let mut settings = self.settings.clone();
        self.ui.label("Snap", row(y));
        let snaps: Vec<String> = SNAPS.iter().map(|s| format!("1/{s}")).collect();
        let mut snap = self.snap;
        if self.ui.dropdown("snap", field(y), &snaps, &mut snap) {
            self.snap = snap;
        }
        y += ROW;
        self.ui.label("Zoom", row(y));
        let mut zoom = self.zoom;
        if self.ui.stepper("zoom", field(y), &mut zoom, 0.25, 0.25, 4.0, 2) {
            self.zoom = zoom;
        }
        y += ROW;
        self.ui.label("Playback rate", row(y));
        self.ui.stepper("rate", field(y), &mut settings.rate, 0.05, 0.25, 2.0, 2);
        y += ROW;
        self.ui.label("Inst volume", row(y));
        self.ui.stepper("instvol", field(y), &mut settings.inst_volume, 0.05, 0.0, 1.0, 2);
        y += ROW;
        self.ui.label("Vocals volume", row(y));
        self.ui.stepper("voicevol", field(y), &mut settings.voices_volume, 0.05, 0.0, 1.0, 2);
        y += ROW;
        self.ui.label("Waveform", row(y));
        let waves = options(&["Off", "Instrumental", "Vocals"]);
        self.ui.dropdown("wave", field(y), &waves, &mut settings.waveform);
        y += ROW;
        self.ui.checkbox("hitplayer", row(y), "Hit sounds on player notes", &mut settings.hit_player);
        y += ROW;
        self.ui.checkbox("hitopp", row(y), "Hit sounds on opponent notes", &mut settings.hit_opponent);
        y += ROW;
        self.ui.checkbox("metronome", row(y), "Metronome", &mut settings.metronome);
        y += ROW + 10.0;
        self.ui.text("Live charting: while the song plays, press 1-8 to drop notes", X, y, 16.0, TEXT_DIM);
        self.ui.text("on the playhead. Notes snap to the current snap.", X, y + 20.0, 16.0, TEXT_DIM);
        let rate_changed = settings.rate != self.settings.rate;
        self.settings = settings;
        self.apply_audio_settings();
        if rate_changed && self.playing {
            let t = self.time;
            self.seek(ctx, t);
        }
    }
}