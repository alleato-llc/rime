//! A text field with a suggestion popup — the input primitive for anything
//! that completes as you type (a formula bar, a command palette, a search
//! box). Reach for it instead of a bare [`text_field`](crate::widgets::text_field)
//! when the host can offer candidates for the current input.
//!
//! **Stateless, and it does NOT filter.** The caller computes the suggestions
//! however it likes (an engine's completion pass, a fuzzy matcher, a history
//! scan) and passes the finished list plus which row is `highlighted`; the
//! widget only draws them and reports a click via `on_accept(index)`. This is
//! deliberately unlike iced's `combo_box`, which owns a fixed option set and
//! substring-filters it internally — here the candidate logic is the host's.
//!
//! Keyboard is the host's too: a single-line text input ignores ↑/↓, so the
//! host handles them (move the highlight when the popup is open, walk input
//! history when it's closed — the "dual role") and re-renders with a new
//! `highlighted`. The popup shows only when `suggestions` is non-empty, so the
//! host opens/closes it by passing candidates or an empty slice.
//!
//! The popup expands **below** the field. A host with a bottom-anchored input
//! bar (suggestions above) composes the two halves itself.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Border, Color, Element, Length};

use crate::theme::{input_style, tokens};

const POPUP_MAX_ROWS: usize = 8;
const ROW_RADIUS: f32 = 4.0;

/// One suggestion row: the completion `text` plus an optional dim `hint` (a
/// function signature, a type, a shortcut) shown right-aligned. Owns its
/// strings, since suggestions are usually *computed* per keystroke — a
/// borrowed slice couldn't outlive the `view()` that builds them.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub hint: Option<String>,
}

impl Suggestion {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hint: None,
        }
    }

    /// A suggestion with a right-aligned dim hint.
    pub fn with_hint(text: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            hint: Some(hint.into()),
        }
    }
}

impl From<&str> for Suggestion {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Suggestion {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

/// A text input bound to `value` (emitting `on_input`) with a popup of
/// caller-provided `suggestions`. `highlighted` tints one row (the host moves
/// it with ↑/↓); clicking a row emits `on_accept(index)`. An empty
/// `suggestions` slice hides the popup.
pub fn autocomplete_field<'a, M: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
    suggestions: Vec<Suggestion>,
    highlighted: Option<usize>,
    on_input: impl Fn(String) -> M + 'a,
    on_accept: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let field = text_input(placeholder, value)
        .on_input(on_input)
        .padding([8, 10])
        .style(input_style);

    match suggestion_list(suggestions, highlighted, on_accept) {
        Some(popup) => column![field, popup].spacing(4).into(),
        None => field.into(),
    }
}

/// Just the suggestion popup — the caller-provided `suggestions` as a bordered
/// list, one row `highlighted`, each row clickable (`on_accept(index)`).
/// Returns `None` when `suggestions` is empty (nothing to draw).
///
/// [`autocomplete_field`] stacks this *below* its input; a host with a
/// bottom-anchored input bar composes it *above* instead — put this in a
/// `column!` before the field. That placement freedom is exactly why the popup
/// is exposed on its own.
pub fn suggestion_list<'a, M: Clone + 'a>(
    suggestions: Vec<Suggestion>,
    highlighted: Option<usize>,
    on_accept: impl Fn(usize) -> M + 'a,
) -> Option<Element<'a, M>> {
    if suggestions.is_empty() {
        return None;
    }
    let palette = tokens();

    let mut list = column![].spacing(1).padding(4);
    for (index, suggestion) in suggestions.into_iter().take(POPUP_MAX_ROWS).enumerate() {
        let is_highlighted = highlighted == Some(index);

        let mut content = row![text(suggestion.text).size(13)].spacing(8);
        if let Some(hint) = suggestion.hint {
            let hint_color = if is_highlighted {
                palette.bg
            } else {
                palette.muted
            };
            content = content
                .push(Space::new().width(Length::Fill))
                .push(text(hint).size(12).color(hint_color));
        }

        list = list.push(
            button(content.width(Length::Fill))
                .on_press(on_accept(index))
                .width(Length::Fill)
                .padding([4, 8])
                .style(move |_theme, status| {
                    let hovered = matches!(status, button::Status::Hovered);
                    let background = if is_highlighted {
                        Some(palette.accent.into())
                    } else if hovered {
                        Some(palette.surface.into())
                    } else {
                        Some(Color::TRANSPARENT.into())
                    };
                    button::Style {
                        background,
                        text_color: if is_highlighted {
                            palette.bg
                        } else {
                            palette.ink
                        },
                        border: Border {
                            radius: ROW_RADIUS.into(),
                            ..Border::default()
                        },
                        ..button::Style::default()
                    }
                }),
        );
    }

    Some(
        container(list)
            .width(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(palette.surface.into()),
                border: Border {
                    color: palette.hairline,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..container::Style::default()
            })
            .into(),
    )
}
