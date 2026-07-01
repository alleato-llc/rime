use super::*;

/// A minimal theme: just a name, serialized as its own text.
struct Mini(String);
impl NamedTheme for Mini {
    fn theme_name(&self) -> &str {
        &self.0
    }
    fn to_toml(&self) -> String {
        self.0.clone()
    }
}
fn resolve_builtin(k: &str) -> Option<Mini> {
    (k == "base").then(|| Mini("Base".into()))
}
fn parse(s: &str, _hint: &str) -> Result<Mini, String> {
    Ok(Mini(s.trim().to_string()))
}
fn fallback() -> Mini {
    Mini("Base".into())
}

fn registry(dir: Option<PathBuf>) -> ThemeRegistry<Mini> {
    ThemeRegistry::new(vec!["Base".into()], resolve_builtin, parse, fallback, dir)
}

#[test]
fn builtins_resolve_and_are_readonly() {
    let r = registry(None);
    assert_eq!(r.get("base").0, "Base");
    assert_eq!(r.get("nope").0, "Base"); // fallback
    assert!(r.is_builtin("Base"));
    assert!(r.is_builtin("base")); // case-insensitive
    assert!(!r.is_builtin("Custom"));
}

#[test]
fn save_registers_in_place_and_delete_removes() {
    let dir = std::env::temp_dir().join("rime-registry-test");
    let _ = std::fs::remove_dir_all(&dir);
    let mut r = registry(Some(dir.clone()));

    let path = r.save(&Mini("Custom".into())).unwrap();
    assert!(path.exists());
    // No reload: the registry sees it immediately.
    assert!(r.names().iter().any(|n| n == "Custom"));
    assert!(!r.is_builtin("Custom"));
    assert_eq!(r.get("Custom").0, "Custom");

    r.delete("Custom").unwrap();
    assert!(!path.exists());
    assert!(!r.names().iter().any(|n| n == "Custom")); // dropped in place
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deleting_a_user_override_keeps_the_builtin() {
    let dir = std::env::temp_dir().join("rime-registry-override-test");
    let _ = std::fs::remove_dir_all(&dir);
    let mut r = registry(Some(dir.clone()));

    // A user file named like the built-in shadows it (no duplicate name).
    r.save(&Mini("Base".into())).unwrap();
    assert_eq!(
        r.names()
            .iter()
            .filter(|n| n.eq_ignore_ascii_case("base"))
            .count(),
        1
    );

    // Deleting the override leaves the built-in resolvable + listed.
    r.delete("Base").unwrap();
    assert!(r.is_builtin("Base"));
    assert!(r.names().iter().any(|n| n.eq_ignore_ascii_case("base")));
    assert_eq!(r.get("Base").0, "Base");
    let _ = std::fs::remove_dir_all(&dir);
}
