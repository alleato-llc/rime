//! A small `− value +` numeric stepper row. Stateless — the host owns the value
//! and gets `on_dec` / `on_inc` when the buttons are pressed.

use iced::widget::{button, container, row, text};
use iced::{Border, Element, Length};

use crate::theme::tokens;

/// A `label … [−] value [+]` row. `value` is pre-formatted by the host (e.g.
/// `"14"` or `"4 spaces"`).
pub fn stepper<'a, M: Clone + 'a>(
    label: &str,
    value: impl Into<String>,
    on_dec: M,
    on_inc: M,
) -> Element<'a, M> {
    let p = tokens();
    let btn = |glyph: &str, msg: M| {
        button(text(glyph.to_string()).size(15).color(p.ink))
            .on_press(msg)
            .padding([1, 9])
            .style(move |_, _| button::Style {
                background: Some(p.surface.into()),
                text_color: p.ink,
                border: Border {
                    color: p.hairline,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
    };
    row![
        text(label.to_string()).size(13).color(p.ink),
        iced::widget::Space::new().width(Length::Fill),
        btn("−", on_dec),
        container(text(value.into()).size(13).color(p.ink)).padding([0, 8]),
        btn("+", on_inc),
    ]
    .align_y(iced::Alignment::Center)
    .into()
}
