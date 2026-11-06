//! Textures painted with the browser's 2D canvas, then uploaded to Bevy.
//!
//! The canvas gives text rendering and anti-aliased strokes for free, which
//! the signs, note and painted details rely on.

use std::f64::consts::{FRAC_PI_3, FRAC_PI_4, TAU};

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// The seeded generator the forest has always used, so the same trees grow
/// in the same places.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 as f64 / 4_294_967_296.0
    }

    pub fn f32(&mut self) -> f32 {
        self.next() as f32
    }
}

pub struct Canvas {
    element: HtmlCanvasElement,
    pub c: CanvasRenderingContext2d,
    pub w: f64,
    pub h: f64,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("textures are painted in a browser document");
        let element: HtmlCanvasElement = document
            .create_element("canvas")
            .expect("canvas element")
            .unchecked_into();
        element.set_width(width);
        element.set_height(height);
        let c: CanvasRenderingContext2d = element
            .get_context("2d")
            .ok()
            .flatten()
            .expect("2D canvas context")
            .unchecked_into();
        Self { element, c, w: width as f64, h: height as f64 }
    }

    pub fn fill(&self, style: &str) {
        self.c.set_fill_style_str(style);
    }

    pub fn stroke(&self, style: &str, width: f64) {
        self.c.set_stroke_style_str(style);
        self.c.set_line_width(width);
    }

    pub fn line(&self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.c.begin_path();
        self.c.move_to(x1, y1);
        self.c.line_to(x2, y2);
        self.c.stroke();
    }

    pub fn text(&self, text: &str, x: f64, y: f64) {
        self.c.fill_text(text, x, y).expect("fillText");
    }

    fn pixels(&self) -> Vec<u8> {
        let (w, h) = (self.element.width(), self.element.height());
        self.c
            .get_image_data(0.0, 0.0, w as f64, h as f64)
            .expect("canvas pixels")
            .data()
            .0
    }

    /// Converts the canvas into a mipmapped sRGB image.
    pub fn into_image(self, repeat: bool) -> Image {
        let (width, height) = (self.element.width(), self.element.height());
        let base = self.pixels();
        let mut image = Image::new(
            Extent3d { width, height, depth_or_array_layers: 1 },
            TextureDimension::D2,
            base.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        let (data, levels) = mip_chain(base, width, height);
        image.data = Some(data);
        image.texture_descriptor.mip_level_count = levels;
        let address = if repeat { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: address,
            address_mode_v: address,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            anisotropy_clamp: 8,
            ..default()
        });
        image
    }
}

/// Appends successively halved levels, averaging color weighted by alpha so
/// cut-out foliage doesn't pick up dark fringes from transparent pixels.
fn mip_chain(base: Vec<u8>, width: u32, height: u32) -> (Vec<u8>, u32) {
    let mut data = base.clone();
    let (mut level, mut w, mut h, mut levels) = (base, width, height, 1);
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = [0.0f32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(w - 1);
                    let sy = (y * 2 + dy).min(h - 1);
                    let i = ((sy * w + sx) * 4) as usize;
                    let alpha = level[i + 3] as f32;
                    for channel in 0..3 {
                        sum[channel] += level[i + channel] as f32 * alpha;
                    }
                    sum[3] += alpha;
                }
                let o = ((y * nw + x) * 4) as usize;
                if sum[3] > 0.0 {
                    for channel in 0..3 {
                        next[o + channel] = (sum[channel] / sum[3]).round() as u8;
                    }
                }
                next[o + 3] = (sum[3] / 4.0).round() as u8;
            }
        }
        data.extend_from_slice(&next);
        (level, w, h) = (next, nw, nh);
        levels += 1;
    }
    (data, levels)
}

/// Individually laid floorboards with grain and staggered seams.
pub fn floor(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(512, 512);
    k.fill("#543e2c");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    for row in 0..8 {
        for column in -1..3 {
            let x = column as f64 * 256.0 + (row % 2) as f64 * 128.0;
            let y = row as f64 * 64.0;
            let value = 45.0 + rng.next() * 15.0;
            k.fill(&format!("hsl(28,27%,{}%)", value - 20.0));
            k.c.fill_rect(x + 1.0, y + 1.0, 254.0, 62.0);
            for _ in 0..50 {
                k.c.set_stroke_style_str(&format!("rgba(20,12,4,{})", rng.next() * 0.15));
                k.c.begin_path();
                let gy = y + rng.next() * 64.0;
                k.c.move_to(x, gy);
                k.c.bezier_curve_to(x + 80.0, gy - 3.0, x + 170.0, gy + 3.0, x + 256.0, gy);
                k.c.stroke();
            }
        }
    }
    k
}

