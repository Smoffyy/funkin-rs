use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn set(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }
}

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { state: seed ^ 0x9E37_79B9_7F4A_7C15 }
    }

    pub fn from_time() -> Rng {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Rng::new(nanos)
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn int(&mut self, min: i64, max: i64) -> i64 {
        if min == max {
            return min;
        }
        let (lo, hi) = if min < max { (min, max) } else { (max, min) };
        (lo as f64 + self.unit() * (hi - lo + 1) as f64).floor() as i64
    }

    pub fn float(&mut self, min: f64, max: f64) -> f64 {
        min + self.unit() * (max - min)
    }

    pub fn bool(&mut self, chance: f64) -> bool {
        self.float(0.0, 100.0) < chance
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.int(0, items.len() as i64 - 1) as usize)
        }
    }
}

pub fn lerp(a: f64, b: f64, ratio: f64) -> f64 {
    a + ratio * (b - a)
}

pub fn remap(value: f64, start1: f64, stop1: f64, start2: f64, stop2: f64) -> f64 {
    start2 + (value - start1) * ((stop2 - start2) / (stop1 - start1))
}

pub fn haxe_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

pub fn round_decimal(value: f64, precision: i32) -> f64 {
    let mult = 10f64.powi(precision);
    haxe_round(value * mult) / mult
}

pub fn frame_lerp(elapsed: f64, ratio: f64) -> f64 {
    ratio * (elapsed / (1.0 / 60.0))
}

pub fn decay(ratio: f64, elapsed: f64) -> f64 {
    ratio.powf(elapsed * 60.0)
}

pub fn color_rgba(argb: u32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}

pub fn interpolate_color(from: u32, to: u32, factor: f64) -> u32 {
    let channel = |shift: u32| {
        let a = ((from >> shift) & 0xFF) as f64;
        let b = ((to >> shift) & 0xFF) as f64;
        (((b - a) * factor + a) as i64).clamp(0, 255) as u32
    };
    (channel(24) << 24) | (channel(16) << 16) | (channel(8) << 8) | channel(0)
}