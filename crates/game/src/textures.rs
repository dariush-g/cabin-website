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

    pub fn pixels(&self) -> Vec<u8> {
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
        pixels_to_image(self.pixels(), width, height, TextureFormat::Rgba8UnormSrgb, repeat)
    }
}

/// Builds a mipmapped image from RGBA pixels. Data such as normal and depth
/// maps uses a linear format; colour uses sRGB.
pub fn pixels_to_image(base: Vec<u8>, width: u32, height: u32, format: TextureFormat, repeat: bool) -> Image {
    let mut image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        base.clone(),
        format,
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
    boards(rng, "#543e2c", 25.0)
}

fn boards(rng: &mut Rng, seam: &str, lightness: f64) -> Canvas {
    let k = Canvas::new(512, 512);
    k.fill(seam);
    k.c.fill_rect(0.0, 0.0, k.w, k.h);
    for row in 0..8 {
        for column in -1..3 {
            let x = column as f64 * 256.0 + (row % 2) as f64 * 128.0;
            let y = row as f64 * 64.0;
            let value = lightness + rng.next() * 15.0;
            k.fill(&format!("hsl(28,27%,{}%)", value));
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

/// A peeled log: long grain along its length (canvas x), drawknife bands,
/// seasoning checks and a few knots. Tiles in both directions, since `v`
/// wraps around the log.
pub fn log(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(1024, 256);
    let (w, h, c) = (k.w, k.h, &k.c);
    k.fill("hsl(25,40%,21%)");
    c.fill_rect(0.0, 0.0, w, h);
    // Broad bands left by the drawknife, lighter and darker.
    for _ in 0..34 {
        let y = rng.next() * h;
        let band = 6.0 + rng.next() * 26.0;
        let light = if rng.next() > 0.5 { "230,190,140" } else { "25,12,4" };
        k.fill(&format!("rgba({light},{})", 0.04 + rng.next() * 0.08));
        for dy in [-h, 0.0, h] {
            c.fill_rect(0.0, y + dy, w, band);
        }
    }
    // Fine grain lines, gently wavering.
    for _ in 0..900 {
        let y = rng.next() * h;
        let x = rng.next() * w;
        let length = 120.0 + rng.next() * 500.0;
        let sway = rng.next() * 4.0 - 2.0;
        let dark = rng.next() > 0.35;
        c.set_stroke_style_str(&if dark {
            format!("rgba(45,24,8,{})", 0.06 + rng.next() * 0.16)
        } else {
            format!("rgba(255,222,170,{})", 0.04 + rng.next() * 0.09)
        });
        c.set_line_width(0.6 + rng.next() * 1.4);
        for (ox, oy) in [(0.0, 0.0), (-w, 0.0), (0.0, -h), (0.0, h)] {
            c.begin_path();
            c.move_to(x + ox, y + oy);
            c.bezier_curve_to(x + ox + length * 0.33, y + oy + sway, x + ox + length * 0.66, y + oy - sway, x + ox + length, y + oy);
            c.stroke();
        }
    }
    // Seasoning checks: long, thin, dark splits.
    for _ in 0..5 {
        let y = rng.next() * h;
        let x = rng.next() * w;
        let length = 150.0 + rng.next() * 350.0;
        k.stroke("rgba(25,12,3,0.55)", 1.2 + rng.next() * 1.3);
        for ox in [0.0, -w] {
            k.line(x + ox, y, x + ox + length, y + rng.next() * 3.0 - 1.5);
        }
    }
    // Knots, with the grain swirling around them.
    for _ in 0..7 {
        let x = rng.next() * w;
        let y = rng.next() * h;
        let r = 4.0 + rng.next() * 9.0;
        for (ox, oy) in [(0.0, 0.0), (-w, 0.0), (w, 0.0), (0.0, -h), (0.0, h)] {
            let (cx, cy) = (x + ox, y + oy);
            for ring in (1..5).rev() {
                let s = ring as f64;
                k.stroke(&format!("rgba(50,26,8,{})", 0.12 + 0.05 * (5.0 - s)), 1.0);
                c.begin_path();
                c.ellipse(cx, cy, r * (1.0 + s * 0.9) * 1.8, r * (1.0 + s * 0.5), 0.0, 0.0, TAU).expect("ellipse");
                c.stroke();
            }
            k.fill("rgba(38,18,5,0.85)");
            c.begin_path();
            c.ellipse(cx, cy, r * 1.3, r, 0.0, 0.0, TAU).expect("ellipse");
            c.fill();
            k.fill("rgba(90,50,20,0.6)");
            c.begin_path();
            c.ellipse(cx, cy, r * 0.6, r * 0.45, 0.0, 0.0, TAU).expect("ellipse");
            c.fill();
        }
    }
    k
}

/// Overlapping cedar shingles. Courses run down the canvas's x axis, which
/// the roof maps down-slope; each course's butt edge throws a dark line.
pub fn shingles(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(512, 512);
    let c = &k.c;
    k.fill("#2a211b");
    c.fill_rect(0.0, 0.0, k.w, k.h);
    let courses = 6;
    let course = k.w / courses as f64;
    for i in 0..courses {
        let x = i as f64 * course;
        let mut y = -rng.next() * 40.0 - (i % 2) as f64 * 30.0;
        while y < k.h {
            let width = 34.0 + rng.next() * 46.0;
            let light = 22.0 + rng.next() * 12.0;
            let tone = format!("hsl({},{}%,{}%)", 20.0 + rng.next() * 12.0, 14.0 + rng.next() * 14.0, light);
            for oy in [-k.h, 0.0, k.h] {
                k.fill(&tone);
                c.fill_rect(x, y + oy, course, width - 3.0);
                // Weathered grain along each shingle.
                for _ in 0..8 {
                    let gy = y + oy + rng.next() * (width - 3.0);
                    k.stroke(&format!("rgba(0,0,0,{})", 0.05 + rng.next() * 0.1), 1.0);
                    k.line(x, gy, x + course, gy + rng.next() * 2.0 - 1.0);
                }
            }
            y += width;
        }
        // The exposed butt edge, down-slope, sits in its own shadow.
        let shade = c.create_linear_gradient(x + course - 14.0, 0.0, x + course, 0.0);
        shade.add_color_stop(0.0, "rgba(0,0,0,0)").expect("gradient stop");
        shade.add_color_stop(1.0, "rgba(0,0,0,0.55)").expect("gradient stop");
        c.set_fill_style_canvas_gradient(&shade);
        c.fill_rect(x + course - 14.0, 0.0, 14.0, k.h);
        k.fill("rgba(10,7,5,0.9)");
        c.fill_rect(x + course - 2.0, 0.0, 2.0, k.h);
    }
    // A little lichen and snow-melt staining.
    for _ in 0..3000 {
        k.fill(if rng.next() > 0.6 { "rgba(170,175,160,0.08)" } else { "rgba(0,0,0,0.1)" });
        c.fill_rect(rng.next() * k.w, rng.next() * k.h, 2.0, 2.0);
    }
    k
}

/// A soft teardrop of flame on black, for additive blending: dim orange at
/// the edges, a white-gold core near the base.
pub fn flame() -> Canvas {
    let k = Canvas::new(128, 256);
    let c = &k.c;
    let (cx, base) = (k.w / 2.0, k.h * 0.86);
    for i in 0..9 {
        let t = i as f64 / 8.0;
        let scale = 1.0 - t * 0.72;
        let rw = 54.0 * scale;
        let tip = base - (base - 8.0) * (1.0 - t * 0.45);
        let r = 255.0;
        let g = 70.0 + t * 175.0;
        let b = 10.0 + t * 170.0;
        k.fill(&format!("rgba({r},{g},{b},{})", 0.16 + t * 0.1));
        c.begin_path();
        c.move_to(cx, tip);
        c.bezier_curve_to(cx + rw * 0.25, tip + (base - tip) * 0.35, cx + rw * 1.05, base - rw * 0.9, cx + rw, base - rw * 0.15);
        c.arc(cx, base - rw * 0.15, rw, 0.0, std::f64::consts::PI).expect("arc");
        c.bezier_curve_to(cx - rw * 1.05, base - rw * 0.9, cx - rw * 0.25, tip + (base - tip) * 0.35, cx, tip);
        c.fill();
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

/// One brushstroke: a round-capped line in an HSL colour.
fn dab(k: &Canvas, x: f64, y: f64, length: f64, angle: f64, width: f64, hsl: (f64, f64, f64), alpha: f64) {
    let (h, s, l) = hsl;
    k.c.set_stroke_style_str(&format!("hsla({},{}%,{}%,{alpha})", h.rem_euclid(360.0), s.clamp(0.0, 100.0), l.clamp(0.0, 100.0)));
    k.c.set_line_width(width);
    let (dx, dy) = (angle.cos() * length / 2.0, angle.sin() * length / 2.0);
    k.line(x - dx, y - dy, x + dx, y + dy);
}

/// Interpolates HSL stops by height; hues are given unwrapped.
fn ramp(stops: &[(f64, (f64, f64, f64))], y: f64) -> (f64, f64, f64) {
    let last = stops.len() - 1;
    if y <= stops[0].0 {
        return stops[0].1;
    }
    if y >= stops[last].0 {
        return stops[last].1;
    }
    let i = stops.iter().rposition(|s| s.0 <= y).unwrap_or(0).min(last - 1);
    let (a, b) = (stops[i], stops[i + 1]);
    let t = (y - a.0) / (b.0 - a.0);
    (a.1.0 + (b.1.0 - a.1.0) * t, a.1.1 + (b.1.1 - a.1.1) * t, a.1.2 + (b.1.2 - a.1.2) * t)
}

fn jitter(rng: &mut Rng, (h, s, l): (f64, f64, f64), amount: f64) -> (f64, f64, f64) {
    (h + (rng.next() - 0.5) * amount * 2.0, s + (rng.next() - 0.5) * amount * 2.0, l + (rng.next() - 0.5) * amount * 1.4)
}

/// An oil sketch of a winter lake at dusk: a cabin with a lit window on the
/// near shore, snowy pines, and peaks catching the last of the sun. Painted
/// stroke by stroke, then given canvas weave and a warm varnish.
pub fn painting(rng: &mut Rng) -> Canvas {
    let k = Canvas::new(1024, 720);
    let (w, c) = (k.w, &k.c);
    c.set_line_cap("round");
    let horizon = 432.0;
    let sky = [
        (0.0, (222.0, 42.0, 17.0)),
        (190.0, (250.0, 24.0, 31.0)),
        (300.0, (322.0, 24.0, 44.0)),
        (390.0, (370.0, 52.0, 58.0)),
        (horizon, (392.0, 70.0, 68.0)),
    ];
    // Sky: an underpainting, then layers of loose horizontal strokes.
    for y in 0..horizon as i32 {
        let (h, s, l) = ramp(&sky, y as f64);
        k.fill(&format!("hsl({},{s}%,{l}%)", h.rem_euclid(360.0)));
        c.fill_rect(0.0, y as f64, w, 1.0);
    }
    for _ in 0..3200 {
        let y = rng.next() * horizon;
        let blend = y + (rng.next() - 0.5) * 30.0;
        let color = jitter(rng, ramp(&sky, blend), 4.0);
        dab(&k, rng.next() * w, y, 18.0 + rng.next() * 50.0, (rng.next() - 0.5) * 0.25, 3.0 + rng.next() * 6.0, color, 0.5);
    }
    // The low sun, mostly behind the range, and clouds lit from beneath.
    let (sun_x, sun_y) = (712.0, 404.0);
    let halo = c.create_radial_gradient(sun_x, sun_y, 0.0, sun_x, sun_y, 260.0).expect("radial gradient");
    halo.add_color_stop(0.0, "rgba(255,226,170,0.85)").expect("gradient stop");
    halo.add_color_stop(0.25, "rgba(255,180,120,0.35)").expect("gradient stop");
    halo.add_color_stop(1.0, "rgba(255,150,120,0)").expect("gradient stop");
    c.set_fill_style_canvas_gradient(&halo);
    c.fill_rect(0.0, 0.0, w, horizon);
    for _ in 0..7 {
        let (cx, cy) = (rng.next() * w, 170.0 + rng.next() * 170.0);
        let span = 120.0 + rng.next() * 220.0;
        for _ in 0..70 {
            let x = cx + (rng.next() - 0.5) * span;
            let y = cy + (rng.next() - 0.5) * 22.0;
            let lit = 1.0 - ((x - sun_x).abs() / 700.0).min(1.0);
            dab(&k, x, y + 6.0, 25.0 + rng.next() * 40.0, (rng.next() - 0.5) * 0.12, 3.0 + rng.next() * 4.0, (275.0 + lit * 110.0, 30.0 + lit * 35.0, 38.0 + lit * 30.0), 0.45);
            dab(&k, x, y, 25.0 + rng.next() * 40.0, (rng.next() - 0.5) * 0.12, 3.0 + rng.next() * 5.0, (250.0, 18.0, 30.0 + lit * 8.0), 0.35);
        }
    }

    // Far range: a silhouette, then strokes down the slopes, the sunward
    // faces pink-gold and the rest in cold lavender shadow.
    let ridge = |x: f64| {
        let peak = |at: f64, spread: f64, height: f64| height * (-((x - at) / spread).powi(2)).exp();
        355.0 - peak(380.0, 115.0, 175.0) - peak(610.0, 85.0, 120.0) - peak(150.0, 95.0, 80.0) - peak(880.0, 140.0, 70.0)
            + 9.0 * (x * 0.045).sin()
            + 5.0 * (x * 0.13 + 1.0).sin()
            + 3.0 * (x * 0.31).sin()
    };
    k.fill("hsl(238,18%,40%)");
    c.begin_path();
    c.move_to(0.0, horizon + 2.0);
    let mut x = 0.0;
    while x <= w {
        c.line_to(x, ridge(x));
        x += 4.0;
    }
    c.line_to(w, horizon + 2.0);
    c.fill();
    for _ in 0..5200 {
        let x = rng.next() * w;
        let top = ridge(x);
        let y = top + rng.next().powf(1.6) * (horizon - top);
        let slope = (ridge(x + 3.0) - ridge(x - 3.0)) / 6.0;
        let lit = slope > 0.05;
        let snowline = (y - top) < 45.0 + 25.0 * (x * 0.07).sin();
        let color = match (snowline, lit) {
            (true, true) => (24.0, 55.0, 80.0),
            (true, false) => (240.0, 22.0, 62.0),
            (false, true) => (330.0, 16.0, 46.0),
            (false, false) => (236.0, 20.0, 34.0),
        };
        let angle = slope.atan() + (rng.next() - 0.5) * 0.4;
        dab(&k, x, y, 6.0 + rng.next() * 16.0, angle, 2.0 + rng.next() * 3.0, jitter(rng, color, 4.0), 0.7);
    }
    // Near hills in blue-green dusk, furred with distant forest.
    let hills = |x: f64| 405.0 - 34.0 * (-((x - 860.0) / 160.0).powi(2)).exp() - 22.0 * (-((x - 120.0) / 130.0).powi(2)).exp() + 5.0 * (x * 0.03).sin() + 3.0 * (x * 0.2).sin();
    k.fill("hsl(215,22%,20%)");
    c.begin_path();
    c.move_to(0.0, horizon + 2.0);
    let mut x = 0.0;
    while x <= w {
        c.line_to(x, hills(x));
        x += 4.0;
    }
    c.line_to(w, horizon + 2.0);
    c.fill();
    for _ in 0..1800 {
        let x = rng.next() * w;
        let y = hills(x) + rng.next() * (horizon - hills(x));
        dab(&k, x, y, 4.0 + rng.next() * 8.0, -std::f64::consts::FRAC_PI_2 + (rng.next() - 0.5) * 0.3, 2.0 + rng.next() * 2.0, jitter(rng, (205.0, 24.0, 17.0), 4.0), 0.75);
    }

    // The lake: sky and peaks mirrored in long, dragged strokes.
    let shore = 560.0;
    for y in horizon as i32..shore as i32 {
        let mirrored = horizon - (y as f64 - horizon) * 2.2;
        let (h, s, l) = ramp(&sky, mirrored);
        k.fill(&format!("hsl({},{}%,{}%)", h.rem_euclid(360.0), s * 0.8, l * 0.72));
        c.fill_rect(0.0, y as f64, w, 1.0);
    }
    k.fill("rgba(40,40,70,0.45)");
    c.begin_path();
    c.move_to(0.0, horizon);
    let mut x = 0.0;
    while x <= w {
        c.line_to(x, horizon + (horizon - ridge(x)) * 0.45);
        x += 4.0;
    }
    c.line_to(w, horizon);
    c.fill();
    for _ in 0..2600 {
        let y = horizon + rng.next() * (shore - horizon);
        let mirrored = horizon - (y - horizon) * 2.2;
        let (h, s, l) = ramp(&sky, mirrored);
        dab(&k, rng.next() * w, y, 30.0 + rng.next() * 70.0, (rng.next() - 0.5) * 0.04, 1.5 + rng.next() * 3.0, jitter(rng, (h, s * 0.8, l * 0.7), 4.0), 0.4);
    }
    // The sun's path across the water.
    for _ in 0..260 {
        let t = rng.next();
        let y = horizon + 2.0 + t * (shore - horizon - 10.0);
        let x = sun_x + (rng.next() - 0.5) * (14.0 + t * 140.0);
        dab(&k, x, y, 6.0 + rng.next() * 22.0, 0.0, 1.5 + rng.next() * 2.0, (36.0, 90.0, 74.0 + rng.next() * 16.0), 0.75 - t * 0.4);
    }
    dab(&k, w / 2.0, horizon + 1.0, w, 0.0, 2.5, (230.0, 25.0, 18.0), 0.6);

    // Snow bank in the foreground, warm where the sun reaches.
    let bank = |x: f64| shore - 6.0 + 14.0 * (x * 0.008).sin() + 6.0 * (x * 0.03 + 2.0).sin();
    k.fill("hsl(228,24%,46%)");
    c.begin_path();
    c.move_to(0.0, k.h);
    let mut x = 0.0;
    while x <= w {
        c.line_to(x, bank(x));
        x += 4.0;
    }
    c.line_to(w, k.h);
    c.fill();
    for _ in 0..7000 {
        let x = rng.next() * w;
        let top = bank(x);
        let y = top + rng.next().powf(0.8) * (k.h - top);
        let near_top = ((y - top) / 60.0).min(1.0);
        let sunward = (1.0 - (x - sun_x).abs() / 900.0).max(0.0);
        let color = if rng.next() < 0.55 - near_top * 0.3 + sunward * 0.2 {
            (30.0, 45.0 + sunward * 20.0, 72.0 + sunward * 10.0)
        } else {
            (228.0, 28.0, 40.0 + (1.0 - near_top) * 14.0)
        };
        let drift = 0.25 * ((x * 0.01 + y * 0.02).sin());
        dab(&k, x, y, 8.0 + rng.next() * 22.0, drift + (rng.next() - 0.5) * 0.5, 2.0 + rng.next() * 3.5, jitter(rng, color, 3.0), 0.4);
    }

    // A cabin on the shore with its window lit and smoke rising.
    let (cx, cy) = (318.0, bank(318.0) - 2.0);
    k.fill("hsl(18,30%,16%)");
    c.fill_rect(cx - 30.0, cy - 30.0, 60.0, 30.0);
    for i in 0..6 {
        dab(&k, cx, cy - 4.0 - i as f64 * 5.0, 58.0, 0.0, 2.0, (20.0, 28.0, 22.0 + (i % 2) as f64 * 5.0), 0.8);
    }
    k.fill("hsl(222,22%,78%)");
    c.begin_path();
    c.move_to(cx - 40.0, cy - 28.0);
    c.line_to(cx, cy - 56.0);
    c.line_to(cx + 40.0, cy - 28.0);
    c.close_path();
    c.fill();
    dab(&k, cx + 16.0, cy - 52.0, 14.0, std::f64::consts::FRAC_PI_2, 6.0, (15.0, 20.0, 22.0), 0.9);
    let glow = c.create_radial_gradient(cx - 8.0, cy - 14.0, 0.0, cx - 8.0, cy - 14.0, 40.0).expect("radial gradient");
    glow.add_color_stop(0.0, "rgba(255,200,110,0.55)").expect("gradient stop");
    glow.add_color_stop(1.0, "rgba(255,160,80,0)").expect("gradient stop");
    c.set_fill_style_canvas_gradient(&glow);
    c.fill_rect(cx - 50.0, cy - 55.0, 85.0, 85.0);
    k.fill("hsl(42,95%,70%)");
    c.fill_rect(cx - 13.0, cy - 19.0, 10.0, 9.0);
    for i in 0..18 {
        let t = i as f64 / 18.0;
        let sx = cx + 16.0 + (t * 9.0).sin() * 6.0 + t * 40.0;
        dab(&k, sx, cy - 60.0 - t * 110.0, 10.0, -1.2, 4.0 + t * 10.0, (240.0, 10.0, 60.0), 0.18 * (1.0 - t));
    }
    for i in 0..6 {
        dab(&k, cx - 8.0 + (rng.next() - 0.5) * 6.0, horizon + 70.0 + i as f64 * 7.0, 6.0 + rng.next() * 6.0, 0.0, 1.5, (40.0, 90.0, 66.0), 0.5);
    }

    // Snowy pines framing the view.
    let pine = |k: &Canvas, rng: &mut Rng, x: f64, base: f64, height: f64| {
        let c = &k.c;
        let width = height * 0.3;
        dab(k, x, base - height * 0.06, height * 0.12, std::f64::consts::FRAC_PI_2, height * 0.03, (20.0, 25.0, 12.0), 0.9);
        let tiers = (height / 22.0) as i32 + 3;
        for t in 0..tiers {
            let f = t as f64 / tiers as f64;
            let y = base - height * 0.08 - f * height * 0.9;
            let reach = width * (1.0 - f).powf(0.9) * (0.85 + rng.next() * 0.3);
            let drop = height / tiers as f64 * 1.6;
            // A drooping skirt of boughs, ragged along its hem.
            k.fill(&format!("hsl(198,26%,{}%)", 10.0 + rng.next() * 4.0));
            c.begin_path();
            c.move_to(x, y - drop * 0.35);
            let steps = 9;
            for side in [-1.0, 1.0] {
                for i in 0..=steps {
                    let u = i as f64 / steps as f64;
                    let px = x + side * reach * u;
                    let py = y + drop * (0.2 + 0.8 * u.powf(1.4)) + (rng.next() - 0.5) * drop * 0.3;
                    if side < 0.0 { c.line_to(px, py) } else if i == 0 { c.move_to(x, y - drop * 0.35); c.line_to(px, py) } else { c.line_to(px, py) }
                }
                c.line_to(x, y - drop * 0.35);
            }
            c.fill();
            // Snow resting on the upper side of the boughs, warm toward the sun.
            for _ in 0..(reach / 3.0) as i32 + 2 {
                let side = if rng.next() > 0.5 { 1.0 } else { -1.0 };
                let u = rng.next() * 0.9;
                let px = x + side * reach * u;
                let py = y + drop * (0.2 + 0.8 * u.powf(1.4)) - drop * 0.25;
                let snow = if side > 0.0 { (32.0, 40.0, 76.0) } else { (226.0, 26.0, 60.0) };
                let angle = side * (0.35 + u * 0.4);
                dab(k, px, py, 4.0 + rng.next() * reach * 0.35, angle, 1.5 + rng.next() * 2.5, jitter(rng, snow, 4.0), 0.75);
            }
        }
    };
    for (x, base, height) in [
        (40.0, 640.0, 360.0),
        (110.0, 600.0, 280.0),
        (175.0, 575.0, 190.0),
        (230.0, 566.0, 120.0),
        (860.0, 590.0, 230.0),
        (935.0, 620.0, 330.0),
        (1000.0, 650.0, 400.0),
    ] {
        pine(&k, rng, x, base, height);
    }

    // Canvas weave, then a warm, slightly darkened varnish at the edges.
    for x in (0..k.w as i32).step_by(3) {
        k.stroke("rgba(255,255,255,0.025)", 1.0);
        k.line(x as f64, 0.0, x as f64, k.h);
    }
    for y in (0..k.h as i32).step_by(3) {
        k.stroke("rgba(0,0,0,0.03)", 1.0);
        k.line(0.0, y as f64, k.w, y as f64);
    }
    k.fill("rgba(255,190,110,0.06)");
    c.fill_rect(0.0, 0.0, k.w, k.h);
    let vignette = c.create_radial_gradient(k.w / 2.0, k.h / 2.0, k.h * 0.35, k.w / 2.0, k.h / 2.0, k.w * 0.7).expect("radial gradient");
    vignette.add_color_stop(0.0, "rgba(30,15,0,0)").expect("gradient stop");
    vignette.add_color_stop(1.0, "rgba(30,15,0,0.35)").expect("gradient stop");
    c.set_fill_style_canvas_gradient(&vignette);
    c.fill_rect(0.0, 0.0, k.w, k.h);
    k
}

/// Gilded moulding for the frame, laid along the canvas's x axis: a rounded
/// profile across it (lit along the middle), with either a row of beads or
/// carved leaves, and dark patina where the gilt has worn into recesses.
pub fn gilt(rng: &mut Rng, leaves: bool) -> Canvas {
    let k = Canvas::new(512, 128);
    let c = &k.c;
    let profile = c.create_linear_gradient(0.0, 0.0, 0.0, k.h);
    for (stop, color) in [(0.0, "#4a3412"), (0.25, "#b88a3a"), (0.5, "#e8c374"), (0.75, "#a27628"), (1.0, "#3e2a0e")] {
        profile.add_color_stop(stop, color).expect("gradient stop");
    }
    c.set_fill_style_canvas_gradient(&profile);
    c.fill_rect(0.0, 0.0, k.w, k.h);
    let count = if leaves { 6 } else { 16 };
    let pitch = k.w / count as f64;
    for i in 0..count {
        let x = (i as f64 + 0.5) * pitch;
        if leaves {
            // An acanthus leaf: a central rib with lobes curling either side.
            for side in [-1.0, 1.0] {
                for lobe in 0..4 {
                    let t = lobe as f64 / 4.0;
                    let (lx, ly) = (x + side * (10.0 + t * 22.0), 30.0 + t * 22.0);
                    k.fill("rgba(40,24,6,0.55)");
                    c.begin_path();
                    c.ellipse(lx + side * 2.0, ly + 3.0, 13.0 - t * 4.0, 7.0, side * 0.6, 0.0, TAU).expect("ellipse");
                    c.fill();
                    k.fill("rgba(255,226,150,0.55)");
                    c.begin_path();
                    c.ellipse(lx, ly, 11.0 - t * 4.0, 5.0, side * 0.6, 0.0, TAU).expect("ellipse");
                    c.fill();
                }
            }
            k.stroke("rgba(40,24,6,0.7)", 3.0);
            k.line(x, 22.0, x, 100.0);
            k.stroke("rgba(255,230,160,0.5)", 1.5);
            k.line(x - 1.5, 22.0, x - 1.5, 98.0);
        } else {
            k.fill("rgba(40,24,6,0.6)");
            c.begin_path();
            c.ellipse(x + 2.0, k.h / 2.0 + 4.0, pitch * 0.38, 26.0, 0.0, 0.0, TAU).expect("ellipse");
            c.fill();
            let bead = c.create_radial_gradient(x - 5.0, k.h / 2.0 - 10.0, 2.0, x, k.h / 2.0, 30.0).expect("radial gradient");
            bead.add_color_stop(0.0, "#fff1c4").expect("gradient stop");
            bead.add_color_stop(0.5, "#d0a24c").expect("gradient stop");
            bead.add_color_stop(1.0, "#6b4a16").expect("gradient stop");
            c.set_fill_style_canvas_gradient(&bead);
            c.begin_path();
            c.ellipse(x, k.h / 2.0, pitch * 0.34, 24.0, 0.0, 0.0, TAU).expect("ellipse");
            c.fill();
        }
    }
    // Worn gilt and grime.
    for _ in 0..2500 {
        let dark = rng.next() > 0.35;
        k.fill(if dark { "rgba(35,22,8,0.18)" } else { "rgba(255,240,190,0.12)" });
        let size = 1.0 + rng.next() * 2.5;
        c.fill_rect(rng.next() * k.w, rng.next() * k.h, size, size);
    }
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
