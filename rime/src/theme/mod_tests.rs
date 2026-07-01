use super::*;

#[test]
fn scope_nests_and_restores() {
    // Outside any scope the default (GITHUB) is active.
    assert_eq!(tokens().bg, GITHUB.bg);
    scope(DRACULA, || {
        assert_eq!(tokens().bg, DRACULA.bg);
        // A nested scope overrides, then restores its parent on drop.
        scope(GITHUB, || assert_eq!(tokens().bg, GITHUB.bg));
        assert_eq!(tokens().bg, DRACULA.bg);
    });
    assert_eq!(tokens().bg, GITHUB.bg);
}

#[test]
fn theme_choice_toggles() {
    assert_eq!(ThemeChoice::Dark.toggled(), ThemeChoice::Light);
    assert_eq!(ThemeChoice::Light.toggled(), ThemeChoice::Dark);
    assert_eq!(ThemeChoice::default(), ThemeChoice::Dark);
}

#[test]
fn persistence_round_trips_and_defaults_to_dark() {
    let dir = std::env::temp_dir().join("rime-theme-test");
    let path = dir.join("choice");
    let _ = std::fs::remove_file(&path);

    // Missing file → Dark.
    assert_eq!(load(&path), ThemeChoice::Dark);

    save(&path, ThemeChoice::Light);
    assert_eq!(load(&path), ThemeChoice::Light);
    save(&path, ThemeChoice::Dark);
    assert_eq!(load(&path), ThemeChoice::Dark);

    let _ = std::fs::remove_file(&path);
}
