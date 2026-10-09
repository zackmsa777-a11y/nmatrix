use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Rain,
    Waterfall,
    Waves,
    Spiral,
    Glitch,
    Starfield,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    Emerald,
    Cyan,
    Violet,
    Amber,
    Rainbow,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphSet {
    Matrix,
    Binary,
    Hex,
}

macro_rules! choices {
    ($kind:ident, $($variant:ident => $name:literal),+ $(,)?) => {
        impl $kind {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub fn name(self) -> &'static str {
                match self { $(Self::$variant => $name),+ }
            }
            pub fn parse(value: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|item| item.name() == value)
            }
            pub fn next(self, direction: isize) -> Self {
                let i = Self::ALL.iter().position(|&item| item == self).unwrap() as isize;
                Self::ALL[(i + direction).rem_euclid(Self::ALL.len() as isize) as usize]
            }
        }
    };
}
choices!(Mode, Rain => "rain", Waterfall => "waterfall", Waves => "waves",
         Spiral => "spiral", Glitch => "glitch", Starfield => "starfield");
choices!(Palette, Emerald => "emerald", Cyan => "cyan", Violet => "violet",
         Amber => "amber", Rainbow => "rainbow");
choices!(GlyphSet, Matrix => "matrix", Binary => "binary", Hex => "hex");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub glyph: u8,
    pub level: f32,
    pub hue: f32,
}
impl Default for Cell {
    fn default() -> Self {
        Self {
            glyph: b' ',
            level: 0.0,
            hue: 0.0,
        }
    }
}

pub fn color(palette: Palette, level: f32, hue: f32) -> [u8; 3] {
    let base = match palette {
        Palette::Emerald => [32.0, 246.0, 151.0],
        Palette::Cyan => [37.0, 207.0, 255.0],
        Palette::Violet => [188.0, 106.0, 255.0],
        Palette::Amber => [255.0, 177.0, 58.0],
        Palette::Rainbow => {
            let h = hue.rem_euclid(1.0) * 6.0;
            let p = 0.22;
            let q = 1.0 - h.fract() * 0.78;
            let t = p + h.fract() * 0.78;
            let rgb = match h as u8 {
                0 => [1.0, t, p],
                1 => [q, 1.0, p],
                2 => [p, 1.0, t],
                3 => [p, q, 1.0],
                4 => [t, p, 1.0],
                _ => [1.0, p, q],
            };
            rgb.map(|v| v * 255.0)
        }
    };
    let level = level.clamp(0.0, 1.0);
    let brightness = level.powf(0.70);
    let white = ((level - 0.83) / 0.17).clamp(0.0, 1.0) * 0.90;
    base.map(|v| ((v * (1.0 - white) + 255.0 * white) * brightness).round() as u8)
}

#[derive(Clone)]
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0xA83C_E41B_13D7_9E15
        } else {
            seed
        })
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + self.unit() * (high - low)
    }
}

struct Stream {
    x: usize,
    head: f32,
    speed: f32,
    length: f32,
    depth: f32,
}
struct Star {
    x: f32,
    y: f32,
    z: f32,
    speed: f32,
    trail: Vec<(i32, i32)>,
}

pub struct Scene {
    pub width: usize,
    pub height: usize,
    pub mode: Mode,
    pub time: f64,
    pub density: f32,
    pub glyphs: GlyphSet,
    rng: Rng,
    salt: u64,
    streams: Vec<Stream>,
    stars: Vec<Star>,
    cells: Vec<Cell>,
    next_glitch: f64,
    glitch_end: f64,
    glitch_band: (usize, usize, isize),
}

impl Scene {
    pub fn new(
        width: usize,
        height: usize,
        mode: Mode,
        density: f32,
        glyphs: GlyphSet,
        seed: u64,
    ) -> Self {
        let mut scene = Self {
            width: 1,
            height: 1,
            mode,
            time: 0.0,
            density: density.clamp(0.2, 1.4),
            glyphs,
            rng: Rng::new(seed),
            salt: seed,
            streams: Vec::new(),
            stars: Vec::new(),
            cells: Vec::new(),
            next_glitch: 0.0,
            glitch_end: -1.0,
            glitch_band: (0, 0, 0),
        };
        scene.resize(width, height);
        scene
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        // Bound allocation for malformed/absurd terminal dimensions.
        self.width = width.clamp(1, 1024);
        self.height = height.clamp(1, 512);
        self.cells = vec![Cell::default(); self.width * self.height];
        self.streams.clear();
        self.stars.clear();
        let h = self.height as f32;
        for x in 0..self.width {
            if self.rng.unit() < (self.density * 0.92).min(0.97) {
                self.streams.push(Stream {
                    x,
                    head: self.rng.range(-h * 0.8, h * 1.6),
                    speed: self.rng.range(5.0, 19.0),
                    length: self.rng.range(6.0f32.max(h * 0.28), 10.0f32.max(h * 0.90)),
                    depth: self.rng.range(0.66, 1.0),
                });
            }
            if self.density > 0.6 && self.rng.unit() < (self.density - 0.6) * 0.6 {
                self.streams.push(Stream {
                    x,
                    head: self.rng.range(-h, h),
                    speed: self.rng.range(3.0, 7.0),
                    length: self.rng.range(5.0, h.max(8.0)),
                    depth: 0.34,
                });
            }
        }
        let count = ((self.width * self.height) as f32 * 0.10 * self.density) as usize;
        for _ in 0..count.max(12) {
            let star = self.new_star(true);
            self.stars.push(star);
        }
        self.next_glitch = self.time + self.rng.range(0.5, 1.5) as f64;
        self.glitch_end = -1.0;
    }

