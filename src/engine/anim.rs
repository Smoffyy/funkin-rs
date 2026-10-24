use std::collections::HashMap;

use super::atlas::Frames;

pub fn parse_int(text: &str) -> Option<i64> {
    let s = text.trim_start();
    let (negative, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (radix, digits) = match digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
        Some(rest) => (16, rest),
        None => (10, digits),
    };
    let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    let value = i64::from_str_radix(&digits[..end], radix).ok()?;
    Some(if negative { -value } else { value })
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub name: String,
    pub frames: Vec<usize>,
    pub frame_rate: f64,
    pub delay: f64,
    pub cur_frame: usize,
    pub finished: bool,
    pub paused: bool,
    pub looped: bool,
    pub reversed: bool,
    pub flip_x: bool,
    pub flip_y: bool,
    frame_timer: f64,
}

impl Animation {
    pub fn new(name: &str, frames: Vec<usize>, frame_rate: f64, looped: bool, flip_x: bool) -> Animation {
        let mut anim = Animation {
            name: name.to_string(),
            frames,
            frame_rate: 0.0,
            delay: 0.0,
            cur_frame: 0,
            finished: true,
            paused: true,
            looped,
            reversed: false,
            flip_x,
            flip_y: false,
            frame_timer: 0.0,
        };
        anim.set_frame_rate(frame_rate);
        anim
    }

    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    pub fn set_frame_rate(&mut self, value: f64) {
        self.frame_rate = value;
        self.delay = if value > 0.0 { 1.0 / value } else { 0.0 };
    }

    fn assign_frame(&mut self, frame: i64) -> bool {
        let max = self.num_frames() as i64 - 1;
        let temp = if self.reversed { max - frame } else { frame };
        if temp >= 0 {
            if !self.looped && frame > max {
                self.finished = true;
                self.cur_frame = if self.reversed { 0 } else { max.max(0) as usize };
            } else {
                self.cur_frame = frame.clamp(0, max.max(0)) as usize;
            }
        } else {
            self.cur_frame = max.max(0) as usize;
        }
        self.finished
    }

    fn play(&mut self, force: bool, reversed: bool, frame: i64) -> bool {
        if !force && !self.finished && self.reversed == reversed {
            self.paused = false;
            self.finished = false;
            return false;
        }
        self.reversed = reversed;
        self.paused = false;
        self.frame_timer = 0.0;
        self.finished = self.delay == 0.0;
        let max = self.num_frames() as i64 - 1;
        let mut fired = false;
        if frame < 0 {
            self.cur_frame = 0;
        } else {
            let mut target = frame.min(max);
            if reversed {
                target = max - target;
            }
            fired |= self.assign_frame(target);
        }
        fired || self.finished
    }

    fn stop(&mut self) {
        self.finished = true;
        self.paused = true;
    }

    fn update(&mut self, elapsed: f64) -> bool {
        if self.delay == 0.0 || self.finished || self.paused {
            return false;
        }
        let mut fired = false;
        self.frame_timer += elapsed;
        while self.frame_timer > self.delay && !self.finished {
            self.frame_timer -= self.delay;
            let last = self.num_frames() - 1;
            let next = if self.reversed {
                if self.looped && self.cur_frame == 0 { last as i64 } else { self.cur_frame as i64 - 1 }
            } else if self.looped && self.cur_frame == last {
                0
            } else {
                self.cur_frame as i64 + 1
            };
            fired |= self.assign_frame(next);
        }
        fired
    }

