//! The visual language: the semantic [`Palette`], the active-palette channel the
//! widgets read, theme persistence, and the small style functions that shape every
//! built-in iced widget consistently.
//!
//! Styling here is data returned by functions, not objects: a "style" is an
//! `fn(&Theme, Status) -> SomeStyle`. The flagship is [`rounded`], which wraps a
//! built-in button style so every button shares one corner radius from one place.
//!
//! rime owns the theming *system* and ships defaults ([`DRACULA`], [`GITHUB`]); a
//! host app overrides the *content* — it can construct its own [`Palette`], choose
//! which one is active, and decide where the choice persists ([`load`]/[`save`]).
//! The token *vocabulary* (the nine [`Palette`] fields) is fixed: that is exactly
//! what lets a component render correctly under any palette.

use std::cell::Cell;
use std::path::Path;

use iced::{Color, Theme};

mod palettes;
mod registry;
mod style;

pub use palettes::{
    builtin_themes, DRACULA, GITHUB, GRUVBOX_DARK, NORD, SOLARIZED_DARK, SOLARIZED_LIGHT,
};
pub use registry::{NamedTheme, ThemeRegistry};
pub use style::{editor_style, input_style, pick_style, rounded};

/// The nine semantic colors every component draws with — one coherent set per
/// theme. The built-in iced widgets (buttons, inputs, scrollbars) follow the
/// matching [`iced::Theme`] instead (see [`Palette::iced_theme`]), so the whole
/// app stays in step.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// The window background.
    pub bg: Color,
    /// Raised surfaces — cards, panels.
    pub surface: Color,
    /// Primary text.
    pub ink: Color,
    /// Secondary / caption text.
    pub muted: Color,
    /// Hairline borders and dividers.
    pub hairline: Color,
    /// The brand / call-to-action color.
    pub accent: Color,
    /// Success / healthy state.
    pub success: Color,
    /// Warning / caution state.
    pub warn: Color,
    /// Error / destructive state.
    pub danger: Color,
}

impl Palette {
    /// Derive an [`iced::Theme`] for the built-in widgets from this palette, so a
    /// custom palette and the built-in widgets stay coherent from one source. iced
    /// carries five slots; the surface tokens (`surface`/`muted`/`hairline`) have
    /// no iced equivalent and are used only by rime's own components.
    pub fn iced_theme(&self, name: impl Into<String>) -> Theme {
        Theme::custom(
            name.into(),
            iced::theme::Palette {
                background: self.bg,
                text: self.ink,
                primary: self.accent,
                success: self.success,
                warning: self.warn,
                danger: self.danger,
            },
        )
    }

    /// Read a token by its [`PALETTE_KEYS`] name (for a theme editor / serializer);
    /// `None` for an unknown key.
    pub fn color(&self, key: &str) -> Option<Color> {
        Some(match key {
            "bg" => self.bg,
            "surface" => self.surface,
            "ink" => self.ink,
            "muted" => self.muted,
            "hairline" => self.hairline,
            "accent" => self.accent,
            "success" => self.success,
            "warn" => self.warn,
            "danger" => self.danger,
            _ => return None,
        })
    }

    /// Set a token by its [`PALETTE_KEYS`] name; unknown keys are ignored.
    pub fn set(&mut self, key: &str, c: Color) {
        match key {
            "bg" => self.bg = c,
            "surface" => self.surface = c,
            "ink" => self.ink = c,
            "muted" => self.muted = c,
            "hairline" => self.hairline = c,
            "accent" => self.accent = c,
            "success" => self.success = c,
            "warn" => self.warn = c,
            "danger" => self.danger = c,
            _ => {}
        }
    }
}

/// The nine palette token names, in display order — for a theme editor's UI rows
/// and for serializing the `[ui]` table.
pub const PALETTE_KEYS: &[&str] = &[
    "bg", "surface", "ink", "muted", "hairline", "accent", "success", "warn", "danger",
];

