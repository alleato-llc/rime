//! The rounded, padded surface every screen is built from — one container
//! primitive so cards look identical everywhere.

use iced::widget::container;
use iced::{Border, Color, Element, Length, Shadow, Vector};

use crate::theme::tokens;

/// A raised surface around `content`: padded, hairline-bordered, softly shadowed.
///
/// # Compared to raw iced
///
/// The raw version is a style closure you'd copy onto every container:
///
/// ```ignore
/// // raw iced
/// container(content).padding(16).style(|_| container::Style {
///     background: Some(surface.into()),
///     border: Border { color: hairline, width: 1.0, radius: 12.0.into() },
///     shadow: Shadow { /* offset, blur, palette-aware alpha … */ },
///     ..Default::default()
/// })
///
/// // rime
/// card(content)
/// ```
pub fn card<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    // Capture the palette now, so the draw-time style closure is independent of
    // the thread-local.
    let p = tokens();
    let shadow_alpha = if p.surface.r > 0.5 { 0.06 } else { 0.20 };
    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(p.surface.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: Shadow {
                color: Color {
                    a: shadow_alpha,
                    ..Color::BLACK
                },
                offset: Vector::new(0.0, 1.0),
                blur_radius: 8.0,
            },
            ..container::Style::default()
        })
        .into()
}
