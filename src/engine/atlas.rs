use super::gfx::{Affine, Texture};

#[derive(Clone, Debug)]
pub struct Frame {
    pub name: String,
    pub uv: [f64; 4],
    pub width: f64,
    pub height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub source_width: f64,
    pub source_height: f64,
    pub rotated: bool,
    pub empty: bool,
}

impl Frame {
    pub fn base_matrix(&self) -> Affine {
        let mut m = Affine::IDENTITY;
        if self.rotated {
            m = Affine { a: 0.0, b: -1.0, c: 1.0, d: 0.0, tx: 0.0, ty: 0.0 };
            m.translate(0.0, self.width);
        }
        m.translate(self.offset_x, self.offset_y);
        m
    }

    pub fn matrix(&self, flip_x: bool, flip_y: bool) -> Affine {
        let mut m = self.base_matrix();
        if flip_x {
            m.scale(-1.0, 1.0);
            m.translate(self.source_width.trunc(), 0.0);
        }
        if flip_y {
            m.scale(1.0, -1.0);
            m.translate(0.0, self.source_height.trunc());
        }
        m
    }

    pub fn clip_to(&self, clip: [f64; 4]) -> Frame {
        let mut out = self.clone();
        let (w, h) = if self.rotated { (self.height, self.width) } else { (self.width, self.height) };
        let cx = clip[0] - self.offset_x;
        let cy = clip[1] - self.offset_y;
        let x0 = cx.max(0.0);
        let y0 = cy.max(0.0);
        let x1 = (cx + clip[2]).min(w);
        let y1 = (cy + clip[3]).min(h);
        if x1 <= x0 || y1 <= y0 || self.empty {
            out.empty = true;
            out.width = 0.0;
            out.height = 0.0;
            out.offset_x = 0.0;
            out.offset_y = 0.0;
            return out;
        }
        out.offset_x = x0 + self.offset_x;
        out.offset_y = y0 + self.offset_y;
        let (tx0, ty0, tx1, ty1) = if self.rotated {
            (self.width - y1, x0, self.width - y0, x1)
        } else {
            (x0, y0, x1, y1)
        };
        let [u0, v0, u1, v1] = self.uv;
        let du = (u1 - u0) / self.width;
        let dv = (v1 - v0) / self.height;
        out.uv = [u0 + tx0 * du, v0 + ty0 * dv, u0 + tx1 * du, v0 + ty1 * dv];
        out.width = tx1 - tx0;
        out.height = ty1 - ty0;
        out
    }
}

pub struct Frames {
    pub texture: Texture,
    pub frames: Vec<Frame>,
}

impl Frames {
    fn push(&mut self, name: String, rect: [f64; 4], source: [f64; 2], offset: [f64; 2], rotated: bool) {
        if !name.is_empty() && self.frames.iter().any(|f| f.name == name) {
            return;
        }
        let tw = self.texture.width as f64;
        let th = self.texture.height as f64;
        let x = rect[0].clamp(0.0, tw);
        let y = rect[1].clamp(0.0, th);
        let r = (rect[0] + rect[2]).clamp(0.0, tw);
        let b = (rect[1] + rect[3]).clamp(0.0, th);
        self.frames.push(Frame {
            name,
            uv: [x / tw, y / th, r / tw, b / th],
            width: r - x,
            height: b - y,
            offset_x: offset[0],
            offset_y: offset[1],
            source_width: source[0],
            source_height: source[1],
            rotated,
            empty: false,
        });
    }

    pub fn single(texture: Texture) -> Frames {
        let mut frames = Frames { texture, frames: Vec::new() };
        let (w, h) = (texture.width as f64, texture.height as f64);
        frames.push(String::new(), [0.0, 0.0, w, h], [w, h], [0.0, 0.0], false);
        frames
    }

    pub fn solid(texture: Texture, width: f64, height: f64) -> Frames {
        Frames {
            texture,
            frames: vec![Frame {
                name: String::new(),
                uv: [0.5, 0.5, 0.5, 0.5],
                width,
                height,
                offset_x: 0.0,
                offset_y: 0.0,
                source_width: width,
                source_height: height,
                rotated: false,
                empty: false,
            }],
        }
    }

    pub fn grid(texture: Texture, tile_width: u32, tile_height: u32) -> Frames {
        let mut frames = Frames { texture, frames: Vec::new() };
        let cols = texture.width / tile_width.max(1);
        let rows = texture.height / tile_height.max(1);
        let (w, h) = (tile_width as f64, tile_height as f64);
        for row in 0..rows {
            for col in 0..cols {
                let rect = [col as f64 * w, row as f64 * h, w, h];
                frames.frames.push(Frame {
                    name: String::new(),
                    uv: [
                        rect[0] / texture.width as f64,
                        rect[1] / texture.height as f64,
                        (rect[0] + w) / texture.width as f64,
                        (rect[1] + h) / texture.height as f64,
                    ],
                    width: w,
                    height: h,
                    offset_x: 0.0,
                    offset_y: 0.0,
                    source_width: w,
                    source_height: h,
                    rotated: false,
                    empty: false,
                });
            }
        }
        frames
    }

    pub fn sparrow(texture: Texture, xml: &str) -> Result<Frames, String> {
        let mut frames = Frames { texture, frames: Vec::new() };
        for attrs in sub_textures(xml) {
            let get = |key: &str| attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
            let num = |key: &str| get(key).and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(0.0);
            let int = |key: &str| num(key).trunc();
            let name = get("name").unwrap_or("").to_string();
            let trimmed = get("frameX").is_some();
            let rotated = get("rotated") == Some("true");
            let rect = [num("x"), num("y"), num("width"), num("height")];
            let size = if trimmed {
                [int("frameX"), int("frameY"), int("frameWidth"), int("frameHeight")]
            } else {
                [0.0, 0.0, rect[2], rect[3]]
            };
            let mut source = [size[2], size[3]];
            if rotated && !trimmed {
                source = [size[3], size[2]];
            }
            frames.push(name, rect, source, [-size[0], -size[1]], rotated);
        }
        Ok(frames)
    }

    pub fn packer(texture: Texture, text: &str) -> Frames {
        let mut frames = Frames { texture, frames: Vec::new() };
        for line in text.trim().lines() {
            let mut parts = line.splitn(2, '=');
            let name = parts.next().unwrap_or("").trim().to_string();
            let values: Vec<f64> = parts
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(|v| v.parse::<f64>().unwrap_or(0.0))
                .collect();
            if values.len() < 4 {
                continue;
            }
            frames.push(name, [values[0], values[1], values[2], values[3]], [values[2], values[3]], [0.0, 0.0], false);
        }
        frames
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }
}

fn decode(value: &str) -> String {
    value.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn sub_textures(xml: &str) -> Vec<Vec<(String, String)>> {
    let mut out = Vec::new();
    let bytes = xml.as_bytes();
    let mut pos = 0;
    while let Some(found) = xml[pos..].find("<SubTexture") {
        let mut i = pos + found + "<SubTexture".len();
        let mut attrs = Vec::new();
        loop {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] == b'/' || bytes[i] == b'>' {
                break;
            }
            let key_start = i;
            while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let key = xml[key_start..i].to_string();
            while i < bytes.len() && bytes[i] != b'"' && bytes[i] != b'\'' {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            attrs.push((key, decode(&xml[value_start..i.min(bytes.len())])));
            i += 1;
        }
        out.push(attrs);
        pos = i.min(xml.len());
    }
    out
}