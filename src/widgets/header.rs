//! A screen header: a title on the left, a warn-colored notice on the right.

use iced::widget::{row, text, Space};
use iced::{Element, Length};

use crate::theme::tokens;

/// A title with an optional warn-colored `notice` pushed to the right edge.
pub fn header_row<'a, M: 'a>(title: &'a str, notice: &'a str) -> Element<'a, M> {
    row![
        text(title).size(22).color(tokens().ink),
        Space::with_width(Length::Fill),
        text(notice).size(13).color(tokens().warn),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
