//! A single-color editor: a label, a live swatch, the `#hex` readout, and R/G/B/A
//! sliders — the building block of the theme editor. Stateless: it renders `color`
//! and emits `on_change(new_color)` when a slider moves; the host owns the value.
//! (Editing is via the sliders + swatch; the hex is a live readout — keeping it
//! stateless avoids threading a per-color edit buffer through the host.)

use iced::widget::{container, row, slider, text, Space};
use iced::{Border, Color, Element, Font, Length};

use crate::theme::tokens;

/// A color row for `label`, editing `color`. Moving any channel slider emits
/// `on_change` with the updated color.
pub fn color_field<'a, M, F>(label: &'a str, color: Color, on_change: F) -> Element<'a, M>
where
    M: Clone + 'a,
    F: Fn(Color) -> M + Clone + 'a,
{
    let p = tokens();

    let swatch = container(Space::new().width(Length::Fixed(22.0)).height(Length::Fixed(22.0))).style(move |_| {
        container::Style::default().background(color).border(Border {
            color: p.hairline,
            width: 1.0,
            radius: 4.0.into(),
        })
    });

    let channel =
        |value: f32, make: fn(Color, f32) -> Color, glyph: &'static str| -> Element<'a, M> {
            let oc = on_change.clone();
            row![
                text(glyph).size(11).color(p.muted),
                slider(0.0..=255.0, value, move |v| oc(make(color, v))).step(1.0),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center)
            // Fill so all four channels share the row width and never overflow the
            // panel (which clipped B/A when they were fixed-width).
            .width(Length::Fill)
            .into()
        };

    row![
        text(label).size(13).color(p.ink).width(Length::Fixed(150.0)),
        swatch,
        text(hex_of(color))
            .size(12)
            .font(Font::MONOSPACE)
            .color(p.muted)
            .width(Length::Fixed(86.0)),
        channel(color.r * 255.0, |c, v| Color { r: v / 255.0, ..c }, "R"),
        channel(color.g * 255.0, |c, v| Color { g: v / 255.0, ..c }, "G"),
        channel(color.b * 255.0, |c, v| Color { b: v / 255.0, ..c }, "B"),
        channel(color.a * 255.0, |c, v| Color { a: v / 255.0, ..c }, "A"),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_y(iced::Alignment::Center)
    .into()
}

/// `#rrggbb` (opaque) or `#rrggbbaa` (with alpha).
fn hex_of(c: Color) -> String {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let (r, g, b, a) = (q(c.r), q(c.g), q(c.b), q(c.a));
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}
