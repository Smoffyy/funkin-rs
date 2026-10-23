#[derive(Clone, Copy, Debug, Default)]
pub struct Timer {
    pub time: f64,
    pub loops: u32,
    pub active: bool,
    pub finished: bool,
    counter: f64,
    loops_done: u32,
    running: bool,
}

impl Timer {
    pub fn new() -> Timer {
        Timer { finished: true, ..Default::default() }
    }

    pub fn started(time: f64, loops: u32) -> Timer {
        let mut t = Timer::new();
        t.start(time, loops);
        t
    }

    pub fn start(&mut self, time: f64, loops: u32) {
        self.time = time;
        self.loops = loops;
        self.active = true;
        self.finished = false;
        self.counter = 0.0;
        self.loops_done = 0;
        self.running = true;
    }

    pub fn update(&mut self, elapsed: f64) -> bool {
        if !self.running || !self.active || self.finished || self.time < 0.0 {
            return false;
        }
        let before = self.loops_done;
        self.counter += elapsed;
        while self.counter >= self.time && self.active && !self.finished {
            self.counter -= self.time;
            self.loops_done += 1;
            if self.loops > 0 && self.loops_done >= self.loops {
                self.finished = true;
            }
        }
        if before != self.loops_done {
            if self.finished {
                self.running = false;
            }
            true
        } else {
            false
        }
    }
}