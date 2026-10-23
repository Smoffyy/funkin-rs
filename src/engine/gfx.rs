use std::collections::HashMap;

use miniquad::*;

pub const GAME_WIDTH: f64 = 1280.0;
pub const GAME_HEIGHT: f64 = 720.0;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub mul: [f32; 4],
    pub add: [f32; 4],
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Effect {
    #[default]
    None,
    Hue(f32),
    Fade(f32),
}

impl Effect {
    fn uniform(self) -> [f32; 4] {
        match self {
            Effect::None => [0.0; 4],
            Effect::Hue(shift) => [shift, 0.0, 1.0, 0.0],
            Effect::Fade(amount) => [0.0, amount, 2.0, 0.0],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

impl Default for Affine {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.tx += dx;
        self.ty += dy;
    }

    pub fn scale(&mut self, sx: f64, sy: f64) {
        self.a *= sx;
        self.b *= sy;
        self.c *= sx;
        self.d *= sy;
        self.tx *= sx;
        self.ty *= sy;
    }

    pub fn rotate(&mut self, cos: f64, sin: f64) {
        let (a, b, c, d, tx, ty) = (self.a, self.b, self.c, self.d, self.tx, self.ty);
        self.a = a * cos - b * sin;
        self.b = a * sin + b * cos;
        self.c = c * cos - d * sin;
        self.d = c * sin + d * cos;
        self.tx = tx * cos - ty * sin;
        self.ty = tx * sin + ty * cos;
    }

    pub fn then(&self, next: &Affine) -> Affine {
        Affine {
            a: self.a * next.a + self.b * next.c,
            b: self.a * next.b + self.b * next.d,
            c: self.c * next.a + self.d * next.c,
            d: self.c * next.b + self.d * next.d,
            tx: self.tx * next.a + self.ty * next.c + next.tx,
            ty: self.tx * next.b + self.ty * next.d + next.ty,
        }
    }

    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (self.a * x + self.c * y + self.tx, self.b * x + self.d * y + self.ty)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Texture {
    pub id: TextureId,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy)]
pub struct Quad {
    pub texture: TextureId,
    pub linear: bool,
    pub effect: Effect,
    pub vertices: [Vertex; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub screen_width: f64,
    pub screen_height: f64,
}

impl View {
    pub fn fit(screen_width: f64, screen_height: f64) -> View {
        let scale = (screen_width / GAME_WIDTH).min(screen_height / GAME_HEIGHT);
        View {
            scale,
            offset_x: ((screen_width - GAME_WIDTH * scale) / 2.0).floor(),
            offset_y: ((screen_height - GAME_HEIGHT * scale) / 2.0).floor(),
            screen_width,
            screen_height,
        }
    }
}

pub struct Layer {
    pub transform: Affine,
    pub scroll: [f64; 2],
    pub scissor: [i32; 4],
    pub background: Option<[f32; 4]>,
    pub quads: Vec<Quad>,
    pub overlays: Vec<[f32; 4]>,
    pub alpha: f32,
}

pub struct DrawList {
    pub view: View,
    pub layers: Vec<Layer>,
    slots: Vec<Option<usize>>,
}

impl DrawList {
    pub fn new(view: View) -> DrawList {
        DrawList { view, layers: Vec::new(), slots: Vec::new() }
    }

    pub fn push_layer(&mut self, slot: usize, transform: Affine, scroll: [f64; 2], rect: [f64; 4], background: Option<[f32; 4]>, alpha: f32) {
        let v = &self.view;
        let x0 = (v.offset_x + rect[0] * v.scale).round();
        let y0 = (v.offset_y + rect[1] * v.scale).round();
        let x1 = (v.offset_x + (rect[0] + rect[2]) * v.scale).round();
        let y1 = (v.offset_y + (rect[1] + rect[3]) * v.scale).round();
        let scissor = [x0 as i32, (v.screen_height - y1) as i32, (x1 - x0).max(0.0) as i32, (y1 - y0).max(0.0) as i32];
        self.layers.push(Layer { transform, scroll, scissor, background, quads: Vec::new(), overlays: Vec::new(), alpha });
        if self.slots.len() <= slot {
            self.slots.resize(slot + 1, None);
        }
        self.slots[slot] = Some(self.layers.len() - 1);
    }

    pub fn layer(&self, slot: usize) -> Option<&Layer> {
        self.slots.get(slot).copied().flatten().map(|i| &self.layers[i])
    }

    pub fn quad(&mut self, slot: usize, quad: Quad) {
        let Some(index) = self.slots.get(slot).copied().flatten() else { return };
        let layer = &mut self.layers[index];
        let mut quad = quad;
        if layer.alpha < 1.0 {
            for v in quad.vertices.iter_mut() {
                v.mul[3] *= layer.alpha;
            }
        }
        layer.quads.push(quad);
    }

    pub fn overlay(&mut self, slot: usize, color: [f32; 4]) {
        if let Some(index) = self.slots.get(slot).copied().flatten() {
            self.layers[index].overlays.push(color);
        }
    }
}

struct Batch {
    texture: TextureId,
    linear: bool,
    effect: Effect,
    scissor: [i32; 4],
    start: usize,
    count: usize,
}

#[repr(C)]
struct Uniforms {
    screen: [f32; 2],
    effect: [f32; 4],
}

pub struct Gfx {
    pub ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    vertex_buffer: BufferId,
    index_buffer: BufferId,
    capacity: usize,
    filters: HashMap<TextureId, bool>,
    white: TextureId,
    vertices: Vec<Vertex>,
    batches: Vec<Batch>,
}

const VERTEX_SHADER: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec2 in_uv;
attribute vec4 in_mul;
attribute vec4 in_add;
uniform vec2 screen;
varying highp vec2 uv;
varying highp vec4 mul;
varying highp vec4 add;
void main() {
    gl_Position = vec4(in_pos.x / screen.x * 2.0 - 1.0, 1.0 - in_pos.y / screen.y * 2.0, 0.0, 1.0);
    uv = in_uv;
    mul = in_mul;
    add = in_add;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
varying highp vec4 mul;
varying highp vec4 add;
uniform sampler2D tex;
uniform vec4 effect;
vec3 rgb2hsv(vec3 c) {
    vec4 k = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, k.wz), vec4(c.gb, k.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));
    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}
vec3 hsv2rgb(vec3 c) {
    vec4 k = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, 0.0, 1.0), c.y);
}
void main() {
    vec4 color = texture2D(tex, uv);
    if (color.a == 0.0) {
        color = vec4(0.0);
    } else {
        color = vec4(color.rgb / color.a, color.a);
        color = clamp(add + color * vec4(mul.rgb, 1.0), 0.0, 1.0);
        color = vec4(color.rgb * color.a * mul.a, color.a * mul.a);
    }
    if (effect.z == 1.0) {
        vec3 hsv = rgb2hsv(color.rgb);
        hsv.x += effect.x;
        color = vec4(hsv2rgb(hsv), color.a);
    } else if (effect.z == 2.0) {
        if (color.a > 0.0) {
            color -= effect.y;
        }
    }
    gl_FragColor = color;
}
"#;

