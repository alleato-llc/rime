//! A full-width dismissible banner — a transient notification strip with a message
//! and a close (✕) button on a surface background. Hosts drop it at the top of their
//! layout; the ✕ emits `on_dismiss`.

use iced::widget::{button, container, row, text, Space};
use iced::{Element, Font, Length};

use crate::theme::tokens;

/// A notification strip showing `message`, with a trailing ✕ that emits `on_dismiss`.
pub fn banner<'a, M: Clone + 'a>(message: &str, on_dismiss: M) -> Element<'a, M> {
    let p = tokens();
    let close = button(text("✕").font(Font::MONOSPACE).size(12).color(p.muted))
        .on_press(on_dismiss)
        .style(button::text);
    let content = row![
        text(message.to_string()).size(13).color(p.ink),
        Space::new().width(Length::Fill),
        close,
    ]
    .spacing(8)
    .padding([4, 8]);
    container(content)
        .width(Length::Fill)
        .style(move |_| container::background(p.surface))
        .into()
}
