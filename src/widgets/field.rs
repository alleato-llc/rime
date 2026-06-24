//! A labelled field: a small muted caption above any input element.

use iced::widget::{column, text};
use iced::{Element, Length};

use crate::theme::tokens;

/// Stack a muted `label` caption above `input`.
pub fn labeled<'a, M: 'a>(label: &'a str, input: impl Into<Element<'a, M>>) -> Element<'a, M> {
    column![text(label).size(12).color(tokens().muted), input.into()]
        .spacing(3)
        .width(Length::Fill)
        .into()
}