    fn new_star(&mut self, initial: bool) -> Star {
        Star {
            x: self.rng.range(-(self.width as f32), self.width as f32),
            y: self.rng.range(-(self.height as f32), self.height as f32),
            z: if initial {
                self.rng.range(0.10, 1.0)
            } else {
                1.0
            },
            speed: self.rng.range(0.09, 0.26),
            trail: Vec::with_capacity(6),
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.time += dt as f64;
        let t = self.time as f32;
        for stream in &mut self.streams {
            let flow = if self.mode == Mode::Waterfall {
                1.15 + 0.20 * (t * 0.6 + stream.x as f32 * 0.08).sin()
            } else {
                1.0
            };
            stream.head += dt * stream.speed * flow;
            if stream.head - stream.length > self.height as f32 {
                let overshoot = stream.head - stream.length - self.height as f32;
                stream.head = -1.0 + overshoot % (self.height as f32 + stream.length);
                stream.speed = self.rng.range(5.0, 19.0);
            }
        }
        for i in 0..self.stars.len() {
            self.stars[i].z -= dt * self.stars[i].speed;
            let z = self.stars[i].z;
            if z <= 0.04 {
                self.stars[i] = self.new_star(false);
                continue;
            }
            let x = (self.width as f32 * 0.5 + self.stars[i].x * 0.23 / z) as i32;
            let y = (self.height as f32 * 0.5 + self.stars[i].y * 0.23 / z) as i32;
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                self.stars[i] = self.new_star(false);
                continue;
            }
            let trail = &mut self.stars[i].trail;
            if trail.last().copied() != Some((x, y)) {
                if trail.len() == 6 {
                    trail.remove(0);
                }
                trail.push((x, y));
            }
        }
        if self.mode == Mode::Glitch && self.time >= self.next_glitch {
            let y = (self.rng.next() as usize) % self.height;
            let height = 1 + (self.rng.next() as usize) % 4;
            let shift = (self.rng.next() % 19) as isize - 9;
            self.glitch_band = (y, (y + height).min(self.height), shift);
            self.glitch_end = self.time + self.rng.range(0.08, 0.16) as f64;
            self.next_glitch = self.time + self.rng.range(0.7, 2.4) as f64;
        }
    }

    pub fn frame(&mut self) -> &[Cell] {
        self.cells.fill(Cell::default());
        match self.mode {
            Mode::Rain | Mode::Waterfall | Mode::Glitch => self.rain(),
            Mode::Waves => self.waves(),
            Mode::Spiral => self.spiral(),
            Mode::Starfield => self.starfield(),
        }
        if self.mode == Mode::Glitch && self.time < self.glitch_end {
            let (start, end, shift) = self.glitch_band;
            for y in start..end {
                let row = &mut self.cells[y * self.width..(y + 1) * self.width];
                row.rotate_right(shift.rem_euclid(self.width as isize) as usize);
                for (x, cell) in row.iter_mut().enumerate() {
                    cell.hue = (cell.hue + 0.15).fract();
                    if hash(x as u64 + y as u64 * 101 + self.salt) % 23 == 0 {
                        cell.glyph = glyph(self.glyphs, self.salt, x, y, self.time);
                        cell.level = 0.80;
                        cell.hue = 0.6;
                    }
                }
            }
        }
        &self.cells
    }

    fn rain(&mut self) {
        let t = self.time as f32;
        for stream in &self.streams {
            let x = stream.x;
            let end = stream.head.floor() as i32;
            let start = (stream.head - stream.length).ceil().max(0.0) as i32;
            for y in start..=(end.min(self.height as i32 - 1)) {
                if y < 0 {
                    continue;
                }
                let d = stream.head - y as f32;
                let level = if y == end {
                    stream.depth
                } else {
                    (1.0 - d / stream.length).clamp(0.0, 1.0).powf(1.55) * 0.78 * stream.depth
                };
                let hue = x as f32 / self.width as f32 * 0.55 + t * 0.025;
                let c = glyph(self.glyphs, self.salt, x, y as usize, self.time);
                put(
                    &mut self.cells,
                    self.width,
                    self.height,
                    x as i32,
                    y,
                    Cell {
                        glyph: c,
                        level,
                        hue,
                    },
                );
            }
        }
        if self.mode == Mode::Waterfall {
            for y in 0..self.height {
                for x in 0..self.width {
                    let field = (x as f32 * 0.16 + y as f32 * 0.045 - t * 1.05).sin();
                    let layer = ((field - 0.30) / 0.70).max(0.0) * 0.35;
                    if hash(x as u64 + y as u64 * 79) % 5 < 3 && layer > 0.03 {
                        let c = glyph(self.glyphs, self.salt ^ 131, x, y, self.time * 0.4);
                        put(
                            &mut self.cells,
                            self.width,
                            self.height,
                            x as i32,
                            y as i32,
                            Cell {
                                glyph: c,
                                level: layer,
                                hue: x as f32 / self.width as f32 + t * 0.02,
                            },
                        );
                    }
                }
            }
        }
    }

