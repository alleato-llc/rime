//! A big-number readout with a caption — a screen's vitals.

use iced::widget::{column, text};
use iced::Element;

use crate::theme::tokens;

/// A large `value` over a small muted `label`.
///
/// # Compared to raw iced
///
/// ```ignore
/// // raw iced
/// column![
///     text(value).size(22).color(ink),
///     text(label).size(12).color(muted),
/// ].spacing(2)
///
/// // rime
/// stat("p50", "12 ms".to_string())
/// ```
pub fn stat<'a, M: 'a>(label: &str, value: String) -> Element<'a, M> {
    column![
        text(value).size(22).color(tokens().ink),
        text(label.to_string()).size(12).color(tokens().muted)
    ]
    .spacing(2)
    .into()
}
