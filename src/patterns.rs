//! Procedural space/ambient scenes. Drawing never advances simulation state.
use crate::engine::{Cell, GlyphSet, Mode, glyph, hash, put};
use std::f32::consts::TAU;

pub(crate) fn render(
    cells: &mut [Cell],
    width: usize,
    height: usize,
    mode: Mode,
    density: f32,
    glyphs: GlyphSet,
    seed: u64,
    time: f64,
) {
    let p = Pattern {
        width,
        height,
        density,
        glyphs,
        seed,
        time,
    };
    match mode {
        Mode::Blackhole => p.field(cells, |x, y| {
            let (r, a) = p.polar(x, y);
            let core = (height as f32 * 0.10).max(1.2);
            if r < core {
                return (0.0, 0.0);
            }
            let disc = (-((r - core * 2.1) / (0.75 + density * 0.4)).powi(2)).exp();
            let winding = (3.0 * a - r * 0.55 + time as f32 * 1.3).cos();
            let arm = ((winding - (0.82 - density * 0.20)) / 0.38).clamp(0.0, 1.0);
            let falloff = (-r / (height as f32 * 0.85).max(1.0)).exp();
            (
                (disc * (0.82 + 0.18 * a.cos()) + arm * falloff * 0.75).min(1.0),
                a / TAU + r * 0.03,
            )
        }),
        Mode::Aurora => p.field(cells, |x, y| {
            let nx = x as f32 / width as f32;
            let t = time as f32;
            let center = height as f32
                * (0.38
                    + 0.14 * (nx * TAU * 1.6 + t * 0.35).sin()
                    + 0.06 * (nx * TAU * 4.0 - t * 0.5).sin());
            let distance = y as f32 - center;
            let spread = if distance < 0.0 {
                1.0 + density * 1.2
            } else {
                2.0 + density * 3.0
            };
            let glow = (-(distance / spread).powi(2)).exp();
            let curtain = 0.50 + 0.50 * (nx * TAU * 3.5 + t * 0.8).sin().abs();
            (glow * curtain, nx * 0.5 + t * 0.025)
        }),
        Mode::Plasma => p.field(cells, |x, y| {
            let dx = x as f32 * 0.14;
            let dy = y as f32 * 0.29;
            let t = time as f32;
            let v = ((dx + t * 0.9).sin()
                + (dy - t * 0.65).sin()
                + ((dx * 0.6 + dy * 0.8).hypot(dy * 0.7) - t).sin())
                / 3.0;
            let level = ((v + 0.30 + density * 0.25) / 1.45).clamp(0.0, 1.0);
            (level, v * 0.4 + t * 0.05 + dx * 0.01)
        }),
        Mode::Tunnel => p.field(cells, |x, y| {
            let (r, a) = p.polar(x, y);
            let t = time as f32;
            let ring = ((r + 1.0).ln() * 5.0 - t * 2.0).sin().abs();
            let spoke = (a * 8.0 + t * 0.2).sin().abs();
            let edge = 0.84 - density * 0.13;
            let rings = ((ring - edge) / (1.0 - edge)).max(0.0);
            let spokes = ((spoke - 0.97) / 0.03).max(0.0) * 0.38;
            (
                (rings.max(spokes) * (r * 0.5).min(1.0)).min(1.0),
                r * 0.035 - t * 0.03,
            )
        }),
        Mode::Galaxy => p.galaxy(cells),
        Mode::Fireworks => p.fireworks(cells),
        _ => unreachable!("existing scenes are handled by Scene"),
    }
}

