use nmatrix::effects::{Effects, Settings};
use nmatrix::engine::Cell;

fn bright() -> Cell { Cell { glyph: b'A', level: 0.8, hue: 0.3 } }

#[test]
fn disabled_effects_preserve_input_exactly() {
    let frame = vec![bright(); 4];
    assert_eq!(Effects::default().apply(&frame, 2, 0.1, 2.0, Settings::default()), frame);
}
#[test]
fn echo_remembers_then_decays_and_clears_on_resize_or_disable() {
    let echo = Settings { echo: true, ..Settings::default() };
    let mut fx = Effects::default();
    fx.apply(&[bright(); 4], 2, 0.0, 0.0, echo);
    let tail = fx.apply(&[Cell::default(); 4], 2, 0.2, 0.2, echo);
    assert!(tail.iter().all(|c| c.glyph == b'A' && c.level > 0.1 && c.level < 0.8));
    let faded = fx.apply(&[Cell::default(); 4], 2, 3.0, 3.2, echo);
    assert!(faded.iter().all(|c| c.level == 0.0));
    fx.apply(&[bright(); 4], 2, 0.0, 0.0, echo);
    assert_eq!(fx.apply(&[Cell::default(); 4], 4, 0.0, 0.0, echo), vec![Cell::default(); 4]);
    fx.apply(&[bright(); 4], 2, 0.0, 0.0, echo);
    fx.apply(&[Cell::default(); 4], 2, 0.0, 0.0, Settings::default());
    assert_eq!(fx.apply(&[Cell::default(); 4], 2, 0.0, 0.0, echo), vec![Cell::default(); 4]);
}
#[test]
fn pulse_and_scanlines_are_visible_and_bounded() {
    let settings = Settings { pulse: true, scanlines: true, ..Settings::default() };
    let mut fx = Effects::default();
    let early = fx.apply(&[bright(); 4], 2, 0.1, 0.0, settings);
    let later = fx.apply(&[bright(); 4], 2, 0.1, 1.0, settings);
    assert_ne!(early, later);
    assert!(early[2].level < early[0].level);
    assert!(later.iter().all(|c| c.level.is_finite() && (0.0..=1.0).contains(&c.level)));
}
