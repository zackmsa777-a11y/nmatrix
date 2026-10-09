use crate::engine::{Palette, color};
use crate::profile::{clip_text, display_width};
use crate::render::ColorDepth;
use std::fmt::Write;

pub fn name_mask(name: &str, width: usize, height: usize) -> Option<Vec<Vec<bool>>> {
    let letters: Option<Vec<[u8; 7]>> =
        name.bytes().map(|c| font(c.to_ascii_uppercase())).collect();
    let letters = letters?;
    if letters.is_empty() {
        return None;
    }
    let base_width = letters.len() * 6 - 1;
    if base_width > width || height < 7 {
        return None;
    }
    let vertical = if base_width * 4 <= width && height >= 14 {
        2
    } else {
        1
    };
    let horizontal = if base_width * 2 * vertical <= width {
        vertical * 2
    } else {
        vertical
    };
    let mut mask = vec![vec![false; base_width * horizontal]; 7 * vertical];
    for (letter, rows) in letters.iter().enumerate() {
        for (y, bits) in rows.iter().enumerate() {
            for x in 0..5 {
                if bits & (1 << (4 - x)) == 0 {
                    continue;
                }
                for yy in 0..vertical {
                    for xx in 0..horizontal {
                        mask[y * vertical + yy][(letter * 6 + x) * horizontal + xx] = true;
                    }
                }
            }
        }
    }
    Some(mask)
}

pub fn draw(
    output: &mut String,
    width: usize,
    height: usize,
    depth: ColorDepth,
    palette: Palette,
    name: &str,
    time: f64,
) {
    if width < 6 || height < 3 {
        return;
    }
    let mask = name_mask(name, width - 4, height - 3);
    if let Some(mask) = mask {
        let w = mask[0].len();
        let x = (width - w - 4) / 2 + 1;
        let y = (height - mask.len() - 2) / 2 + 1;
        for (row, pixels) in mask.iter().enumerate() {
            write!(output, "\x1b[{};{}H{}  ", y + row, x, depth.background()).unwrap();
            for (column, &on) in pixels.iter().enumerate() {
                if on {
                    let level = 0.78 + 0.20 * (time * 1.7 + column as f64 * 0.13).sin() as f32;
                    output.push_str(&depth.foreground(color(
                        palette,
                        level,
                        column as f32 / w as f32 + time as f32 * 0.03,
                    )));
                    output.push('█');
                } else {
                    output.push(' ');
                }
            }
            output.push_str("  ");
        }
        let label = clip_text(&format!("{name} / n hide"), width - 4);
        write!(
            output,
            "\x1b[{};{}H{}{} {label} ",
            y + mask.len() + 1,
            (width - display_width(&label) - 2) / 2 + 1,
            depth.background(),
            depth.foreground([145, 189, 166])
        )
        .unwrap();
    } else {
        let label = clip_text(name, width - 4);
        write!(
            output,
            "\x1b[{};{}H{}{}  {label}  ",
            height / 2 + 1,
            (width - display_width(&label) - 4) / 2 + 1,
            depth.background(),
            depth.foreground(color(palette, 0.9, time as f32 * 0.05))
        )
        .unwrap();
    }
}

fn font(c: u8) -> Option<[u8; 7]> {
    Some(match c {
        b'A' => [14, 17, 17, 31, 17, 17, 17],
        b'B' => [30, 17, 17, 30, 17, 17, 30],
        b'C' => [14, 17, 16, 16, 16, 17, 14],
        b'D' => [30, 17, 17, 17, 17, 17, 30],
        b'E' => [31, 16, 16, 30, 16, 16, 31],
        b'F' => [31, 16, 16, 30, 16, 16, 16],
        b'G' => [14, 17, 16, 23, 17, 17, 15],
        b'H' => [17, 17, 17, 31, 17, 17, 17],
        b'I' => [31, 4, 4, 4, 4, 4, 31],
        b'J' => [7, 2, 2, 2, 18, 18, 12],
        b'K' => [17, 18, 20, 24, 20, 18, 17],
        b'L' => [16, 16, 16, 16, 16, 16, 31],
        b'M' => [17, 27, 21, 21, 17, 17, 17],
        b'N' => [17, 25, 21, 19, 17, 17, 17],
        b'O' => [14, 17, 17, 17, 17, 17, 14],
        b'P' => [30, 17, 17, 30, 16, 16, 16],
        b'Q' => [14, 17, 17, 17, 21, 18, 13],
        b'R' => [30, 17, 17, 30, 20, 18, 17],
        b'S' => [15, 16, 16, 14, 1, 1, 30],
        b'T' => [31, 4, 4, 4, 4, 4, 4],
        b'U' => [17, 17, 17, 17, 17, 17, 14],
        b'V' => [17, 17, 17, 17, 17, 10, 4],
        b'W' => [17, 17, 17, 21, 21, 21, 10],
        b'X' => [17, 17, 10, 4, 10, 17, 17],
        b'Y' => [17, 17, 10, 4, 4, 4, 4],
        b'Z' => [31, 1, 2, 4, 8, 16, 31],
        b'0' => [14, 17, 19, 21, 25, 17, 14],
        b'1' => [4, 12, 4, 4, 4, 4, 14],
        b'2' => [14, 17, 1, 2, 4, 8, 31],
        b'3' => [30, 1, 1, 14, 1, 1, 30],
        b'4' => [2, 6, 10, 18, 31, 2, 2],
        b'5' => [31, 16, 16, 30, 1, 1, 30],
        b'6' => [14, 16, 16, 30, 17, 17, 14],
        b'7' => [31, 1, 2, 4, 8, 8, 8],
        b'8' => [14, 17, 17, 14, 17, 17, 14],
        b'9' => [14, 17, 17, 15, 1, 1, 14],
        b' ' => [0; 7],
        b'-' => [0, 0, 0, 31, 0, 0, 0],
        b'_' => [0, 0, 0, 0, 0, 0, 31],
        b'.' => [0, 0, 0, 0, 0, 12, 12],
        b'\'' => [4, 4, 0, 0, 0, 0, 0],
        _ => return None,
    })
}
