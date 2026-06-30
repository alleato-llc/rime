//! A labeled horizontal slider with a live readout — the "how much" control
//! (intensities, opacities, volumes). Stateless: it renders `value` within
//! `range` and emits `on_change(new)` as the handle moves; the host owns the
//! value. The sibling of [`stepper`](super::stepper) for continuous amounts.

use std::ops::RangeInclusive;

use iced::widget::{row, slider as islider, text};
use iced::{Alignment, Element, Length};

use crate::theme::tokens;

/// A `label` + slider over `range` at `value`, with a right-aligned `readout`
/// (e.g. `"60%"`). Dragging the handle emits `on_change` with the new value.
pub fn slider<'a, M, F>(
    label: &'a str,
    range: RangeInclusive<f32>,
    value: f32,
    readout: impl Into<String>,
    on_change: F,
) -> Element<'a, M>
where
    M: Clone + 'a,
    F: Fn(f32) -> M + 'a,
{
    let p = tokens();
    // A continuous control: default to ~1% granularity across the range. (iced's
    // slider defaults to a step of 1.0, which would snap a 0..=1 range to just its
    // endpoints — only the readout would ever read 0% or 100%.)
    let step = ((range.end() - range.start()) / 100.0).max(f32::MIN_POSITIVE);
    row![
        text(label)
            .size(13)
            .color(p.ink)
            .width(Length::Fixed(170.0)),
        islider(range, value, on_change).step(step),
        text(readout.into())
            .size(12)
            .color(p.muted)
            .width(Length::Fixed(48.0)),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}
