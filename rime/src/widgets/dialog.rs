//! A modal dialog: a title, a message, and a right-aligned row of action buttons,
//! centered over a dimmed backdrop. Collapses the alert/confirm shape both apps
//! were hand-rolling — the host supplies the actions (rime buttons) and the
//! `on_dismiss` message for a backdrop click.

use iced::widget::{column, row, text, Space};
use iced::{Element, Length};

use super::modal::modal;
use super::section::section;
use crate::theme::tokens;

/// Overlay a titled dialog (`title` + `message` + a right-aligned `actions` row)
/// over `base`. A backdrop click emits `on_dismiss`; the action buttons carry their
/// own messages.
pub fn dialog<'a, M: Clone + 'a>(
    base: impl Into<Element<'a, M>>,
    title: &str,
    message: &str,
    actions: Vec<Element<'a, M>>,
    on_dismiss: M,
) -> Element<'a, M> {
    let p = tokens();
    let mut action_row = row![Space::new().width(Length::Fill)].spacing(8);
    for action in actions {
        action_row = action_row.push(action);
    }
    let body = column![
        section(title),
        text(message.to_string()).size(13).color(p.ink),
        action_row,
    ]
    .spacing(16);
    modal(base, body, on_dismiss)
}
