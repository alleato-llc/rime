//! A keyboard-shortcut reference row — a fixed-width monospace chord cell beside a
//! muted description. A vertical stack of these (grouped under [`section`] /
//! [`caption`](crate::widgets::caption) headings) is a shortcut reference / key map,
//! identical in every app that documents its bindings, so the chord gutter lines up.
//!
//! [`section`]: crate::widgets::section

use iced::widget::{container, row, text};
use iced::{Element, Font, Length};

use crate::theme::tokens;

/// Width of the chord gutter — wide enough for compound chords like
/// `right-click / ⌃-click` so descriptions line up across rows.
const CHORD_WIDTH: f32 = 160.0;

/// One reference row: `keys` in a fixed-width monospace ink cell, then a muted
/// `description`.
pub fn shortcut_row<'a, M: 'a>(keys: &str, description: &str) -> Element<'a, M> {
    let p = tokens();
    row![
        container(
            text(keys.to_string())
                .size(12)
                .font(Font::MONOSPACE)
                .color(p.ink),
        )
        .width(Length::Fixed(CHORD_WIDTH)),
        text(description.to_string()).size(12).color(p.muted),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .into()
}
