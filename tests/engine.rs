use nmatrix::engine::{Cell, GlyphSet, Mode, Palette, Scene, color};

fn valid(frame: &[Cell], width: usize, height: usize) {
    assert_eq!(frame.len(), width * height);
    for cell in frame {
        assert!(cell.glyph.is_ascii_graphic() || cell.glyph == b' ');
        assert!(cell.level.is_finite() && (0.0..=1.0).contains(&cell.level));
        assert!(cell.hue.is_finite());
    }
}

#[test]
fn all_scenes_animate_with_valid_bounded_frames() {
    let mut fingerprints = Vec::new();
    for &mode in Mode::ALL {
        for (width, height) in [(1, 1), (12, 4), (100, 30)] {
            let mut scene = Scene::new(width, height, mode, 0.7, GlyphSet::Matrix, 91);
            for _ in 0..120 {
                scene.update(1.0 / 60.0);
                valid(scene.frame(), width, height);
            }
            if width == 100 {
                let before = scene.frame().to_vec();
                assert!(before.iter().any(|c| c.level > 0.4));
                fingerprints.push(before.clone());
                scene.update(0.3);
                assert_ne!(before, scene.frame());
            }
        }
    }
    for i in 0..fingerprints.len() {
        for j in i + 1..fingerprints.len() {
            assert_ne!(fingerprints[i], fingerprints[j]);
        }
    }
}

#[test]
fn seed_and_resize_are_repeatable() {
    for &mode in Mode::ALL {
        let mut a = Scene::new(50, 20, mode, 0.8, GlyphSet::Matrix, 32);
        let mut b = Scene::new(50, 20, mode, 0.8, GlyphSet::Matrix, 32);
        for _ in 0..40 {
            a.update(0.05);
            b.update(0.05);
            assert_eq!(a.frame(), b.frame());
        }
        for (width, height) in [(1, 1), (120, 40), (2, 1), (0, 0)] {
            a.resize(width, height);
            a.update(0.2);
            valid(a.frame(), width.max(1), height.max(1));
        }
    }
}

#[test]
fn rendering_frequency_does_not_change_simulation() {
    for &mode in Mode::ALL {
        let mut a = Scene::new(90, 24, mode, 0.7, GlyphSet::Matrix, 123);
        let mut b = Scene::new(90, 24, mode, 0.7, GlyphSet::Matrix, 123);
        for i in 0..120 {
            a.update(1.0 / 60.0);
            b.update(1.0 / 60.0);
            a.frame();
            if i % 60 == 0 {
                b.frame();
            }
        }
        assert!((a.time - 2.0).abs() < 0.001);
        assert_eq!(a.frame(), b.frame());
    }
}

#[test]
fn glyph_sets_and_density_change_visible_output() {
    for glyphs in [GlyphSet::Binary, GlyphSet::Hex] {
        for &mode in Mode::ALL {
            let mut scene = Scene::new(100, 30, mode, 0.9, glyphs, 7);
            for _ in 0..120 {
                scene.update(1.0 / 60.0);
            }
            for cell in scene.frame().iter().filter(|c| c.level > 0.02) {
                match glyphs {
                    GlyphSet::Binary => assert!(
                        b"01".contains(&cell.glyph),
                        "{mode:?}: {}",
                        cell.glyph as char
                    ),
                    GlyphSet::Hex => assert!(
                        b"0123456789ABCDEF".contains(&cell.glyph),
                        "{mode:?}: {}",
                        cell.glyph as char
                    ),
                    _ => unreachable!(),
                }
            }
        }
    }
    let mut sparse = Scene::new(120, 40, Mode::Rain, 0.2, GlyphSet::Matrix, 8);
    let mut dense = Scene::new(120, 40, Mode::Rain, 1.2, GlyphSet::Matrix, 8);
    let visible = |scene: &mut Scene| scene.frame().iter().filter(|c| c.level > 0.04).count();
    assert!(visible(&mut dense) > visible(&mut sparse));
}

#[test]
fn palettes_fade_to_dark_and_have_distinct_midtone_colors() {
    let mut samples = Vec::new();
    for &palette in Palette::ALL {
        assert_eq!(color(palette, 0.0, 0.2), [0, 0, 0]);
        let middle = color(palette, 0.6, 0.2);
        let head = color(palette, 1.0, 0.2);
        assert!(
            head.iter().map(|&c| c as u32).sum::<u32>()
                > middle.iter().map(|&c| c as u32).sum::<u32>()
        );
        assert!(!samples.contains(&middle));
        samples.push(middle);
    }
}
