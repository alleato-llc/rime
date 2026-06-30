//! A settings panel shell: a left-rail section nav beside a content pane, over a
//! dimmed backdrop. Stateless — the host owns which `active` section is selected
//! and supplies that section's `content`; the shell just frames it so every app's
//! settings share one layout. Clicking the backdrop emits `on_dismiss`.
//!
//! ```ignore
//! settings(base, &["Appearance", "Editor"], self.active, Msg::Section, body, Msg::Close)
//! ```

use iced::widget::{
    button, center, column, container, mouse_area, opaque, row, stack, text, Space,
};
use iced::{Border, Color, Element, Length};

use crate::theme::tokens;

const RAIL_WIDTH: f32 = 168.0;
// The panel grows with the (resizable) window, capped so it never sprawls on a
// large display, and inset from the edges by `MARGIN` so the backdrop shows.
const MAX_PANEL_WIDTH: f32 = 1100.0;
const MAX_PANEL_HEIGHT: f32 = 860.0;
const MARGIN: f32 = 40.0;

/// Frame `content` (the active section's body) beside a nav rail listing
/// `sections`, over `base`. `on_select(i)` switches section; `on_dismiss` closes.
pub fn settings<'a, M: Clone + 'a>(
    base: impl Into<Element<'a, M>>,
    sections: &[&str],
    active: usize,
    on_select: impl Fn(usize) -> M + 'a,
    content: impl Into<Element<'a, M>>,
    footer: Option<Element<'a, M>>,
    on_dismiss: M,
) -> Element<'a, M> {
    let p = tokens();

    // Left rail: one nav button per section, the active one inked on a raised chip.
    // An optional `footer` (e.g. "Edit settings file…") sits pinned at the bottom.
    let mut rail = column![text("Settings").size(16).color(p.ink)]
        .spacing(4)
        .padding([4, 8])
        .height(Length::Fill);
    for (i, label) in sections.iter().enumerate() {
        let is_active = i == active;
        let color = if is_active { p.ink } else { p.muted };
        rail = rail.push(
            button(text(label.to_string()).size(13).color(color))
                .on_press(on_select(i))
                .width(Length::Fill)
                .padding([6, 10])
                .style(move |_, _| button::Style {
                    background: is_active.then(|| p.bg.into()),
                    text_color: color,
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }
    if let Some(footer) = footer {
        rail = rail.push(Space::new().height(Length::Fill)).push(footer);
    }
    let rail = container(rail)
        .width(Length::Fixed(RAIL_WIDTH))
        .height(Length::Fill)
        .padding(8)
        .style(move |_| container::Style {
            background: Some(p.bg.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

    let body = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20);

    let panel = container(row![rail, body])
        .width(Length::Fill)
        .height(Length::Fill)
        .max_width(MAX_PANEL_WIDTH)
        .max_height(MAX_PANEL_HEIGHT)
        .style(move |_| container::Style {
            background: Some(p.surface.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

    // Center the (capped) panel in the window with a margin, so resizing the
    // window resizes the settings up to the cap.
    let backdrop = center(opaque(panel))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(MARGIN)
        .style(|_| container::Style {
            background: Some(
                Color {
                    a: 0.45,
                    ..Color::BLACK
                }
                .into(),
            ),
            ..Default::default()
        });

    stack![
        base.into(),
        opaque(mouse_area(backdrop).on_press(on_dismiss)),
    ]
    .into()
}
