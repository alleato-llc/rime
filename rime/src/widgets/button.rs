//! Rounded buttons in the house style. These are the answer to "I don't want to
//! customize each button" — the corner radius, padding, and label size live here
//! once, so a screen writes `button::primary("Run", Message::Run)` and never
//! repeats `.padding(..).style(rounded(..))`. For a button whose content isn't a
//! plain label (an icon column, a different size) reach for [`crate::theme::rounded`]
//! directly.
//!
//! Generic over the message type so the set is reusable for any `M`; the press
//! message is cloned by iced on each press, hence the `Clone` bound.

use iced::widget::{button, text, Button};
use iced::Theme;

use crate::theme::rounded;

type BaseStyle = fn(&Theme, button::Status) -> button::Style;

fn styled<'a, M: Clone + 'a>(label: &str, on_press: M, base: BaseStyle) -> Button<'a, M> {
    button(text(label.to_string()).size(13))
        .on_press(on_press)
        .padding([7, 16])
        .style(rounded(base))
}

/// The primary call-to-action.
pub fn primary<'a, M: Clone + 'a>(label: &str, on_press: M) -> Button<'a, M> {
    styled(label, on_press, button::primary)
}

/// A neutral secondary action.
pub fn secondary<'a, M: Clone + 'a>(label: &str, on_press: M) -> Button<'a, M> {
    styled(label, on_press, button::secondary)
}

/// A destructive action (Delete, Stop).
pub fn danger<'a, M: Clone + 'a>(label: &str, on_press: M) -> Button<'a, M> {
    styled(label, on_press, button::danger)
}

/// A quiet, borderless action (Cancel, Back).
pub fn ghost<'a, M: Clone + 'a>(label: &str, on_press: M) -> Button<'a, M> {
    styled(label, on_press, button::text)
}
