use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct ChartNote {
    pub time: f64,
    pub data: i64,
    pub length: f64,
    pub alt: bool,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub notes: Vec<ChartNote>,
    pub length_in_steps: i64,
    pub must_hit: bool,
    pub bpm: f64,
    pub change_bpm: bool,
    pub alt_anim: bool,
}

impl Default for Section {
    fn default() -> Self {
        Section { notes: Vec::new(), length_in_steps: 16, must_hit: true, bpm: 0.0, change_bpm: false, alt_anim: false }
    }
}

#[derive(Clone, Debug)]
pub struct Song {
    pub id: String,
    pub name: String,
    pub notes: Vec<Section>,
    pub bpm: f64,
    pub needs_voices: bool,
    pub speed: f64,
    pub player1: String,
    pub player2: String,
    pub valid_score: bool,
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

impl Song {
    pub fn parse(id: &str, raw: &str) -> Option<Song> {
        let mut text = raw.trim();
        while !text.is_empty() && !text.ends_with('}') {
            text = &text[..text.len() - 1];
        }
        let root: Value = serde_json::from_str(text).ok()?;
        let song = root.get("song").filter(|s| s.is_object()).unwrap_or(&root);
        let mut notes = Vec::new();
        if let Some(sections) = song.get("notes").and_then(|n| n.as_array()) {
            for s in sections {
                let mut section = Section {
                    length_in_steps: s.get("lengthInSteps").map(num).map(|v| v as i64).filter(|v| *v > 0).unwrap_or(16),
                    must_hit: s.get("mustHitSection").map(|v| truthy(Some(v))).unwrap_or(true),
                    bpm: s.get("bpm").map(num).unwrap_or(0.0),
                    change_bpm: truthy(s.get("changeBPM")),
                    alt_anim: truthy(s.get("altAnim")),
                    notes: Vec::new(),
                };
                if let Some(list) = s.get("sectionNotes").and_then(|n| n.as_array()) {
                    for n in list.iter().filter_map(|n| n.as_array()) {
                        if n.len() < 2 {
                            continue;
                        }
                        section.notes.push(ChartNote {
                            time: num(&n[0]),
                            data: num(&n[1]) as i64,
                            length: n.get(2).map(num).unwrap_or(0.0),
                            alt: truthy(n.get(3)),
                        });
                    }
                }
                notes.push(section);
            }
        }
        let text_field = |key: &str, default: &str| song.get(key).and_then(|v| v.as_str()).unwrap_or(default).to_string();
        Some(Song {
            id: id.to_lowercase(),
            name: text_field("song", id),
            notes,
            bpm: song.get("bpm").map(num).filter(|b| *b > 0.0).unwrap_or(100.0),
            needs_voices: song.get("needsVoices").map(|v| truthy(Some(v))).unwrap_or(true),
            speed: song.get("speed").map(num).filter(|s| *s > 0.0).unwrap_or(1.0),
            player1: text_field("player1", "bf"),
            player2: text_field("player2", "dad"),
            valid_score: true,
        })
    }
}

pub fn chart_file(song: &str, difficulty: usize) -> String {
    let id = song.to_lowercase();
    match difficulty {
        0 => format!("{id}-easy"),
        2 => format!("{id}-hard"),
        _ => id,
    }
}