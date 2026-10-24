#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod game;

use miniquad::conf::{Conf, Icon, Platform};

fn pixels<const N: usize>(png: &[u8]) -> [u8; N] {
    let mut out = [0; N];
    if let Ok(image) = image::load_from_memory(png) {
        let rgba = image.to_rgba8().into_raw();
        let len = rgba.len().min(N);
        out[..len].copy_from_slice(&rgba[..len]);
    }
    out
}

fn main() {
    let samples = game::msaa_samples();
    let icon = Icon {
        small: pixels(include_bytes!("../assets/art/icon16.png")),
        medium: pixels(include_bytes!("../assets/art/icon32.png")),
        big: pixels(include_bytes!("../assets/art/icon64.png")),
    };
    let conf = Conf {
        window_title: "Friday Night Funkin'".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        fullscreen: false,
        sample_count: samples,
        window_resizable: true,
        icon: Some(icon),
        platform: Platform { swap_interval: Some(0), ..Default::default() },
    };
    miniquad::start(conf, || Box::new(game::boot()));
}