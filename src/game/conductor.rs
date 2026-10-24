use super::song::Song;

#[derive(Clone, Copy, Debug, Default)]
pub struct BpmChange {
    pub step_time: f64,
    pub song_time: f64,
}

#[derive(Clone, Debug)]
pub struct Conductor {
    pub bpm: f64,
    pub crochet: f64,
    pub step_crochet: f64,
    pub song_position: f64,
    pub offset: f64,
    pub safe_zone_offset: f64,
    pub bpm_change_map: Vec<BpmChange>,
}

impl Default for Conductor {
    fn default() -> Self {
        let mut c = Conductor {
            bpm: 100.0,
            crochet: 0.0,
            step_crochet: 0.0,
            song_position: 0.0,
            offset: 0.0,
            safe_zone_offset: (10.0 / 60.0) * 1000.0,
            bpm_change_map: Vec::new(),
        };
        c.change_bpm(100.0);
        c
    }
}

impl Conductor {
    pub fn change_bpm(&mut self, bpm: f64) {
        self.bpm = bpm;
        self.crochet = (60.0 / bpm) * 1000.0;
        self.step_crochet = self.crochet / 4.0;
    }

    pub fn map_bpm_changes(&mut self, song: &Song) {
        self.bpm_change_map.clear();
        let mut steps = 0.0;
        for pair in song.timing.windows(2) {
            steps += (pair[1].time - pair[0].time) / ((60.0 / pair[0].bpm) * 1000.0 / 4.0);
            self.bpm_change_map.push(BpmChange { step_time: steps.round(), song_time: pair[1].time });
        }
    }

    pub fn current_step(&self, inclusive: bool) -> i64 {
        let mut last = BpmChange::default();
        for change in &self.bpm_change_map {
            let reached = if inclusive { self.song_position >= change.song_time } else { self.song_position > change.song_time };
            if reached {
                last = *change;
            }
        }
        last.step_time as i64 + ((self.song_position - last.song_time) / self.step_crochet).floor() as i64
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BeatTracker {
    pub cur_step: i64,
    pub cur_beat: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct BeatEvent {
    pub beat: Option<i64>,
}

impl BeatTracker {
    pub fn update(&mut self, conductor: &Conductor, inclusive: bool) -> Option<BeatEvent> {
        let old = self.cur_step;
        self.cur_step = conductor.current_step(inclusive);
        self.cur_beat = (self.cur_step as f64 / 4.0).floor() as i64;
        if old != self.cur_step && self.cur_step >= 0 {
            let beat = if self.cur_step % 4 == 0 { Some(self.cur_beat) } else { None };
            Some(BeatEvent { beat })
        } else {
            None
        }
    }
}