    pub fn frame_index(&self) -> usize {
        self.frames.get(self.cur_frame).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnimController {
    anims: Vec<Animation>,
    by_name: HashMap<String, usize>,
    cur: Option<usize>,
    pub frame_index: usize,
    finished_events: Vec<String>,
}

impl AnimController {
    pub fn clear(&mut self) {
        self.anims.clear();
        self.by_name.clear();
        self.cur = None;
        self.frame_index = 0;
    }

    fn insert(&mut self, anim: Animation) {
        match self.by_name.get(&anim.name) {
            Some(&i) => {
                if self.cur == Some(i) {
                    self.cur = None;
                }
                self.anims[i] = anim;
            }
            None => {
                self.by_name.insert(anim.name.clone(), self.anims.len());
                self.anims.push(anim);
            }
        }
    }

    pub fn add(&mut self, frames: &Frames, name: &str, indices: &[usize], frame_rate: f64, looped: bool, flip_x: bool) {
        let list: Vec<usize> = indices.iter().copied().filter(|&i| i < frames.len()).collect();
        if !list.is_empty() {
            self.insert(Animation::new(name, list, frame_rate, looped, flip_x));
        }
    }

    pub fn add_by_prefix(&mut self, frames: &Frames, name: &str, prefix: &str, frame_rate: f64, looped: bool, flip_x: bool) {
        let mut found: Vec<usize> = (0..frames.len()).filter(|&i| frames.frames[i].name.starts_with(prefix)).collect();
        if found.is_empty() {
            return;
        }
        let first = &frames.frames[found[0]].name;
        let post_len = match first[prefix.len().min(first.len())..].find('.') {
            Some(pos) => first.len() - (prefix.len() + pos),
            None => 0,
        };
        let key = |i: usize| {
            let n = &frames.frames[i].name;
            let end = n.len().saturating_sub(post_len).max(prefix.len().min(n.len()));
            n.get(prefix.len()..end).and_then(parse_int).unwrap_or(0)
        };
        found.sort_by_key(|&i| key(i));
        self.insert(Animation::new(name, found, frame_rate, looped, flip_x));
    }

    pub fn add_by_indices(&mut self, frames: &Frames, name: &str, prefix: &str, indices: &[i64], postfix: &str, frame_rate: f64, looped: bool, flip_x: bool) {
        let mut list = Vec::new();
        for &index in indices {
            let hit = frames.frames.iter().position(|f| {
                let n = &f.name;
                n.starts_with(prefix)
                    && n.ends_with(postfix)
                    && n.len() >= prefix.len() + postfix.len()
                    && parse_int(&n[prefix.len()..n.len() - postfix.len()]) == Some(index)
            });
            if let Some(i) = hit {
                list.push(i);
            }
        }
        if !list.is_empty() {
            self.insert(Animation::new(name, list, frame_rate, looped, flip_x));
        }
    }

    pub fn append_by_prefix(&mut self, frames: &Frames, name: &str, prefix: &str) {
        let Some(&i) = self.by_name.get(name) else { return };
        let mut found: Vec<usize> = (0..frames.len()).filter(|&f| frames.frames[f].name.starts_with(prefix)).collect();
        found.sort_by_key(|&f| frames.frames[f].name.get(prefix.len()..).and_then(parse_int).unwrap_or(0));
        self.anims[i].frames.extend(found);
    }

    pub fn play(&mut self, name: &str, force: bool, reversed: bool, frame: i64) {
        let Some(&index) = self.by_name.get(name) else { return };
        if let Some(cur) = self.cur {
            if cur != index {
                self.anims[cur].stop();
            }
        }
        self.cur = Some(index);
        let fired = self.anims[index].play(force, reversed, frame);
        self.frame_index = self.anims[index].frame_index();
        if fired {
            self.finished_events.push(name.to_string());
        }
    }

    pub fn update(&mut self, elapsed: f64) {
        if let Some(cur) = self.cur {
            if self.anims[cur].update(elapsed) {
                self.finished_events.push(self.anims[cur].name.clone());
            }
            self.frame_index = self.anims[cur].frame_index();
        }
    }

    pub fn clear_finished(&mut self) {
        self.finished_events.clear();
    }

    pub fn cur(&self) -> Option<&Animation> {
        self.cur.map(|i| &self.anims[i])
    }

    pub fn name(&self) -> &str {
        self.cur().map(|a| a.name.as_str()).unwrap_or("")
    }

    pub fn finished(&self) -> bool {
        self.cur().map(|a| a.finished).unwrap_or(true)
    }

    pub fn cur_frame(&self) -> usize {
        self.cur().map(|a| a.cur_frame).unwrap_or(0)
    }

    pub fn set_cur_frame(&mut self, frame: usize) {
        if let Some(i) = self.cur {
            if self.anims[i].assign_frame(frame as i64) {
                self.finished_events.push(self.anims[i].name.clone());
            }
            self.frame_index = self.anims[i].frame_index();
        }
    }

    pub fn finish(&mut self) {
        if let Some(i) = self.cur {
            let anim = &mut self.anims[i];
            anim.stop();
            let target = if anim.reversed { 0 } else { anim.num_frames() as i64 - 1 };
            if anim.assign_frame(target) {
                self.finished_events.push(anim.name.clone());
            }
            self.frame_index = self.anims[i].frame_index();
        }
    }

    pub fn pause(&mut self) {
        if let Some(i) = self.cur {
            self.anims[i].paused = true;
        }
    }

    pub fn exists(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    pub fn swap_frames(&mut self, a: &str, b: &str) {
        if let (Some(&ia), Some(&ib)) = (self.by_name.get(a), self.by_name.get(b)) {
            let fa = self.anims[ia].frames.clone();
            let fb = std::mem::replace(&mut self.anims[ib].frames, fa);
            self.anims[ia].frames = fb;
        }
    }
}