/// Ice crystals that gather along the edges of the window pane.
pub fn frost(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(1024, 512);
    k.c.set_line_width(0.6);
    for _ in 0..6500 {
        let x = rng.next() * k.w;
        let y = rng.next() * k.h;
        let edge = (x / k.w).min(1.0 - x / k.w).min(y / k.h).min(1.0 - y / k.h);
        if edge > 0.085 * rng.next() {
            continue;
        }
        k.c.set_stroke_style_str(&format!("rgba(227,239,244,{})", 0.12 + rng.next() * 0.35));
        let size = 1.0 + rng.next() * 6.0;
        for arm in 0..3 {
            let angle = arm as f64 * FRAC_PI_3;
            let (sin, cos) = angle.sin_cos();
            k.line(x - cos * size, y - sin * size, x + cos * size, y + sin * size);
        }
    }
    k
}

/// A brass-framed sign naming the room behind a door.
pub fn label(text: &str) -> Canvas {
    let k = Canvas::new(512, 150);
    k.fill("#8d7549");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    k.stroke("#d3b982", 4.0);
    k.c.stroke_rect(10.0, 10.0, k.w - 20.0, k.h - 20.0);
    k.c.set_text_align("center");
    k.fill("#f4e5bc");
    k.c.set_font("38px Georgia, serif");
    k.text(text, k.w / 2.0, 82.0);
    k
}

/// Runner rug with a woven border and repeating medallions.
pub fn rug(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(512, 1024);
    let c = &k.c;
    k.fill("#50332c");
    c.fill_rect(0.0, 0.0, k.w, k.h);
    k.stroke("#aa8b5a", 12.0);
    c.stroke_rect(20.0, 20.0, k.w - 40.0, k.h - 40.0);
    c.set_line_width(3.0);
    c.stroke_rect(42.0, 42.0, k.w - 84.0, k.h - 84.0);
    let mut y = 110.0;
    while y < k.h - 50.0 {
        c.save();
        c.translate(k.w / 2.0, y).expect("translate");
        c.scale(1.0, 0.7).expect("scale");
        c.rotate(FRAC_PI_4).expect("rotate");
        for (color, half) in [("#a88b60", 55.0), ("#4e5550", 36.0), ("#b39b70", 12.0)] {
            k.fill(color);
            c.fill_rect(-half, -half, half * 2.0, half * 2.0);
        }
        c.restore();
        y += 155.0;
    }
    for _ in 0..12000 {
        k.fill(if rng.next() > 0.5 { "#ffffff07" } else { "#0000000b" });
        c.fill_rect(rng.next() * k.w, rng.next() * k.h, 1.0, 3.0);
    }
    k
}

/// Woven linen for the lampshade.
pub fn linen() -> Canvas {
    let k = Canvas::new(512, 512);
    k.fill("#dbcfb5");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    for x in (0..512).step_by(3) {
        k.stroke(if x % 9 == 0 { "#a99c8544" } else { "#fff5d955" }, 0.7);
        k.line(x as f64, 0.0, x as f64, k.h);
    }
    for y in (0..512).step_by(3) {
        k.stroke(if y % 9 == 0 { "#8b806133" } else { "#fff5d944" }, 0.65);
        k.line(0.0, y as f64, k.w, y as f64);
    }
    k
}

/// How a bulb's light spreads through fabric or opal glass: brightest at the
/// middle, interrupted slightly by ribs and weave.
pub fn diffusion() -> Canvas {
    let k = Canvas::new(512, 512);
    let glow = k.c.create_linear_gradient(0.0, 0.0, 0.0, k.h);
    for (stop, color) in [(0.0, "#6b6b6b"), (0.16, "#bebebe"), (0.5, "#ffffff"), (0.8, "#c5c5c5"), (1.0, "#757575")] {
        glow.add_color_stop(stop, color).expect("gradient stop");
    }
    k.c.set_fill_style_canvas_gradient(&glow);
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    k.fill("#0000000c");
    for rib in 0..8 {
        k.c.fill_rect(rib as f64 * k.w / 8.0, 0.0, 3.0, k.h);
    }
    k.fill("#00000008");
    for y in (0..512).step_by(3) {
        k.c.fill_rect(0.0, y as f64, k.w, 1.0);
    }
    k
}

pub fn note() -> Canvas {
    let k = Canvas::new(512, 512);
    k.fill("#d4c4a0");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    k.fill("#4a493a");
    k.c.set_text_align("center");
    k.c.set_font("italic 40px Georgia, serif");
    k.text("Welcome home.", k.w / 2.0, 190.0);
    k.c.set_font("25px Georgia, serif");
    k.text("Leave the weather outside.", k.w / 2.0, 270.0);
    k
}