struct Pattern {
    width: usize,
    height: usize,
    density: f32,
    glyphs: GlyphSet,
    seed: u64,
    time: f64,
}
impl Pattern {
    fn polar(&self, x: usize, y: usize) -> (f32, f32) {
        let dx = (x as f32 - (self.width - 1) as f32 * 0.5) * 0.52;
        let dy = y as f32 - (self.height - 1) as f32 * 0.5;
        (dx.hypot(dy), dy.atan2(dx))
    }
    fn field(&self, cells: &mut [Cell], mut field: impl FnMut(usize, usize) -> (f32, f32)) {
        for y in 0..self.height {
            for x in 0..self.width {
                let (level, hue) = field(x, y);
                let c = if self.glyphs == GlyphSet::Matrix && level < 0.70 {
                    b".:+=*"[(level.clamp(0.0, 1.0) * 4.99) as usize]
                } else {
                    glyph(self.glyphs, self.seed, x, y, self.time * 0.35)
                };
                put(
                    cells,
                    self.width,
                    self.height,
                    x as i32,
                    y as i32,
                    Cell {
                        glyph: c,
                        level,
                        hue,
                    },
                );
            }
        }
    }
    fn point(&self, cells: &mut [Cell], x: f32, y: f32, level: f32, hue: f32, id: usize) {
        let c = if self.glyphs == GlyphSet::Matrix {
            if level < 0.35 {
                b'.'
            } else if level < 0.7 {
                b'+'
            } else {
                b'*'
            }
        } else {
            glyph(self.glyphs, self.seed, id, 0, self.time * 0.4)
        };
        put(
            cells,
            self.width,
            self.height,
            x.round() as i32,
            y.round() as i32,
            Cell {
                glyph: c,
                level,
                hue,
            },
        );
    }
    fn galaxy(&self, cells: &mut [Cell]) {
        let count = ((self.width * self.height) as f32 * 0.22 * self.density).max(12.0) as usize;
        let radius = (self.height as f32 * 0.45)
            .min(self.width as f32 * 0.24)
            .max(1.0);
        for i in 0..count {
            let r = sample(self.seed, i as u64 * 3).sqrt() * radius;
            let scatter = (sample(self.seed, i as u64 * 3 + 1) - 0.5) * 0.95;
            let a = (i % 3) as f32 * TAU / 3.0 + r * 0.20 + self.time as f32 * 0.16 + scatter;
            let x = (self.width - 1) as f32 * 0.5 + a.cos() * r / 0.52;
            let y = (self.height - 1) as f32 * 0.5 + a.sin() * r;
            let level =
                (0.25 + sample(self.seed, i as u64 * 3 + 2) * 0.65 + (1.0 - r / radius) * 0.15)
                    .min(1.0);
            self.point(
                cells,
                x,
                y,
                level,
                r / radius * 0.6 + self.time as f32 * 0.02,
                i,
            );
        }
        self.point(
            cells,
            (self.width - 1) as f32 * 0.5,
            (self.height - 1) as f32 * 0.5,
            1.0,
            0.1,
            0,
        );
    }
    fn fireworks(&self, cells: &mut [Cell]) {
        let slots = (5.0 * self.density).ceil() as usize;
        for slot in 0..slots {
            let clock = self.time + slot as f64 * 0.7;
            let phase = clock.rem_euclid(3.2) as f32;
            let seed = self.seed ^ hash((clock / 3.2) as u64) ^ hash(slot as u64);
            let cx = self.width as f32 * (0.20 + sample(seed, 0) * 0.6);
            let cy = self.height as f32 * (0.18 + sample(seed, 1) * 0.38);
            if phase < 0.45 {
                for tail in 0..5 {
                    let age = (phase - tail as f32 * 0.025).max(0.0) / 0.45;
                    let y = self.height as f32 + (cy - self.height as f32) * age;
                    self.point(
                        cells,
                        cx,
                        y,
                        1.0 - tail as f32 * 0.13,
                        slot as f32 / slots as f32,
                        tail,
                    );
                }
                continue;
            }
            let age = phase - 0.45;
            if age > 2.1 {
                continue;
            }
            let particles = (24.0 + 16.0 * self.density) as usize;
            for particle in 0..particles {
                let angle = particle as f32 / particles as f32 * TAU + sample(seed, 2) * TAU;
                let speed = (self.height as f32 * 0.30).max(2.0)
                    * (0.5 + sample(seed, particle as u64 + 5) * 0.5);
                for tail in 0..3 {
                    let t = (age - tail as f32 * 0.07).max(0.0);
                    let x = cx + angle.cos() * speed * t / 0.52;
                    let y = cy + angle.sin() * speed * t + self.height as f32 * 0.055 * t * t;
                    let level = (1.0 - age / 2.1) * (1.0 - tail as f32 * 0.24);
                    self.point(
                        cells,
                        x,
                        y,
                        level,
                        slot as f32 / slots as f32 + particle as f32 * 0.002,
                        particle,
                    );
                }
            }
        }
    }
}
fn sample(seed: u64, index: u64) -> f32 {
    (hash(seed ^ index.wrapping_mul(7919)) >> 40) as f32 / (1u32 << 24) as f32
}
