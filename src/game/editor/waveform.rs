use kira::sound::static_sound::StaticSoundData;

pub struct Waveform {
    peaks: Vec<f32>,
}

impl Waveform {
    pub fn new(data: &StaticSoundData) -> Waveform {
        let per_ms = (data.sample_rate as f64 / 1000.0).max(1.0);
        let count = (data.frames.len() as f64 / per_ms).ceil() as usize;
        let mut peaks = vec![0.0f32; count.max(1)];
        for (i, frame) in data.frames.iter().enumerate() {
            let bucket = ((i as f64 / per_ms) as usize).min(peaks.len() - 1);
            let level = frame.left.abs().max(frame.right.abs());
            if level > peaks[bucket] {
                peaks[bucket] = level;
            }
        }
        Waveform { peaks }
    }

    pub fn peak(&self, from: f64, to: f64) -> f32 {
        let start = from.max(0.0) as usize;
        let end = (to.max(from + 1.0) as usize).min(self.peaks.len());
        if start >= end {
            return 0.0;
        }
        self.peaks[start..end].iter().copied().fold(0.0, f32::max)
    }
}