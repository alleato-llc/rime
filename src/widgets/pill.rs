//! A small rounded status chip (running / queued / done).

use iced::widget::{container, text};
use iced::{Border, Color, Element};

/// A pill-shaped chip filled with `color`, white label on top. The caller picks
/// the color (usually a [`crate::theme::Palette`] token) so the chip can mean
/// whatever the screen needs.
pub fn pill<'a, M: 'a>(label: &str, color: Color) -> Element<'a, M> {
    container(text(label.to_string()).size(12).color(Color::WHITE))
        .padding([3, 10])
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: 999.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}
