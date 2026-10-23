use std::collections::HashMap;

use miniquad::KeyCode;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Released,
    JustReleased,
    Pressed,
    JustPressed,
}

#[derive(Clone, Copy, Debug)]
struct Input {
    current: State,
    last: State,
    release_queued: bool,
}

impl Input {
    fn pressed(&self) -> bool {
        matches!(self.current, State::Pressed | State::JustPressed)
    }
}

#[derive(Default)]
pub struct Keys {
    inputs: HashMap<KeyCode, Input>,
    pending: Vec<(KeyCode, bool)>,
    pub any_just_pressed: bool,
    first_just_released: Option<KeyCode>,
}

impl Keys {
    pub fn key_event(&mut self, key: KeyCode, down: bool) {
        self.pending.push((key, down));
    }

    pub fn update(&mut self) {
        for input in self.inputs.values_mut() {
            if input.release_queued {
                input.release_queued = false;
                input.last = input.current;
                input.current = State::JustReleased;
            }
        }
        let mut pressed_now: Vec<KeyCode> = Vec::new();
        for (key, down) in std::mem::take(&mut self.pending) {
            let input = self.inputs.entry(key).or_insert(Input { current: State::Released, last: State::Released, release_queued: false });
            if down {
                if input.pressed() {
                    continue;
                }
                input.last = input.current;
                input.current = State::JustPressed;
                input.release_queued = false;
                pressed_now.push(key);
            } else if pressed_now.contains(&key) {
                input.release_queued = true;
            } else if input.pressed() {
                input.last = input.current;
                input.current = State::JustReleased;
            }
        }
        for input in self.inputs.values_mut() {
            if input.last == State::JustReleased && input.current == State::JustReleased {
                input.current = State::Released;
            } else if input.last == State::JustPressed && input.current == State::JustPressed {
                input.current = State::Pressed;
            }
            input.last = input.current;
        }
        self.any_just_pressed = self.inputs.values().any(|i| i.current == State::JustPressed);
        self.first_just_released = self.inputs.iter().find(|(_, i)| i.current == State::JustReleased).map(|(k, _)| *k);
    }

    pub fn pressed(&self, key: KeyCode) -> bool {
        self.inputs.get(&key).map(|i| i.pressed()).unwrap_or(false)
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.inputs.get(&key).map(|i| i.current == State::JustPressed).unwrap_or(false)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        self.inputs.get(&key).map(|i| i.current == State::JustReleased).unwrap_or(false)
    }

    pub fn first_just_released(&self) -> Option<KeyCode> {
        self.first_just_released
    }

    pub fn reset(&mut self) {
        self.inputs.clear();
        self.pending.clear();
        self.any_just_pressed = false;
        self.first_just_released = None;
    }
}

const KEY_NAMES: &[(KeyCode, &str)] = &[
    (KeyCode::A, "A"),
    (KeyCode::B, "B"),
    (KeyCode::C, "C"),
    (KeyCode::D, "D"),
    (KeyCode::E, "E"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::I, "I"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::M, "M"),
    (KeyCode::N, "N"),
    (KeyCode::O, "O"),
    (KeyCode::P, "P"),
    (KeyCode::Q, "Q"),
    (KeyCode::R, "R"),
    (KeyCode::S, "S"),
    (KeyCode::T, "T"),
    (KeyCode::U, "U"),
    (KeyCode::V, "V"),
    (KeyCode::W, "W"),
    (KeyCode::X, "X"),
    (KeyCode::Y, "Y"),
    (KeyCode::Z, "Z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"),
    (KeyCode::Left, "Left"),
    (KeyCode::Right, "Right"),
    (KeyCode::Space, "Space"),
    (KeyCode::Enter, "Enter"),
    (KeyCode::KpEnter, "NumEnter"),
    (KeyCode::Escape, "Escape"),
    (KeyCode::Backspace, "BckSpc"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::LeftShift, "Shift"),
    (KeyCode::RightShift, "RShift"),
    (KeyCode::LeftControl, "Ctrl"),
    (KeyCode::RightControl, "RCtrl"),
    (KeyCode::LeftAlt, "Alt"),
    (KeyCode::RightAlt, "RAlt"),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Slash, "/"),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Backslash, "\\"),
    (KeyCode::GraveAccent, "`"),
    (KeyCode::Minus, "-"),
    (KeyCode::Equal, "="),
    (KeyCode::CapsLock, "Caps"),
    (KeyCode::PageUp, "PgUp"),
    (KeyCode::PageDown, "PgDown"),
    (KeyCode::Home, "Home"),
    (KeyCode::End, "End"),
    (KeyCode::Insert, "Insert"),
    (KeyCode::Delete, "Delete"),
    (KeyCode::Kp0, "#0"),
    (KeyCode::Kp1, "#1"),
    (KeyCode::Kp2, "#2"),
    (KeyCode::Kp3, "#3"),
    (KeyCode::Kp4, "#4"),
    (KeyCode::Kp5, "#5"),
    (KeyCode::Kp6, "#6"),
    (KeyCode::Kp7, "#7"),
    (KeyCode::Kp8, "#8"),
    (KeyCode::Kp9, "#9"),
    (KeyCode::KpAdd, "#+"),
    (KeyCode::KpSubtract, "#-"),
    (KeyCode::KpMultiply, "#*"),
    (KeyCode::KpDecimal, "#."),
    (KeyCode::F1, "F1"),
    (KeyCode::F2, "F2"),
    (KeyCode::F3, "F3"),
    (KeyCode::F4, "F4"),
    (KeyCode::F5, "F5"),
    (KeyCode::F6, "F6"),
    (KeyCode::F7, "F7"),
    (KeyCode::F8, "F8"),
    (KeyCode::F9, "F9"),
    (KeyCode::F10, "F10"),
    (KeyCode::F11, "F11"),
    (KeyCode::F12, "F12"),
];

pub fn key_name(key: KeyCode) -> &'static str {
    KEY_NAMES.iter().find(|(k, _)| *k == key).map(|(_, n)| *n).unwrap_or("?")
}

pub fn key_from_name(name: &str) -> Option<KeyCode> {
    KEY_NAMES.iter().find(|(_, n)| *n == name).map(|(k, _)| *k)
}