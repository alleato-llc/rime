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
fn color_round_trips_through_hex() {
    // color_hex ∘ parse_color is the identity on well-formed hex (both quantize
    // to u8, so no precision drift). Opaque and alpha forms both survive.
    for hex in ["#000000", "#ffffff", "#3b4252", "#ff8800", "#12345678"] {
        let c = parse_color(hex).expect("valid hex parses");
        assert_eq!(color_hex(c), hex, "{hex} should round-trip");
    }

    // Uppercase input parses; the emitted form is canonical lowercase, opaque
    // colors drop the alpha byte.
    assert_eq!(color_hex(parse_color("#ABCDEF").unwrap()), "#abcdef");
    assert_eq!(color_hex(parse_color("#11223344").unwrap()), "#11223344");

    // Malformed input is rejected, not guessed.
    for bad in ["abcdef", "#fff", "#gggggg", "#12345", "", "#"] {
        assert!(parse_color(bad).is_none(), "{bad:?} should not parse");
    }
}

#[test]
fn palette_tokens_read_and_write_by_key() {
    // The nine advertised keys, no duplicates.
    assert_eq!(PALETTE_KEYS.len(), 9);
    let unique: std::collections::HashSet<_> = PALETTE_KEYS.iter().collect();
    assert_eq!(unique.len(), 9, "palette keys must be unique");

    // Every key resolves on a real palette; an unknown one is None.
    for &key in PALETTE_KEYS {
        assert!(DRACULA.color(key).is_some(), "{key} should resolve");
    }
    assert!(DRACULA.color("nope").is_none());

    // set(key, c) then color(key) is the value written; an unknown key is a
    // silent no-op, not a panic or a stray write.
    let mut p = DRACULA;
    let marker = parse_color("#abcdef").unwrap();
    for &key in PALETTE_KEYS {
        p.set(key, marker);
        assert_eq!(
            p.color(key),
            Some(marker),
            "{key} should read back what was set"
        );
    }
    let before = p;
    p.set("nope", Color::BLACK);
    assert_eq!(
        p.color("bg"),
        before.color("bg"),
        "unknown key must not write"
    );
}

#[test]
fn builtin_catalog_is_well_formed() {
    let themes = builtin_themes();
    assert!(!themes.is_empty());

    // Names are non-empty and unique — a theme picker keys off them.
    let names: std::collections::HashSet<_> = themes.iter().map(|(n, _, _)| *n).collect();
    assert_eq!(
        names.len(),
        themes.len(),
        "builtin theme names must be unique"
    );
    assert!(themes.iter().all(|(n, _, _)| !n.is_empty()));

    // Every catalog palette resolves all nine tokens.
    for (name, palette, _) in themes {
        for &key in PALETTE_KEYS {
            assert!(palette.color(key).is_some(), "{name} is missing {key}");
        }
    }

    // The bool flags dark vs light; the catalog ships both kinds.
    assert!(
        themes.iter().any(|(_, _, dark)| *dark),
        "expected a dark theme"
    );
    assert!(
        themes.iter().any(|(_, _, dark)| !*dark),
        "expected a light theme"
    );
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
