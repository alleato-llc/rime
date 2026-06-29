//! A modal overlay: `content` on a surface panel, centered over a dimmed backdrop
//! covering `base`. Clicking the backdrop emits `on_dismiss`; clicks on the panel
//! are absorbed (so they don't dismiss it).

use iced::widget::{center, container, mouse_area, opaque, stack};
use iced::{Border, Color, Element, Length};

use crate::theme::tokens;

/// Overlay `content` (wrapped in a centered surface panel) over `base`. A click on
/// the dimmed backdrop emits `on_dismiss`.
pub fn modal<'a, M: Clone + 'a>(
    base: impl Into<Element<'a, M>>,
    content: impl Into<Element<'a, M>>,
    on_dismiss: M,
) -> Element<'a, M> {
    // Capture the palette now so the draw-time style closures don't read the
    // thread-local after the scope has dropped.
    let p = tokens();

    let panel = container(content)
        .padding(20)
        .max_width(420.0)
        .style(move |_theme| container::Style {
            background: Some(p.surface.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..container::Style::default()
        });

    let backdrop = center(opaque(panel))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(
                Color {
                    a: 0.45,
                    ..Color::BLACK
                }
                .into(),
            ),
            ..container::Style::default()
        });

    stack![
        base.into(),
        opaque(mouse_area(backdrop).on_press(on_dismiss))
    ]
    .into()
}