/// A small landscape of layered green hills under a pale sun.
pub fn painting() -> Canvas {
    let k = Canvas::new(512, 512);
    k.fill("#9aab9e");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    for (layer, color) in ["#7b8b7b", "#5d7367", "#425c51", "#354d40"].into_iter().enumerate() {
        let j = layer as f64;
        k.fill(color);
        k.c.begin_path();
        k.c.move_to(0.0, k.h);
        let mut x = 0.0;
        while x <= k.w {
            k.c.line_to(x, 190.0 + j * 65.0 + (x * 0.012 + j * 2.0).sin() * 42.0 + (x * 0.025).sin() * 16.0);
            x += 8.0;
        }
        k.c.line_to(k.w, k.h);
        k.c.fill();
    }
    k.fill("#c1c0a3");
    k.c.begin_path();
    k.c.arc(340.0, 110.0, 28.0, 0.0, TAU).expect("arc");
    k.c.fill();
    k
}

pub fn bark(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(256, 512);
    k.fill("#514d46");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    for _ in 0..2600 {
        let light = 35.0 + rng.next() * 55.0;
        k.c.set_stroke_style_str(&format!("rgb({},{},{})", light + 8.0, light + 4.0, light));
        k.c.set_line_width(0.5 + rng.next() * 3.0);
        let x = rng.next() * k.w;
        let y = rng.next() * k.h;
        let end_x = x + rng.next() * 5.0 - 2.5;
        let end_y = y + 8.0 + rng.next() * 65.0;
        k.line(x, y, end_x, end_y);
    }
    k
}

/// A fir frond: individually drawn needles, irregular tips, and snow caught
/// along the top of each twig.
pub fn needles(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(512, 192);
    let (w, h, c) = (k.w, k.h, &k.c);
    c.set_line_cap("round");
    k.stroke("#514a37", 5.0);
    c.begin_path();
    c.move_to(12.0, h * 0.52);
    c.quadratic_curve_to(w * 0.5, h * 0.5, w - 10.0, h * 0.58);
    c.stroke();
    let needle_colors = ["#293d32", "#405749", "#617563", "#718473"];
    let snow_colors = ["#cbd6d9", "#e5ecec", "#f2f5f2"];
    for i in 0..29 {
        let x = 15.0 + i as f64 * (w - 35.0) / 29.0;
        let t = i as f64 / 29.0;
        let length = (1.0 - t * 0.78) * (45.0 + rng.next() * 28.0);
        for side in [-1.0, 1.0] {
            let end_x = x + length * 0.58;
            let end_y = h * 0.52 + side * length;
            k.stroke("#4a5140", 1.7);
            k.line(x, h * 0.52, end_x, end_y);
            for j in 0..15 {
                let f = j as f64 / 15.0;
                let px = x + (end_x - x) * f;
                let py = h * 0.52 + (end_y - h * 0.52) * f;
                let len = 6.0 + rng.next() * 12.0;
                c.set_stroke_style_str(needle_colors[(rng.next() * 4.0) as usize]);
                c.set_line_width(0.8 + rng.next());
                for spread in [-1.0, 1.0] {
                    k.line(px, py, px + len * 0.75, py + side * len * 0.4 + spread * len * 0.55);
                }
            }
            if rng.next() > 0.16 {
                c.set_stroke_style_str(snow_colors[i % 3]);
                c.set_line_width(3.0 + rng.next() * 5.0);
                k.line(x + 4.0, h * 0.52 - 3.0, end_x * 0.91 + x * 0.09, end_y - 3.0);
            }
        }
    }
    k.stroke("#e3e9e7", 3.0);
    c.begin_path();
    c.move_to(12.0, h * 0.52 - 3.0);
    c.quadratic_curve_to(w * 0.45, h * 0.49, w * 0.8, h * 0.55 - 3.0);
    c.stroke();
    k
}

pub fn snow_ground(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(512, 512);
    k.fill("#d2dcdf");
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    for _ in 0..12000 {
        let v = 180.0 + rng.next() * 65.0;
        k.fill(&format!("rgba({},{},{},.18)", v, v + 3.0, (v + 8.0).min(255.0)));
        let (x, y, len) = (rng.next() * k.w, rng.next() * k.h, 1.0 + rng.next() * 3.0);
        k.c.fill_rect(x, y, len, 1.0);
    }
    k
}

/// A soft round snowflake sprite.
pub fn flake() -> Canvas {
    let k = Canvas::new(64, 64);
    let gradient = k
        .c
        .create_radial_gradient(k.w / 2.0, k.h / 2.0, 0.0, k.w / 2.0, k.h / 2.0, k.w / 2.0)
        .expect("radial gradient");
    for (stop, color) in [
        (0.0, "rgba(255,255,255,1)"),
        (0.23, "rgba(255,255,255,.95)"),
        (0.58, "rgba(255,255,255,.45)"),
        (1.0, "rgba(255,255,255,0)"),
    ] {
        gradient.add_color_stop(stop, color).expect("gradient stop");
    }
    k.c.set_fill_style_canvas_gradient(&gradient);
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    k
}
