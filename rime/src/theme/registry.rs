//! A generic theme registry: built-in themes plus user themes saved as TOML files
//! in a directory the host owns. Domain-free — it shuffles files and names and
//! delegates (de)serialization to the app (the `parse` fn it is given and the
//! [`NamedTheme`] trait), so any GUI reuses it for its own theme type. fed's
//! `patina::Theme` (chrome + editor + syntax colors) is one such type; a
//! palette-only app would define a smaller one. The registry knows nothing about
//! either — only how to name, list, resolve, and persist them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// What the registry needs from a theme value to persist it: its name (the file
/// stem) and its TOML serialization. Parsing back is the `parse` fn given to
/// [`ThemeRegistry::new`] — a free fn, so it needs no existing value to call.
pub trait NamedTheme {
    /// The theme's display name (also the user-file stem).
    fn theme_name(&self) -> &str;
    /// Serialize to TOML, round-trippable through the registry's `parse` fn.
    fn to_toml(&self) -> String;
}

/// Built-in themes (read-only) plus user themes (TOML files in `user_dir`),
/// resolvable by name. Generic over the app's theme type `T`; the app supplies how
/// to resolve a built-in, how to parse a file, and a fallback for unknown/broken
/// themes. The host owns `user_dir` (rime never guesses a config path).
pub struct ThemeRegistry<T> {
    names: Vec<String>,
    user: HashMap<String, PathBuf>,
    builtins: Vec<String>,
    resolve_builtin: fn(&str) -> Option<T>,
    parse: fn(&str, &str) -> Result<T, String>,
    fallback: fn() -> T,
    user_dir: Option<PathBuf>,
}

impl<T: NamedTheme> ThemeRegistry<T> {
    /// Build a registry. `builtins` are the read-only built-in names;
    /// `resolve_builtin(lowercased_name)` constructs one. `parse(toml, name_hint)`
    /// reads a user file. `fallback()` is used when a name is unknown or a file
    /// won't parse. `user_dir` is where user `.toml` themes live; `None` disables
    /// user themes (built-ins only).
    pub fn new(
        builtins: Vec<String>,
        resolve_builtin: fn(&str) -> Option<T>,
        parse: fn(&str, &str) -> Result<T, String>,
        fallback: fn() -> T,
        user_dir: Option<PathBuf>,
    ) -> Self {
        let mut names = builtins.clone();
        let mut user = HashMap::new();
        if let Some(dir) = &user_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let key = stem.to_lowercase();
                            if !names.iter().any(|n| n.to_lowercase() == key) {
                                names.push(stem.to_string());
                            }
                            user.insert(key, path); // a user file overrides a same-named built-in
                        }
                    }
                }
            }
        }
        Self {
            names,
            user,
            builtins,
            resolve_builtin,
            parse,
            fallback,
            user_dir,
        }
    }

    /// All theme names (built-in + user), for a picker.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Whether `name` is a built-in (case-insensitive) — built-ins are read-only.
    pub fn is_builtin(&self, name: &str) -> bool {
        let key = name.to_lowercase();
        self.builtins.iter().any(|n| n.to_lowercase() == key)
    }

    /// Resolve a theme by name (case-insensitive). A user file overrides a built-in
    /// of the same name; an unknown name or a parse error yields `fallback()`.
    pub fn get(&self, name: &str) -> T {
        let key = name.to_lowercase();
        if let Some(path) = self.user.get(&key) {
            match std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|s| (self.parse)(&s, name))
            {
                Ok(theme) => return theme,
                Err(e) => eprintln!("rime: theme `{name}` ({path:?}): {e}; using fallback"),
            }
        }
        (self.resolve_builtin)(&key).unwrap_or_else(self.fallback)
    }

    /// The directory user themes live in (if any).
    pub fn user_dir(&self) -> Option<&Path> {
        self.user_dir.as_deref()
    }

    /// Save `theme` as `<user_dir>/<name>.toml`, overwriting. Returns the path.
    /// The registry updates in place — the theme is immediately visible to
    /// [`names`](Self::names) and [`get`](Self::get) with no reload.
    pub fn save(&mut self, theme: &T) -> std::io::Result<PathBuf> {
        let dir = self.user_dir.clone().ok_or_else(no_user_dir)?;
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.toml", theme.theme_name()));
        std::fs::write(&path, theme.to_toml())?;
        self.register(theme.theme_name(), path.clone());
        Ok(path)
    }

    /// Delete a user theme file by name. Updates the registry in place: the name
    /// drops out unless it also names a built-in, in which case the built-in
    /// remains (the user file was overriding it).
    pub fn delete(&mut self, name: &str) -> std::io::Result<()> {
        let dir = self.user_dir.clone().ok_or_else(no_user_dir)?;
        std::fs::remove_file(dir.join(format!("{name}.toml")))?;
        let key = name.to_lowercase();
        self.user.remove(&key);
        if !self.builtins.iter().any(|n| n.to_lowercase() == key) {
            self.names.retain(|n| n.to_lowercase() != key);
        }
        Ok(())
    }

    /// Copy a theme `.toml` into the user dir, returning its name (file stem).
    /// Registered in place (no reload needed).
    pub fn import(&mut self, src: &Path) -> std::io::Result<String> {
        let dir = self.user_dir.clone().ok_or_else(no_user_dir)?;
        std::fs::create_dir_all(&dir)?;
        let stem = src
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("theme")
            .to_string();
        let dest = dir.join(format!("{stem}.toml"));
        std::fs::copy(src, &dest)?;
        self.register(&stem, dest);
        Ok(stem)
    }

    /// Write `theme` to `dest` as TOML (export to anywhere). Does not affect the
    /// registry — the destination is outside the user dir.
    pub fn export(&self, theme: &T, dest: &Path) -> std::io::Result<()> {
        std::fs::write(dest, theme.to_toml())
    }

    /// Record a user theme in the in-memory maps (idempotent on the name).
    fn register(&mut self, name: &str, path: PathBuf) {
        let key = name.to_lowercase();
        if !self.names.iter().any(|n| n.to_lowercase() == key) {
            self.names.push(name.to_string());
        }
        self.user.insert(key, path); // a user file overrides a same-named built-in
    }
}

fn no_user_dir() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, "no user theme directory")
}

#[cfg(test)]
mod tests {
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
        assert_eq!(r.names().iter().filter(|n| n.eq_ignore_ascii_case("base")).count(), 1);

        // Deleting the override leaves the built-in resolvable + listed.
        r.delete("Base").unwrap();
        assert!(r.is_builtin("Base"));
        assert!(r.names().iter().any(|n| n.eq_ignore_ascii_case("base")));
        assert_eq!(r.get("Base").0, "Base");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
