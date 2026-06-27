//! A dropdown (single-select) in the house style: a rounded, padded `pick_list`
//! using [`crate::theme::pick_style`]. Screens write
//! `select(options, selected, Message::Pick)` instead of restyling a pick list.

use std::borrow::Borrow;

use iced::widget::{pick_list, PickList};
use iced::Theme;

use crate::theme::pick_style;

/// A styled dropdown over `options`, showing `selected`, emitting
/// `on_select(choice)` when the selection changes.
pub fn select<'a, T, L, V, M>(
    options: L,
    selected: Option<V>,
    on_select: impl Fn(T) -> M + 'a,
) -> PickList<'a, T, L, V, M, Theme>
where
    T: ToString + PartialEq + Clone + 'a,
    L: Borrow<[T]> + 'a,
    V: Borrow<T> + 'a,
    M: Clone,
{
    pick_list(options, selected, on_select)
        .padding([6, 10])
        .style(pick_style)
}
