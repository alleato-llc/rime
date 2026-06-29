//! A status bar (footer): a thin surface strip with a `left` segment and a
//! right-aligned `right` segment. Stateless — the host supplies the two ends as
//! plain text and the bar owns the typography (size + muted color), so the footer
//! looks identical in every app that uses it.

use iced::widget::{container, row, text, Space};
use iced::{Border, Element, Length};

use crate::theme::tokens;

/// The footer text size — owned here so the footer typography is uniform across
/// apps (hosts pass strings, not pre-styled text).
pub const TEXT_SIZE: f32 = 13.0;

/// A footer row: `left` on the left, `right` pushed to the right, on a surface
/// background with a hairline separating it from the content above. Both ends are
/// rendered in the bar's own muted type, so every app's footer matches.
pub fn status_bar<'a, M: 'a>(left: &str, right: &str) -> Element<'a, M> {
    let p = tokens();
    container(
        row![
            text(left.to_string()).size(TEXT_SIZE).color(p.muted),
            Space::new().width(Length::Fill),
            text(right.to_string()).size(TEXT_SIZE).color(p.muted),
        ]
        .align_y(iced::Alignment::Center),
    )
    .padding([7, 14])
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(p.surface.into()),
        border: Border {
            color: p.hairline,
            width: 1.0,
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