/// Parse `#rrggbb` or `#rrggbbaa` into a [`Color`]. The inverse of [`color_hex`].
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
    match s.len() {
        6 => Some(Color::from_rgb8(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::from_rgba8(
            byte(0)?,
            byte(2)?,
            byte(4)?,
            byte(6)? as f32 / 255.0,
        )),
        _ => None,
    }
}

/// Format a [`Color`] as `#rrggbb` (opaque) or `#rrggbbaa` (with alpha). The
/// inverse of [`parse_color`].
pub fn color_hex(c: Color) -> String {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (r, g, b, a) = (q(c.r), q(c.g), q(c.b), q(c.a));
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// The built-in dark/light choice. A convenience for apps that just want the two
/// shipped palettes; apps wanting a bespoke look construct their own [`Palette`]
/// and pass it straight to [`enter`]/[`scope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Dracula (dark) — the default.
    #[default]
    Dark,
    /// GitHub (light).
    Light,
}

impl ThemeChoice {
    /// The rime [`Palette`] for this choice.
    pub fn palette(self) -> Palette {
        match self {
            ThemeChoice::Dark => DRACULA,
            ThemeChoice::Light => GITHUB,
        }
    }

    /// The [`iced::Theme`] the built-in widgets follow — the rich built-in Dracula
    /// for dark, a GitHub-flavoured custom palette for light — kept in step with
    /// [`palette`](Self::palette).
    pub fn theme(self) -> Theme {
        match self {
            ThemeChoice::Dark => Theme::Dracula,
            ThemeChoice::Light => GITHUB.iced_theme("GitHub"),
        }
    }

    /// The other choice — handy for a toggle.
    pub fn toggled(self) -> Self {
        match self {
            ThemeChoice::Dark => ThemeChoice::Light,
            ThemeChoice::Light => ThemeChoice::Dark,
        }
    }
}

thread_local! {
    /// The palette in force for the current `view()` pass. Opened once at the top
    /// of `view()` via [`enter`]/[`scope`] and read by the (parameterless) widget
    /// helpers; `view()` is synchronous and single-threaded, so this is safe and
    /// keeps the helper signatures clean.
    static PALETTE: Cell<Palette> = const { Cell::new(GITHUB) };
}

/// The palette in force for the current render pass. Components call this; the
/// host sets it with [`enter`]/[`scope`].
pub fn tokens() -> Palette {
    PALETTE.with(|p| p.get())
}

/// An open palette scope. While it is alive [`tokens`] returns its palette; when
/// it drops the previous palette is restored (so nested scopes compose, and an
/// unwind can't leak a palette).
#[must_use = "the palette is only active while the Scope is held"]
pub struct Scope {
    prev: Palette,
}

impl Drop for Scope {
    fn drop(&mut self) {
        PALETTE.with(|p| p.set(self.prev));
    }
}

/// Make `palette` the active one until the returned [`Scope`] drops. Call this
/// once at the top of your `view()` and bind it (`let _scope = …`).
pub fn enter(palette: Palette) -> Scope {
    let prev = PALETTE.with(|p| p.replace(palette));
    Scope { prev }
}

/// Run `f` with `palette` active, restoring the previous one afterwards. The
/// closure form of [`enter`].
pub fn scope<T>(palette: Palette, f: impl FnOnce() -> T) -> T {
    let _scope = enter(palette);
    f()
}

/// Read a persisted [`ThemeChoice`] from `path` (the host owns the path).
/// Anything other than `light` — including a missing file — reads as [`Dark`].
///
/// [`Dark`]: ThemeChoice::Dark
pub fn load(path: &Path) -> ThemeChoice {
    match std::fs::read_to_string(path) {
        Ok(s) if s.trim() == "light" => ThemeChoice::Light,
        _ => ThemeChoice::Dark,
    }
}

/// Persist `choice` to `path`, creating the parent directory if needed. Errors are
/// swallowed — a theme preference is not worth failing a render over.
pub fn save(path: &Path, choice: ThemeChoice) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(
        path,
        match choice {
            ThemeChoice::Dark => "dark",
            ThemeChoice::Light => "light",
        },
    );
}

#[cfg(test)]
mod tests {
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
}
