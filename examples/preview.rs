//! Render actual scene frames as a portable SVG; no runtime dependency.
use nmatrix::engine::{GlyphSet, Mode, Palette, Scene, color};
use std::fmt::Write;

fn escaped(c: u8) -> String {
    match c {
        b'<' => "&lt;".into(),
        b'>' => "&gt;".into(),
        b'&' => "&amp;".into(),
        _ => (c as char).to_string(),
    }
}
fn main() {
    let mut svg = String::from(
        "<svg xmlns='http://www.w3.org/2000/svg' width='1500' height='1240' viewBox='0 0 1500 1240'><rect width='1500' height='1240' fill='#060b10'/><g font-family='Noto Sans Mono, monospace'>",
    );
    svg.push_str("<text x='32' y='55' fill='#dbf4ea' font-size='34'>NMATRIX</text><text x='34' y='87' fill='#81979f' font-size='12'>NATIVE RUST / SIX LIVE SCENES / SMOOTH TRANSITIONS / 60 FPS</text>");
    let palettes = [
        Palette::Emerald,
        Palette::Cyan,
        Palette::Violet,
        Palette::Amber,
        Palette::Rainbow,
        Palette::Cyan,
    ];
    for (i, (&mode, palette)) in Mode::ALL.iter().zip(palettes).enumerate() {
        let x = 32 + (i % 2) * 736;
        let y = 112 + (i / 2) * 356;
        write!(svg, "<rect x='{x}' y='{y}' width='708' height='336' rx='12' fill='#060b10' stroke='#28393f'/><text x='{}' y='{}' fill='#d9e7eb' font-size='16'>0{} / {}</text>", x + 16, y + 29, i + 1, mode.name().to_ascii_uppercase()).unwrap();
        let rgb = color(palette, 0.8, 0.2);
        write!(
            svg,
            "<text x='{}' y='{}' fill='rgb({},{},{})' font-size='11'>{}</text>",
            x + 580,
            y + 28,
            rgb[0],
            rgb[1],
            rgb[2],
            palette.name().to_ascii_uppercase()
        )
        .unwrap();
        let mut scene = Scene::new(84, 20, mode, 0.8, GlyphSet::Matrix, 99);
        for _ in 0..180 {
            scene.update(1.0 / 60.0);
        }
        for (index, cell) in scene.frame().iter().enumerate() {
            if cell.level < 0.025 {
                continue;
            }
            let rgb = color(palette, cell.level, cell.hue);
            write!(
                svg,
                "<text x='{}' y='{}' fill='rgb({},{},{})' font-size='12'>{}</text>",
                x + 17 + (index % 84) * 8,
                y + 56 + (index / 84) * 13,
                rgb[0],
                rgb[1],
                rgb[2],
                escaped(cell.glyph)
            )
            .unwrap();
        }
        let subtitle = match mode {
            Mode::Rain => "Layered streams / graduated trails / bright heads",
            Mode::Waterfall => "Flowing curtains / coordinated motion / depth",
            Mode::Waves => "Interference ripples / moving contours",
            Mode::Spiral => "Three rotating arms / balanced orbital motion",
            Mode::Glitch => "Fragmented rain / brief horizontal displacements",
            Mode::Starfield => "Depth particles / outward motion / directional trails",
        };
        write!(
            svg,
            "<text x='{}' y='{}' fill='#809da7' font-size='11'>{subtitle}</text>",
            x + 17,
            y + 319
        )
        .unwrap();
    }
    svg.push_str("<text x='34' y='1214' fill='#adc6cd' font-size='12'>1-6 modes   c color   g glyphs   [ ] density   d demo   +/- speed   space pause   ? help   q quit</text><text x='1280' y='1214' fill='#607982' font-size='11'>ACTUAL RUST FRAMES</text></g></svg>");
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "nmatrix-preview.svg".into());
    std::fs::write(path, svg).unwrap();
}
