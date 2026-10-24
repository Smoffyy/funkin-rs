use serde_json::{Map, Value};

pub const LANES: [&str; 4] = ["left", "down", "up", "right"];
pub const DIFFICULTY_IDS: [&str; 3] = ["easy", "normal", "hard"];
pub const FORMAT: &str = "funkin-rs-chart";

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Side {
    Opponent,
    Player,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Opponent => "opponent",
            Side::Player => "player",
        }
    }

    pub fn other(self) -> Side {
        match self {
            Side::Opponent => Side::Player,
            Side::Player => Side::Opponent,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartNote {
    pub time: f64,
    pub side: Side,
    pub lane: usize,
    pub length: f64,
    pub alt: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimingPoint {
    pub time: f64,
    pub bpm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartEvent {
    pub time: f64,
    pub name: String,
    pub params: Map<String, Value>,
}

impl ChartEvent {
    pub fn new(time: f64, name: &str) -> ChartEvent {
        ChartEvent { time, name: name.to_string(), params: Map::new() }
    }

    pub fn text(&self, key: &str) -> &str {
        self.params.get(key).and_then(|v| v.as_str()).unwrap_or("")
    }

    pub fn number(&self, key: &str, default: f64) -> f64 {
        self.params.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drain {
    pub amount: f64,
    pub floor: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Difficulty {
    pub name: String,
    pub speed: f64,
    pub notes: Vec<ChartNote>,
    pub inst: Option<String>,
    pub voices: Option<String>,
    pub timing: Option<Vec<TimingPoint>>,
    pub events: Option<Vec<ChartEvent>>,
    pub drain: Option<Drain>,
}

impl Difficulty {
    pub fn new(name: &str, speed: f64) -> Difficulty {
        Difficulty { name: difficulty_id(name), speed, notes: Vec::new(), inst: None, voices: None, timing: None, events: None, drain: None }
    }
}

pub fn difficulty_id(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "-")
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartFile {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub charter: String,
    pub stage: Option<String>,
    pub player: String,
    pub opponent: String,
    pub girlfriend: Option<String>,
    pub voices: bool,
    pub offset: f64,
    pub timing: Vec<TimingPoint>,
    pub events: Vec<ChartEvent>,
    pub difficulties: Vec<Difficulty>,
    pub valid_score: bool,
}

#[derive(Clone, Debug)]
pub struct Song {
    pub id: String,
    pub name: String,
    pub difficulty: String,
    pub player1: String,
    pub player2: String,
    pub girlfriend: Option<String>,
    pub stage: Option<String>,
    pub needs_voices: bool,
    pub speed: f64,
    pub offset: f64,
    pub timing: Vec<TimingPoint>,
    pub notes: Vec<ChartNote>,
    pub events: Vec<ChartEvent>,
    pub inst: Option<String>,
    pub voices: Option<String>,
    pub valid_score: bool,
    pub drain: Option<Drain>,
}

impl Song {
    pub fn bpm(&self) -> f64 {
        self.timing.first().map(|t| t.bpm).unwrap_or(100.0)
    }

    pub fn bpm_at(&self, time: f64) -> f64 {
        bpm_at(&self.timing, time)
    }
}

pub fn bpm_at(timing: &[TimingPoint], time: f64) -> f64 {
    let mut bpm = timing.first().map(|t| t.bpm).unwrap_or(100.0);
    for point in timing {
        if point.time <= time {
            bpm = point.bpm;
        }
    }
    bpm
}

pub fn step_crochet(bpm: f64) -> f64 {
    60000.0 / bpm / 4.0
}

pub fn step_at(timing: &[TimingPoint], time: f64) -> f64 {
    let mut steps = 0.0;
    for (i, point) in timing.iter().enumerate() {
        let crochet = step_crochet(point.bpm);
        match timing.get(i + 1) {
            Some(next) if time >= next.time => steps += (next.time - point.time) / crochet,
            _ => return steps + (time - point.time) / crochet,
        }
    }
    time / step_crochet(100.0)
}

pub fn time_at(timing: &[TimingPoint], step: f64) -> f64 {
    let mut steps = 0.0;
    for (i, point) in timing.iter().enumerate() {
        let crochet = step_crochet(point.bpm);
        match timing.get(i + 1) {
            Some(next) => {
                let span = (next.time - point.time) / crochet;
                if step < steps + span {
                    return point.time + (step - steps) * crochet;
                }
                steps += span;
            }
            None => return point.time + (step - steps) * crochet,
        }
    }
    step * step_crochet(100.0)
}

pub fn song_id(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "-")
}

fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        Value::Bool(b) => *b as i64 as f64,
        _ => 0.0,
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::String(s)) => !s.is_empty(),
        _ => false,
    }
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

fn number(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded.fract() == 0.0 { format!("{}", rounded as i64) } else { format!("{rounded}") }
}

fn quote(value: &str) -> String {
    Value::String(value.to_string()).to_string()
}

fn trim_json(raw: &str) -> &str {
    let mut text = raw.trim();
    while !text.is_empty() && !text.ends_with('}') {
        text = &text[..text.len() - 1];
    }
    text
}

fn parse_timing(value: Option<&Value>) -> Option<Vec<TimingPoint>> {
    let mut timing: Vec<TimingPoint> = value?
        .as_array()?
        .iter()
        .map(|p| TimingPoint { time: p.get("time").map(num).unwrap_or(0.0), bpm: p.get("bpm").map(num).unwrap_or(100.0) })
        .filter(|p| p.bpm > 0.0)
        .collect();
    timing.sort_by(|a, b| a.time.total_cmp(&b.time));
    if timing.is_empty() { None } else { Some(timing) }
}

fn parse_events(value: Option<&Value>) -> Option<Vec<ChartEvent>> {
    let list = value?.as_array()?;
    let mut events = Vec::new();
    for e in list {
        let Some(object) = e.as_object() else { continue };
        let mut event = ChartEvent::new(object.get("time").map(num).unwrap_or(0.0), object.get("name").and_then(|n| n.as_str()).unwrap_or(""));
        for (key, value) in object {
            if key != "time" && key != "name" {
                event.params.insert(key.clone(), value.clone());
            }
        }
        events.push(event);
    }
    Some(events)
}

fn parse_drain(value: Option<&Value>) -> Option<Drain> {
    let v = value?;
    Some(Drain { amount: v.get("amount").map(num).unwrap_or(0.023), floor: v.get("floor").map(num).unwrap_or(25.0) })
}

fn parse_difficulty(name: &str, d: &Value) -> Difficulty {
    let mut difficulty = Difficulty::new(name, d.get("speed").map(num).filter(|s| *s > 0.0).unwrap_or(1.0));
    difficulty.notes = d
        .get("notes")
        .and_then(|n| n.as_array())
        .map(|list| {
            list.iter()
                .map(|n| ChartNote {
                    time: n.get("time").map(num).unwrap_or(0.0),
                    side: if n.get("side").and_then(|s| s.as_str()) == Some("player") { Side::Player } else { Side::Opponent },
                    lane: n.get("lane").and_then(|l| l.as_str()).and_then(|l| LANES.iter().position(|x| *x == l)).unwrap_or(0),
                    length: n.get("length").map(num).unwrap_or(0.0).max(0.0),
                    alt: n.get("type").and_then(|t| t.as_str()) == Some("alt"),
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(audio) = d.get("audio") {
        difficulty.inst = text(audio, "inst");
        difficulty.voices = text(audio, "voices");
    }
    difficulty.timing = parse_timing(d.get("timing"));
    difficulty.events = parse_events(d.get("events"));
    difficulty.drain = parse_drain(d.get("opponent_drain"));
    difficulty
}

fn write_timing(timing: &[TimingPoint], indent: &str) -> String {
    let lines: Vec<String> = timing.iter().map(|t| format!("{indent}\t{{\"time\": {}, \"bpm\": {}}}", number(t.time), number(t.bpm))).collect();
    format!("[\n{}\n{indent}]", lines.join(",\n"))
}

fn write_events(events: &[ChartEvent], indent: &str) -> String {
    let mut events = events.to_vec();
    events.sort_by(|a, b| a.time.total_cmp(&b.time));
    if events.is_empty() {
        return "[]".to_string();
    }
    let lines: Vec<String> = events
        .iter()
        .map(|e| {
            let mut fields = vec![format!("\"time\": {}", number(e.time)), format!("\"name\": {}", quote(&e.name))];
            for (key, value) in &e.params {
                fields.push(format!("{}: {}", quote(key), value));
            }
            format!("{indent}\t{{{}}}", fields.join(", "))
        })
        .collect();
    format!("[\n{}\n{indent}]", lines.join(",\n"))
}

fn write_difficulty(d: &Difficulty) -> String {
    let mut fields = vec![format!("\t\t\t\"name\": {}", quote(&d.name)), format!("\t\t\t\"speed\": {}", number(d.speed))];
    if d.inst.is_some() || d.voices.is_some() {
        let mut audio = Vec::new();
        if let Some(inst) = &d.inst {
            audio.push(format!("\"inst\": {}", quote(inst)));
        }
        if let Some(voices) = &d.voices {
            audio.push(format!("\"voices\": {}", quote(voices)));
        }
        fields.push(format!("\t\t\t\"audio\": {{{}}}", audio.join(", ")));
    }
    if let Some(drain) = &d.drain {
        fields.push(format!("\t\t\t\"opponent_drain\": {{\"amount\": {}, \"floor\": {}}}", number(drain.amount), number(drain.floor)));
    }
    if let Some(timing) = &d.timing {
        fields.push(format!("\t\t\t\"timing\": {}", write_timing(timing, "\t\t\t")));
    }
    if let Some(events) = &d.events {
        fields.push(format!("\t\t\t\"events\": {}", write_events(events, "\t\t\t")));
    }
    let mut notes = d.notes.clone();
    notes.sort_by(|a, b| a.time.total_cmp(&b.time).then(a.side.name().cmp(b.side.name())).then(a.lane.cmp(&b.lane)));
    let lines: Vec<String> = notes
        .iter()
        .map(|n| {
            let kind = if n.alt { ", \"type\": \"alt\"" } else { "" };
            format!("\t\t\t\t{{\"time\": {}, \"side\": {}, \"lane\": {}, \"length\": {}{kind}}}", number(n.time), quote(n.side.name()), quote(LANES[n.lane]), number(n.length))
        })
        .collect();
    let notes = if lines.is_empty() { "[]".to_string() } else { format!("[\n{}\n\t\t\t]", lines.join(",\n")) };
    fields.push(format!("\t\t\t\"notes\": {notes}"));
    format!("\t\t{{\n{}\n\t\t}}", fields.join(",\n"))
}

impl ChartFile {
    pub fn new(name: &str) -> ChartFile {
        ChartFile {
            id: song_id(name),
            name: name.to_string(),
            artist: String::new(),
            charter: String::new(),
            stage: None,
            player: "bf".to_string(),
            opponent: "dad".to_string(),
            girlfriend: None,
            voices: true,
            offset: 0.0,
            timing: vec![TimingPoint { time: 0.0, bpm: 150.0 }],
            events: vec![],
            difficulties: vec![Difficulty::new("normal", 1.0)],
            valid_score: true,
        }
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        let id = difficulty_id(name);
        self.difficulties.iter().position(|d| d.name == id)
    }

    pub fn pick(&self, name: &str) -> Option<&Difficulty> {
        self.find(name).or_else(|| self.find("normal")).map(|i| &self.difficulties[i]).or(self.difficulties.first())
    }

    pub fn names(&self) -> Vec<String> {
        self.difficulties.iter().map(|d| d.name.clone()).collect()
    }

    pub fn song(&self, difficulty: &str) -> Option<Song> {
        let chart = self.pick(difficulty)?;
        let mut notes = chart.notes.clone();
        notes.sort_by(|a, b| a.time.total_cmp(&b.time));
        let mut events = chart.events.clone().unwrap_or_else(|| self.events.clone());
        events.sort_by(|a, b| a.time.total_cmp(&b.time));
        Some(Song {
            id: self.id.clone(),
            name: self.name.clone(),
            difficulty: chart.name.clone(),
            player1: self.player.clone(),
            player2: self.opponent.clone(),
            girlfriend: self.girlfriend.clone(),
            stage: self.stage.clone(),
            needs_voices: self.voices,
            speed: chart.speed,
            offset: self.offset,
            timing: chart.timing.clone().unwrap_or_else(|| self.timing.clone()),
            notes,
            events,
            inst: chart.inst.clone(),
            voices: chart.voices.clone(),
            valid_score: self.valid_score,
            drain: chart.drain,
        })
    }

    pub fn parse(id: &str, raw: &str) -> Option<ChartFile> {
        let root: Value = serde_json::from_str(trim_json(raw)).ok()?;
        if root.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
            return None;
        }
        let meta = root.get("song").cloned().unwrap_or(Value::Null);
        let mut chart = ChartFile::new(&text(&meta, "name").unwrap_or_else(|| id.to_string()));
        chart.id = id.to_string();
        chart.artist = text(&meta, "artist").unwrap_or_default();
        chart.charter = text(&meta, "charter").unwrap_or_default();
        chart.stage = text(&meta, "stage");
        chart.player = text(&meta, "player").unwrap_or_else(|| "bf".to_string());
        chart.opponent = text(&meta, "opponent").unwrap_or_else(|| "dad".to_string());
        chart.girlfriend = text(&meta, "girlfriend");
        chart.voices = meta.get("voices").map(|v| truthy(Some(v))).unwrap_or(true);
        chart.offset = meta.get("offset").map(num).unwrap_or(0.0);
        if let Some(timing) = parse_timing(root.get("timing")) {
            chart.timing = timing;
        }
        chart.events = parse_events(root.get("events")).unwrap_or_default();
        chart.difficulties.clear();
        match root.get("difficulties") {
            Some(Value::Array(list)) => {
                for d in list {
                    let name = text(d, "name").unwrap_or_else(|| format!("difficulty-{}", chart.difficulties.len() + 1));
                    chart.difficulties.push(parse_difficulty(&name, d));
                }
            }
            Some(Value::Object(map)) => {
                for key in DIFFICULTY_IDS {
                    if let Some(d) = map.get(key) {
                        chart.difficulties.push(parse_difficulty(key, d));
                    }
                }
                for (key, d) in map {
                    if !DIFFICULTY_IDS.contains(&key.as_str()) {
                        chart.difficulties.push(parse_difficulty(key, d));
                    }
                }
            }
            _ => {}
        }
        if chart.difficulties.is_empty() {
            chart.difficulties.push(Difficulty::new("normal", 1.0));
        }
        Some(chart)
    }

    pub fn write(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        out.push_str(&format!("\t\"format\": {},\n\t\"version\": 2,\n", quote(FORMAT)));
        out.push_str("\t\"song\": {\n");
        let mut meta = vec![
            format!("\"name\": {}", quote(&self.name)),
            format!("\"artist\": {}", quote(&self.artist)),
            format!("\"charter\": {}", quote(&self.charter)),
        ];
        if let Some(stage) = &self.stage {
            meta.push(format!("\"stage\": {}", quote(stage)));
        }
        meta.push(format!("\"player\": {}", quote(&self.player)));
        meta.push(format!("\"opponent\": {}", quote(&self.opponent)));
        if let Some(gf) = &self.girlfriend {
            meta.push(format!("\"girlfriend\": {}", quote(gf)));
        }
        meta.push(format!("\"voices\": {}", self.voices));
        meta.push(format!("\"offset\": {}", number(self.offset)));
        out.push_str(&meta.iter().map(|m| format!("\t\t{m}")).collect::<Vec<_>>().join(",\n"));
        out.push_str("\n\t},\n");
        out.push_str(&format!("\t\"timing\": {},\n", write_timing(&self.timing, "\t")));
        out.push_str(&format!("\t\"events\": {},\n", write_events(&self.events, "\t")));
        let blocks: Vec<String> = self.difficulties.iter().map(write_difficulty).collect();
        out.push_str(&format!("\t\"difficulties\": [\n{}\n\t]\n}}", blocks.join(",\n")));
        out
    }

    pub fn from_legacy(id: &str, files: [Option<String>; 3]) -> Option<ChartFile> {
        let parsed: Vec<Option<Value>> = files
            .iter()
            .map(|f| f.as_ref().and_then(|t| serde_json::from_str::<Value>(trim_json(t)).ok()))
            .map(|v| v.map(|root| root.get("song").filter(|s| s.is_object()).cloned().unwrap_or(root)))
            .collect();
        let main = parsed[1].as_ref().or(parsed.iter().flatten().next())?;
        let mut chart = ChartFile::new(&text(main, "song").unwrap_or_else(|| id.to_string()));
        chart.id = id.to_string();
        chart.player = text(main, "player1").unwrap_or_else(|| "bf".to_string());
        chart.opponent = text(main, "player2").unwrap_or_else(|| "dad".to_string());
        chart.voices = main.get("needsVoices").map(|v| truthy(Some(v))).unwrap_or(true);
        chart.valid_score = main.get("validScore").map(|v| truthy(Some(v))).unwrap_or(true);
        let (timing, events) = legacy_timing(main);
        chart.timing = timing;
        chart.events = events;
        chart.difficulties = parsed
            .iter()
            .zip(DIFFICULTY_IDS)
            .filter_map(|(song, name)| song.as_ref().map(|s| legacy_notes(name, s)))
            .collect();
        Some(chart)
    }
}

fn sections(song: &Value) -> Vec<Value> {
    song.get("notes").and_then(|n| n.as_array()).cloned().unwrap_or_default()
}

fn legacy_timing(song: &Value) -> (Vec<TimingPoint>, Vec<ChartEvent>) {
    let mut bpm = song.get("bpm").map(num).filter(|b| *b > 0.0).unwrap_or(100.0);
    let mut timing = vec![TimingPoint { time: 0.0, bpm }];
    let mut events = Vec::new();
    let mut time = 0.0;
    let mut focus: Option<bool> = None;
    for section in sections(song) {
        let length = section.get("lengthInSteps").map(num).filter(|v| *v > 0.0).unwrap_or(16.0);
        if truthy(section.get("changeBPM")) {
            let next = section.get("bpm").map(num).unwrap_or(bpm);
            if next > 0.0 && next != bpm {
                bpm = next;
                timing.push(TimingPoint { time, bpm });
            }
        }
        let must_hit = section.get("mustHitSection").map(|v| truthy(Some(v))).unwrap_or(true);
        if focus != Some(must_hit) {
            let mut event = ChartEvent::new(time, "focus");
            let target = if must_hit { Side::Player } else { Side::Opponent };
            event.params.insert("target".to_string(), Value::String(target.name().to_string()));
            events.push(event);
            focus = Some(must_hit);
        }
        time += (60.0 / bpm) * 1000.0 / 4.0 * length;
    }
    (timing, events)
}

fn legacy_notes(name: &str, song: &Value) -> Difficulty {
    let mut notes = Vec::new();
    for section in sections(song) {
        let must_hit = section.get("mustHitSection").map(|v| truthy(Some(v))).unwrap_or(true);
        let alt_section = truthy(section.get("altAnim"));
        let Some(list) = section.get("sectionNotes").and_then(|n| n.as_array()) else { continue };
        for n in list.iter().filter_map(|n| n.as_array()) {
            if n.len() < 2 {
                continue;
            }
            let data = num(&n[1]) as i64;
            let mut player = must_hit;
            if data > 3 {
                player = !must_hit;
            }
            let side = if player { Side::Player } else { Side::Opponent };
            notes.push(ChartNote {
                time: num(&n[0]),
                side,
                lane: data.rem_euclid(4) as usize,
                length: n.get(2).map(num).unwrap_or(0.0).max(0.0),
                alt: truthy(n.get(3)) || (alt_section && side == Side::Opponent),
            });
        }
    }
    let mut difficulty = Difficulty::new(name, song.get("speed").map(num).filter(|s| *s > 0.0).unwrap_or(1.0));
    difficulty.notes = notes;
    difficulty
}