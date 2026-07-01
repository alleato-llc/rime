//! An inline rename bar: a captioned, focused text field on the surface — the
//! shared shape behind "Rename tab" / "Rename terminal". Stateless: the host owns
//! the in-progress draft and open/closed state, seeds the field, commits on submit,
//! and cancels on Escape / focus loss.

use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

use crate::theme::tokens;
use crate::widgets::text_field;

/// The rename field's stable widget id, so a host can focus it the moment the bar
/// opens: `iced::widget::operation::focus(rename_field_id())`.
pub fn rename_field_id() -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::new("rime-rename-field")
}

/// An inline rename bar — a muted `caption` beside a focused [`text_field`] prefilled
/// with `value`, on the surface color. `on_change(text)` fires on every edit; `on_submit`
/// on Enter. The host renders it (typically under a tab strip) while a rename is in
/// progress and clears its own draft on Escape / focus loss.
pub fn rename_bar<'a, M: Clone + 'a>(
    caption: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_change: impl Fn(String) -> M + 'a,
    on_submit: M,
) -> Element<'a, M> {
    let p = tokens();
    let field = text_field(placeholder, value, on_change)
        .id(rename_field_id())
        .on_submit(on_submit)
        .size(13);
    container(
        row![text(caption.to_string()).size(12).color(p.muted), field]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([4, 6])
    .width(Length::Fill)
    .style(move |_| container::background(p.surface))
    .into()
}