impl Gfx {
    pub fn new() -> Gfx {
        let mut ctx = window::new_rendering_backend();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: FRAGMENT_SHADER },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("screen", UniformType::Float2), UniformDesc::new("effect", UniformType::Float4)],
                    },
                },
            )
            .expect("failed to compile sprite shader");
        let blend = BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha));
        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
                VertexAttribute::new("in_mul", VertexFormat::Float4),
                VertexAttribute::new("in_add", VertexFormat::Float4),
            ],
            shader,
            PipelineParams { color_blend: Some(blend), alpha_blend: Some(blend), ..Default::default() },
        );
        let capacity = 16384;
        let (vertex_buffer, index_buffer) = Self::make_buffers(&mut *ctx, capacity);
        let white = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);
        Gfx {
            ctx,
            pipeline,
            vertex_buffer,
            index_buffer,
            capacity,
            filters: HashMap::new(),
            white,
            vertices: Vec::new(),
            batches: Vec::new(),
        }
    }

    fn make_buffers(ctx: &mut dyn RenderingBackend, capacity: usize) -> (BufferId, BufferId) {
        let vertex_buffer = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vertex>(capacity * 4));
        let mut indices: Vec<u32> = Vec::with_capacity(capacity * 6);
        for i in 0..capacity as u32 {
            let base = i * 4;
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
        }
        let index_buffer = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        (vertex_buffer, index_buffer)
    }

    pub fn create_texture(&mut self, width: u32, height: u32, premultiplied_rgba: &[u8]) -> Texture {
        let id = self.ctx.new_texture_from_rgba8(width as u16, height as u16, premultiplied_rgba);
        self.ctx.texture_set_wrap(id, TextureWrap::Clamp, TextureWrap::Clamp);
        self.filters.insert(id, true);
        Texture { id, width, height }
    }

    pub fn update_texture(&mut self, texture: &mut Texture, width: u32, height: u32, premultiplied_rgba: &[u8]) {
        if texture.width == width && texture.height == height {
            self.ctx.texture_update(texture.id, premultiplied_rgba);
        } else {
            self.ctx.texture_resize(texture.id, width, height, Some(premultiplied_rgba));
            texture.width = width;
            texture.height = height;
        }
    }

    pub fn delete_texture(&mut self, texture: Texture) {
        self.filters.remove(&texture.id);
        self.ctx.delete_texture(texture.id);
    }

    fn push_fill(&mut self, layer: &Layer, color: [f32; 4], view: &View) {
        let [x, y, w, h] = layer.scissor;
        let top = view.screen_height as f32 - (y + h) as f32;
        let (x0, y0, x1, y1) = (x as f32, top, (x + w) as f32, top + h as f32);
        let mul = [color[0], color[1], color[2], color[3] * layer.alpha];
        let vertex = |px: f32, py: f32| Vertex { pos: [px, py], uv: [0.5, 0.5], mul, add: [0.0; 4] };
        self.push_quad(
            &Quad {
                texture: self.white,
                linear: false,
                effect: Effect::None,
                vertices: [vertex(x0, y0), vertex(x1, y0), vertex(x0, y1), vertex(x1, y1)],
            },
            layer.scissor,
        );
    }

    fn push_quad(&mut self, quad: &Quad, scissor: [i32; 4]) {
        let index = self.vertices.len() / 4;
        let extend = match self.batches.last() {
            Some(b) => b.texture == quad.texture && b.linear == quad.linear && b.effect == quad.effect && b.scissor == scissor,
            None => false,
        };
        if extend {
            self.batches.last_mut().unwrap().count += 1;
        } else {
            self.batches.push(Batch { texture: quad.texture, linear: quad.linear, effect: quad.effect, scissor, start: index, count: 1 });
        }
        self.vertices.extend_from_slice(&quad.vertices);
    }

    pub fn render(&mut self, list: &DrawList) {
        self.vertices.clear();
        self.batches.clear();
        let view = list.view;
        for layer in &list.layers {
            if let Some(color) = layer.background {
                if color[3] > 0.0 {
                    self.push_fill(layer, color, &view);
                }
            }
            for quad in &layer.quads {
                self.push_quad(quad, layer.scissor);
            }
            for color in &layer.overlays {
                self.push_fill(layer, *color, &view);
            }
        }
        let quads = self.vertices.len() / 4;
        if quads > self.capacity {
            while self.capacity < quads {
                self.capacity *= 2;
            }
            self.ctx.delete_buffer(self.vertex_buffer);
            self.ctx.delete_buffer(self.index_buffer);
            let (vb, ib) = Self::make_buffers(&mut *self.ctx, self.capacity);
            self.vertex_buffer = vb;
            self.index_buffer = ib;
        }
        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        if !self.vertices.is_empty() {
            self.ctx.buffer_update(self.vertex_buffer, BufferSource::slice(&self.vertices));
            self.ctx.apply_pipeline(&self.pipeline);
            let screen = [view.screen_width as f32, view.screen_height as f32];
            for i in 0..self.batches.len() {
                let (texture, linear, effect, scissor, start, count) = {
                    let b = &self.batches[i];
                    (b.texture, b.linear, b.effect, b.scissor, b.start, b.count)
                };
                if self.filters.get(&texture) != Some(&linear) {
                    let mode = if linear { FilterMode::Linear } else { FilterMode::Nearest };
                    self.ctx.texture_set_filter(texture, mode, MipmapFilterMode::None);
                    self.filters.insert(texture, linear);
                }
                self.ctx.apply_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
                self.ctx.apply_bindings(&Bindings {
                    vertex_buffers: vec![self.vertex_buffer],
                    index_buffer: self.index_buffer,
                    images: vec![texture],
                });
                self.ctx.apply_uniforms(UniformsSource::table(&Uniforms { screen, effect: effect.uniform() }));
                self.ctx.draw((start * 6) as i32, (count * 6) as i32, 1);
            }
        }
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }
}