use crate::engine::{Cell, Palette, color};
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorDepth {
    TrueColor,
    Colors256,
    Basic,
}
impl ColorDepth {
    pub fn detect() -> Self {
        let colorterm = std::env::var("COLORTERM")
            .unwrap_or_default()
            .to_ascii_lowercase();
        let term = std::env::var("TERM").unwrap_or_default();
        if colorterm == "truecolor" || colorterm == "24bit" || term.contains("kitty") {
            Self::TrueColor
        } else if term.contains("256color") {
            Self::Colors256
        } else {
            Self::Basic
        }
    }
    pub fn background(self) -> &'static str {
        if self == Self::TrueColor {
            "\x1b[48;2;6;11;16m"
        } else {
            "\x1b[40m"
        }
    }
    pub fn foreground(self, rgb: [u8; 3]) -> String {
        match self {
            Self::TrueColor => format!("\x1b[38;2;{};{};{}m", rgb[0], rgb[1], rgb[2]),
            Self::Colors256 => format!("\x1b[38;5;{}m", xterm_color(rgb)),
            Self::Basic => {
                let [r, g, b] = rgb.map(|v| v as u32);
                let brightest = r.max(g).max(b);
                let mut index = 0;
                if r * 2 > brightest {
                    index |= 1;
                }
                if g * 2 > brightest {
                    index |= 2;
                }
                if b * 2 > brightest {
                    index |= 4;
                }
                let style = if brightest > 200 {
                    1
                } else if brightest < 85 {
                    2
                } else {
                    22
                };
                format!("\x1b[{style};{}m", 30 + index)
            }
        }
    }
}

fn xterm_color(rgb: [u8; 3]) -> u8 {
    let levels = [0i32, 95, 135, 175, 215, 255];
    let cube = rgb.map(|v| {
        (0..6)
            .min_by_key(|&i| (v as i32 - levels[i]).abs())
            .unwrap()
    });
    let cube_rgb = cube.map(|i| levels[i]);
    let average = rgb.iter().map(|&c| c as i32).sum::<i32>() / 3;
    let gray = ((average - 8 + 5) / 10).clamp(0, 23);
    let gray_rgb = [8 + gray * 10; 3];
    let error = |candidate: [i32; 3]| {
        rgb.iter()
            .zip(candidate)
            .map(|(&v, c)| (v as i32 - c).pow(2))
            .sum::<i32>()
    };
    if error(gray_rgb) < error(cube_rgb) {
        (232 + gray) as u8
    } else {
        (16 + cube[0] * 36 + cube[1] * 6 + cube[2]) as u8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Painted {
    glyph: u8,
    rgb: [u8; 3],
}

pub struct Renderer {
    depth: ColorDepth,
    previous: Vec<Painted>,
    width: usize,
    height: usize,
}
impl Renderer {
    pub fn new(depth: ColorDepth) -> Self {
        Self {
            depth,
            previous: Vec::new(),
            width: 0,
            height: 0,
        }
    }
    pub fn invalidate(&mut self) {
        self.previous.clear();
    }
    pub fn render(
        &mut self,
        frame: &[Cell],
        width: usize,
        height: usize,
        palette: Palette,
        palette_from: Option<(Palette, f32)>,
    ) -> String {
        assert_eq!(frame.len(), width * height);
        let full =
            self.previous.len() != frame.len() || self.width != width || self.height != height;
        if full {
            self.previous = vec![
                Painted {
                    glyph: b' ',
                    rgb: [0; 3]
                };
                frame.len()
            ];
            self.width = width;
            self.height = height;
        }
        let mut output = String::with_capacity(frame.len() * 6);
        if full {
            output.push_str(self.depth.background());
            output.push_str("\x1b[2J");
        }
        let mut position = usize::MAX;
        let mut active_color = None;
        for (i, cell) in frame.iter().enumerate() {
            let glyph = if cell.level < 0.025 { b' ' } else { cell.glyph };
            let level = (cell.level * 31.0).round() / 31.0;
            let hue = (cell.hue * 47.0).round() / 47.0;
            let mut rgb = if glyph == b' ' {
                [0; 3]
            } else {
                color(palette, level, hue)
            };
            if let Some((from, t)) = palette_from {
                let old = color(from, level, hue);
                for j in 0..3 {
                    rgb[j] = (old[j] as f32 * (1.0 - t) + rgb[j] as f32 * t).round() as u8;
                }
            }
            let painted = Painted { glyph, rgb };
            if !full && self.previous[i] == painted {
                continue;
            }
            if position != i || i % width == 0 {
                write!(output, "\x1b[{};{}H", i / width + 1, i % width + 1).unwrap();
            }
            if glyph != b' ' && active_color != Some(rgb) {
                output.push_str(&self.depth.foreground(rgb));
                active_color = Some(rgb);
            }
            output.push(glyph as char);
            position = i + 1;
            self.previous[i] = painted;
        }
        if !output.is_empty() && !full {
            output.insert_str(0, self.depth.background());
        }
        output
    }
}

pub fn blend_frames(old: &[Cell], new: &[Cell], t: f32) -> Vec<Cell> {
    if old.len() != new.len() || t >= 1.0 {
        return new.to_vec();
    }
    if t <= 0.0 {
        return old.to_vec();
    }
    old.iter()
        .zip(new)
        .enumerate()
        .map(|(i, (a, b))| {
            let threshold = ((i.wrapping_mul(7919) % 65536) as f32) / 65536.0;
            let hue_delta = (b.hue - a.hue + 0.5).rem_euclid(1.0) - 0.5;
            Cell {
                glyph: if t >= threshold { b.glyph } else { a.glyph },
                level: a.level * (1.0 - t) + b.level * t,
                hue: (a.hue + hue_delta * t).rem_euclid(1.0),
            }
        })
        .collect()
}
