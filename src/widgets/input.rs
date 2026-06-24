//! The one text-entry primitive: a rounded, padded text input.

use iced::widget::{text_input, TextInput};

use crate::theme::input_style;

/// A rounded, padded text input bound to `value`, emitting `on_input(text)` on
/// every edit.
pub fn text_field<'a, M: Clone + 'a>(
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
) -> TextInput<'a, M> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([8, 10])
        .style(input_style)
}
