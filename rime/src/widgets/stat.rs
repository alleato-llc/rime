//! A big-number readout with a caption — a screen's vitals.

use iced::widget::{column, text};
use iced::Element;

use crate::theme::tokens;

/// A large `value` over a small muted `label`.
pub fn stat<'a, M: 'a>(label: &str, value: String) -> Element<'a, M> {
    column![
        text(value).size(22).color(tokens().ink),
        text(label.to_string()).size(12).color(tokens().muted)
    ]
    .spacing(2)
    .into()
}
