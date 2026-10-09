use nmatrix::profile::{NameConfig, clip_text, display_width, validate_name};
use nmatrix::signature::name_mask;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "nmatrix-profile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn names_are_trimmed_and_reject_terminal_controls_or_excess_length() {
    assert_eq!(validate_name("  Mohamed  ").unwrap(), "Mohamed");
    assert_eq!(validate_name("محمد").unwrap(), "محمد");
    for name in [
        "",
        "   ",
        "hello\nworld",
        "\x1b[31mZack",
        "x\u{0085}",
        "abcdefghijklmnopqrstu",
    ] {
        assert!(validate_name(name).is_err());
    }
}
#[test]
fn unicode_name_round_trips_without_rewriting_config() {
    let temp = Temp::new();
    let config = NameConfig::new(temp.0.join("nmatrix/config"));
    assert_eq!(config.load().unwrap(), None);
    config.save("محمد").unwrap();
    let before = std::fs::metadata(temp.0.join("nmatrix/config"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(config.load().unwrap().as_deref(), Some("محمد"));
    assert_eq!(
        before,
        std::fs::metadata(temp.0.join("nmatrix/config"))
            .unwrap()
            .modified()
            .unwrap()
    );
}
#[test]
fn rejected_save_preserves_previous_profile_and_private_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let temp = Temp::new();
    let path = temp.0.join("nmatrix/config");
    let config = NameConfig::new(path.clone());
    config.save("Zack").unwrap();
    assert!(config.save("bad\rname").is_err());
    assert_eq!(config.load().unwrap().as_deref(), Some("Zack"));
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[test]
fn unusable_config_target_reports_failure() {
    let temp = Temp::new();
    std::fs::create_dir_all(temp.0.join("config")).unwrap();
    assert!(NameConfig::new(temp.0.join("config")).save("Zack").is_err());
}
#[test]
fn clipping_handles_unicode_width_without_splitting_text() {
    assert_eq!(display_width("你好"), 4);
    assert_eq!(display_width("A\u{0301}"), 1);
    assert_eq!(clip_text("你好Zack", 3), "你");
    assert_eq!(clip_text("محمد", 2), "مح");
}
#[test]
fn emoji_and_combining_marks_stay_within_terminal_columns() {
    assert_eq!(display_width("⌚⌛"), 4);
    assert_eq!(clip_text(&"⌚".repeat(20), 16), "⌚".repeat(8));
    assert_eq!(display_width("❤️"), 2);
    assert_eq!(clip_text("❤️Zack", 1), "");
    assert_eq!(clip_text("❤️Zack", 2), "❤️");
    assert_eq!(clip_text("A\u{0301}Zack", 1), "A\u{0301}");
    assert_eq!(clip_text("x\u{0085}y", 2), "xy");
}
#[test]
fn large_name_masks_fit_and_unicode_falls_back_to_readable_text() {
    let mask = name_mask("Zack", 90, 20).unwrap();
    assert!(mask.len() <= 20 && mask.iter().all(|row| row.len() <= 90));
    assert!(mask.iter().flatten().any(|&pixel| pixel));
    assert!(name_mask("Zack", 3, 2).is_none());
    assert!(name_mask("محمد", 90, 20).is_none());
}
