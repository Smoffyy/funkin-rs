#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Ease {
    #[default]
    Linear,
    QuadOut,
    QuadInOut,
    CubeInOut,
    QuartInOut,
    CircOut,
    ElasticInOut,
}

const ELASTIC_PERIOD: f64 = 0.4;

impl Ease {
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Ease::Linear => t,
            Ease::QuadOut => -t * (t - 2.0),
            Ease::QuadInOut => {
                if t <= 0.5 {
                    t * t * 2.0
                } else {
                    let t = t - 1.0;
                    1.0 - t * t * 2.0
                }
            }
            Ease::CubeInOut => {
                if t <= 0.5 {
                    t * t * t * 4.0
                } else {
                    let t = t - 1.0;
                    1.0 + t * t * t * 4.0
                }
            }
            Ease::QuartInOut => {
                if t <= 0.5 {
                    t * t * t * t * 8.0
                } else {
                    let t = t * 2.0 - 2.0;
                    (1.0 - t * t * t * t) / 2.0 + 0.5
                }
            }
            Ease::CircOut => (1.0 - (t - 1.0) * (t - 1.0)).sqrt(),
            Ease::ElasticInOut => {
                let tau = std::f64::consts::TAU;
                let t = t - 0.5;
                let wave = ((t - ELASTIC_PERIOD / 4.0) * tau / ELASTIC_PERIOD).sin();
                if t < 0.0 {
                    -0.5 * 2f64.powf(10.0 * t) * wave
                } else {
                    2f64.powf(-10.0 * t) * wave * 0.5 + 1.0
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tween {
    pub to: f64,
    pub duration: f64,
    pub delay: f64,
    pub ease: Ease,
    from: Option<f64>,
    time: f64,
    pub finished: bool,
}

impl Tween {
    pub fn new(to: f64, duration: f64) -> Tween {
        Tween { to, duration, delay: 0.0, ease: Ease::Linear, from: None, time: 0.0, finished: false }
    }

    pub fn ease(mut self, ease: Ease) -> Tween {
        self.ease = ease;
        self
    }

    pub fn delay(mut self, delay: f64) -> Tween {
        self.delay = delay;
        self
    }

    pub fn step(&mut self, elapsed: f64, current: f64) -> Option<f64> {
        if self.finished {
            return None;
        }
        self.time += elapsed;
        if self.time < self.delay {
            return None;
        }
        let from = *self.from.get_or_insert(current);
        let mut scale = self.ease.apply((self.time - self.delay) / self.duration.max(1e-9));
        if self.time >= self.duration + self.delay {
            scale = 1.0;
            self.finished = true;
        }
        Some(from + (self.to - from) * scale)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Prop {
    Y,
    Alpha,
}

#[derive(Clone, Debug, Default)]
pub struct TweenSet {
    pub tweens: Vec<(Prop, Tween, u32)>,
    pub completed: Vec<u32>,
}

impl TweenSet {
    pub fn add(&mut self, prop: Prop, tween: Tween, tag: u32) {
        self.tweens.retain(|(p, t, _)| *p != prop || t.finished);
        self.tweens.push((prop, tween, tag));
    }

    pub fn is_active(&self) -> bool {
        self.tweens.iter().any(|(_, t, _)| !t.finished)
    }

    pub fn take_completed(&mut self, tag: u32) -> bool {
        if let Some(i) = self.completed.iter().position(|&t| t == tag) {
            self.completed.remove(i);
            true
        } else {
            false
        }
    }
}