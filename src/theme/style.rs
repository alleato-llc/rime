//! The small style functions that give every built-in iced widget the house
//! corner radius. A "style" in iced is `fn(&Theme, Status) -> SomeStyle`; these
//! wrap the iced defaults so the one knob — radius — lives in one place.

use iced::widget::{button, pick_list, text_editor, text_input};
use iced::Theme;

/// The house corner radius, in logical pixels.
const RADIUS: f32 = 8.0;

/// Wrap a built-in button style so its corners are softly rounded — the one knob
/// that makes every button feel consistent.
pub fn rounded(
    base: fn(&Theme, button::Status) -> button::Style,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut s = base(theme, status);
        s.border.radius = RADIUS.into();
        s
    }
}

/// A rounded text-input style.
pub fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut s = text_input::default(theme, status);
    s.border.radius = RADIUS.into();
    s
}

/// A rounded pick-list (dropdown) style.
pub fn pick_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let mut s = pick_list::default(theme, status);
    s.border.radius = RADIUS.into();
    s
}

/// A rounded text-editor style.
pub fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let mut s = text_editor::default(theme, status);
    s.border.radius = RADIUS.into();
    s
}
