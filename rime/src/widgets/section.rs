//! Section headings: a primary [`section`] heading and a smaller muted [`caption`]
//! sub-heading.

use iced::widget::text;
use iced::Element;

use crate::theme::tokens;

/// A section heading — slightly larger than body, inked.
pub fn section<'a, M: 'a>(label: &str) -> Element<'a, M> {
    text(label.to_string()).size(15).color(tokens().ink).into()
}

/// A muted caption / sub-heading — smaller than body, in the secondary color. For
/// the small group labels above settings rows (e.g. "UI PALETTE", "BINDINGS").
pub fn caption<'a, M: 'a>(label: &str) -> Element<'a, M> {
    text(label.to_string())
        .size(11)
        .color(tokens().muted)
        .into()
}
