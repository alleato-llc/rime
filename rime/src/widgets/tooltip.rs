//! A hover tooltip in the house style — a small surface-colored bubble that
//! explains the thing under the cursor. Pairs with the status `pill`: a chip
//! says *what*, its tooltip says *what that means*.

use iced::widget::{container, text, tooltip as iced_tooltip};
use iced::{Border, Element};

use crate::theme::tokens;

/// Where the bubble appears relative to the hovered content.
pub use iced_tooltip::Position;

/// Wrap `content` so hovering it reveals `label` in a styled bubble. Colors come
/// from [`crate::theme::tokens`], so the bubble matches the active palette.
pub fn tooltip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    label: &str,
    position: Position,
) -> Element<'a, M> {
    let p = tokens();
    let bubble = container(text(label.to_string()).size(12).color(p.ink))
        .padding([7, 11])
        .max_width(320.0)
        .style(move |_theme| container::Style {
            background: Some(p.surface.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        });
    iced_tooltip(content, bubble, position).gap(6).into()
}
