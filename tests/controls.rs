use nmatrix::engine::{Cell, GlyphSet, Mode, Palette};
use nmatrix::options::{Controls, InputDecoder, Key, Options};
use nmatrix::render::{ColorDepth, Renderer, blend_frames};
use std::time::Duration;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}

#[test]
fn arguments_validate_numbers_and_choices() {
    let opts = Options::parse(args(&[])).unwrap();
    assert_eq!(
        (opts.mode, opts.palette, opts.fps),
        (Mode::Rain, Palette::Emerald, 60)
    );
    for values in [
        vec!["--speed", "NaN"],
        vec!["--speed", "inf"],
        vec!["--speed", "0"],
        vec!["--density", "0"],
        vec!["--density", "1.5"],
        vec!["--fps", "0"],
        vec!["--fps", "121"],
        vec!["--mode", "bad"],
        vec!["--seed"],
        vec!["--unknown"],
    ] {
        assert!(Options::parse(args(&values)).is_err(), "{values:?}");
    }
    let opts = Options::parse(args(&[
        "--mode",
        "spiral",
        "--palette",
        "rainbow",
        "--glyphs",
        "binary",
        "--demo",
    ]))
    .unwrap();
    assert_eq!(opts.glyphs, GlyphSet::Binary);
    assert!(opts.demo);
}

#[test]
fn controls_cycle_and_bound_all_settings() {
    let mut c = Controls::new(Options::parse(args(&[])).unwrap());
    for (i, &mode) in Mode::ALL[..6].iter().enumerate() {
        c.key(Key::Char(b'1' + i as u8));
        assert_eq!(c.mode, mode);
    }
    for &mode in &Mode::ALL[6..] {
        c.key(Key::Right);
        assert_eq!(c.mode, mode);
    }
    c.key(Key::Right);
    assert_eq!(c.mode, Mode::Rain);
    c.key(Key::Left);
    assert_eq!(c.mode, Mode::Fireworks);
    c.key(Key::Char(b'c'));
    assert_eq!(c.palette, Palette::Cyan);
    c.key(Key::Char(b'g'));
    assert_eq!(c.glyphs, GlyphSet::Binary);
    c.key(Key::Char(b'd'));
    assert!(c.demo);
    for _ in 0..100 {
        c.key(Key::Char(b'+'));
        c.key(Key::Char(b']'));
    }
    assert_eq!(c.speed, 8.0);
    assert!((c.density - 1.4).abs() < 0.001);
    for _ in 0..100 {
        c.key(Key::Char(b'-'));
        c.key(Key::Char(b'['));
    }
    assert_eq!(c.speed, 0.15);
    assert!((c.density - 0.2).abs() < 0.001);
    c.key(Key::Char(b' '));
    assert!(c.paused);
    c.key(Key::Char(b'h'));
    assert!(!c.hud);
    c.key(Key::Char(b'?'));
    assert!(c.help);
    c.key(Key::Char(b'q'));
    assert!(!c.running);
}

#[test]
fn arrow_bytes_can_arrive_in_separate_reads() {
    let mut decoder = InputDecoder::default();
    decoder.push(b"\x1b", Duration::ZERO);
    assert!(decoder.events(Duration::from_millis(5)).is_empty());
    decoder.push(b"[", Duration::from_millis(10));
    assert!(decoder.events(Duration::from_millis(12)).is_empty());
    decoder.push(b"C", Duration::from_millis(15));
    assert_eq!(decoder.events(Duration::from_millis(15)), vec![Key::Right]);
    decoder.push(b"\x1bOD", Duration::from_millis(20));
    assert_eq!(decoder.events(Duration::from_millis(20)), vec![Key::Left]);
    decoder.push(b"\x1b", Duration::from_millis(30));
    assert_eq!(decoder.events(Duration::from_millis(80)), vec![Key::Escape]);
}

#[test]
fn rendering_falls_back_and_omits_unchanged_cells() {
    let frame = vec![
        Cell {
            glyph: b'A',
            level: 0.7,
            hue: 0.3,
        },
        Cell {
            glyph: b'B',
            level: 1.0,
            hue: 0.0,
        },
    ];
    for depth in [
        ColorDepth::TrueColor,
        ColorDepth::Colors256,
        ColorDepth::Basic,
    ] {
        let mut renderer = Renderer::new(depth);
        let first = renderer.render(&frame, 2, 1, Palette::Cyan, None);
        assert!(first.contains('A') && first.contains('B'));
        if depth == ColorDepth::TrueColor {
            assert!(first.contains("38;2;"));
        } else {
            assert!(!first.contains("38;2;") && !first.contains("48;2;"));
        }
        assert_eq!(renderer.render(&frame, 2, 1, Palette::Cyan, None), "");
        renderer.invalidate();
        assert!(
            !renderer
                .render(&frame, 2, 1, Palette::Cyan, None)
                .is_empty()
        );
    }
}

#[test]
fn crossfade_preserves_endpoints_and_valid_frames() {
    let old = vec![
        Cell {
            glyph: b'A',
            level: 1.0,
            hue: 0.9
        };
        50
    ];
    let new = vec![
        Cell {
            glyph: b'B',
            level: 0.4,
            hue: 0.1
        };
        50
    ];
    assert_eq!(blend_frames(&old, &new, 0.0), old);
    assert_eq!(blend_frames(&old, &new, 1.0), new);
    let midway = blend_frames(&old, &new, 0.5);
    assert!(
        midway
            .iter()
            .all(|c| c.level > 0.4 && c.level < 1.0 && c.hue.is_finite())
    );
    assert!(midway.iter().any(|c| c.glyph == b'A'));
    assert!(midway.iter().any(|c| c.glyph == b'B'));
}