    fn waves(&mut self) {
        let t = self.time as f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = (x as f32 - self.width as f32 * 0.5) * 0.52;
                let dy = y as f32 - self.height as f32 * 0.5;
                let radius = dx.hypot(dy);
                let field = ((radius * 0.36 - t * 1.15).sin()
                    + 0.48 * (x as f32 * 0.09 + y as f32 * 0.20 + t * 0.8).sin())
                    / 1.48;
                let threshold = 0.54 - self.density * 0.18;
                let level = ((field.abs() - threshold) / (1.0 - threshold))
                    .max(0.0)
                    .powf(1.2);
                let c = if self.glyphs != GlyphSet::Matrix || level > 0.70 {
                    glyph(self.glyphs, self.salt, x, y, self.time * 0.35)
                } else {
                    b".:+=*"[(level * 4.99) as usize]
                };
                put(
                    &mut self.cells,
                    self.width,
                    self.height,
                    x as i32,
                    y as i32,
                    Cell {
                        glyph: c,
                        level,
                        hue: radius * 0.012 + t * 0.02,
                    },
                );
            }
        }
    }

    fn spiral(&mut self) {
        let t = self.time as f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = (x as f32 - (self.width - 1) as f32 * 0.5) * 0.52;
                let dy = y as f32 - (self.height - 1) as f32 * 0.5;
                let radius = dx.hypot(dy);
                let angle = dy.atan2(dx);
                let threshold = 0.79 - self.density * 0.22;
                let ridge = ((3.0 * angle - radius * 0.27 + t * 0.88).cos() - threshold)
                    / (1.0 - threshold);
                let level = ridge.max(0.0).powf(0.85) * (radius * 0.4 + 0.2).min(1.0);
                let c = if self.glyphs != GlyphSet::Matrix || level > 0.70 {
                    glyph(self.glyphs, self.salt, x, y, self.time * 0.4)
                } else {
                    b".:+*"[(level * 3.99) as usize]
                };
                put(
                    &mut self.cells,
                    self.width,
                    self.height,
                    x as i32,
                    y as i32,
                    Cell {
                        glyph: c,
                        level,
                        hue: angle / TAU + radius * 0.01 + t * 0.02,
                    },
                );
            }
        }
    }

    fn starfield(&mut self) {
        for (i, star) in self.stars.iter().enumerate() {
            let x = (self.width as f32 * 0.5 + star.x * 0.23 / star.z) as i32;
            let y = (self.height as f32 * 0.5 + star.y * 0.23 / star.z) as i32;
            let level = 0.14 + (1.0 - star.z) * 0.86;
            let hue = i as f32 / self.stars.len() as f32 + self.time as f32 * 0.025;
            for (age, &(tx, ty)) in star.trail.iter().rev().skip(1).enumerate() {
                let tail = Cell {
                    glyph: if self.glyphs == GlyphSet::Matrix {
                        b'.'
                    } else {
                        glyph(self.glyphs, self.salt, i, age + 1, self.time * 0.4)
                    },
                    level: level * 0.45f32.powi(age as i32 + 1),
                    hue,
                };
                put(&mut self.cells, self.width, self.height, tx, ty, tail);
            }
            let c = if self.glyphs == GlyphSet::Matrix {
                if level < 0.45 {
                    b'.'
                } else if level < 0.76 {
                    b'+'
                } else {
                    b'*'
                }
            } else {
                glyph(self.glyphs, self.salt, i, 0, self.time * 0.4)
            };
            put(
                &mut self.cells,
                self.width,
                self.height,
                x,
                y,
                Cell {
                    glyph: c,
                    level,
                    hue,
                },
            );
        }
    }
}

fn put(cells: &mut [Cell], width: usize, height: usize, x: i32, y: i32, mut cell: Cell) {
    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
        return;
    }
    cell.level = cell.level.clamp(0.0, 1.0);
    cell.hue = cell.hue.rem_euclid(1.0);
    let target = &mut cells[y as usize * width + x as usize];
    if cell.level > 0.025 && cell.level >= target.level {
        *target = cell;
    }
}

fn glyph(set: GlyphSet, salt: u64, x: usize, y: usize, time: f64) -> u8 {
    let alphabet: &[u8] = match set {
        GlyphSet::Matrix => b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ<>/{}[]+=*",
        GlyphSet::Binary => b"01",
        GlyphSet::Hex => b"0123456789ABCDEF",
    };
    let value = hash(
        salt ^ (x as u64).wrapping_mul(7919)
            ^ (y as u64).wrapping_mul(104729)
            ^ (time * 7.0) as u64,
    );
    alphabet[value as usize % alphabet.len()]
}

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}
