//! A labelled on/off switch row. Stateless — the host owns the `value` and gets
//! `on_toggle` when the row is clicked. Screens write
//! `toggle("Format on save", on, Msg::Flip)` instead of restyling a checkbox.

use iced::widget::{button, container, row, text, Space};
use iced::{Border, Element, Length};

use crate::theme::tokens;

/// A full-width row: `label` on the left, a switch on the right reflecting
/// `value`. Clicking anywhere on the row emits `on_toggle`.
pub fn toggle<'a, M: Clone + 'a>(label: &str, value: bool, on_toggle: M) -> Element<'a, M> {
    let p = tokens();

    // The switch: a rounded track with a knob pushed to one side by a flexible
    // spacer (no pixel math — alignment does the work).
    let knob = container(Space::new().width(Length::Fixed(14.0)).height(Length::Fixed(14.0))).style(move |_| {
        container::Style {
            background: Some(p.bg.into()),
            border: Border {
                radius: 7.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });
    let track_inner = if value {
        row![Space::new().width(Length::Fill), knob]
    } else {
        row![knob, Space::new().width(Length::Fill)]
    };
    let track = container(track_inner)
        .width(Length::Fixed(38.0))
        .height(Length::Fixed(20.0))
        .padding(3)
        .style(move |_| container::Style {
            background: Some(if value { p.accent } else { p.hairline }.into()),
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    button(
        row![
            text(label.to_string()).size(13).color(p.ink),
            Space::new().width(Length::Fill),
            track,
        ]
        .align_y(iced::Alignment::Center),
    )
    .on_press(on_toggle)
    .style(button::text)
    .padding([6, 4])
    .width(Length::Fill)
    .into()
}
