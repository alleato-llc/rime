//! A section heading inside a card.

use iced::widget::text;
use iced::Element;

use crate::theme::tokens;

/// A section heading — slightly larger than body, inked.
pub fn section<'a, M: 'a>(label: &str) -> Element<'a, M> {
    text(label.to_string()).size(15).color(tokens().ink).into()
}
