//! Chrome for a torn-off / secondary window: a slim title strip sized to the tab
//! bar, and a full [`window_shell`] that stacks that strip over a body and a
//! [`status_bar`](crate::widgets::status_bar) footer. Reach for these when a window
//! other than the main one needs to feel like the same app — a detached editor or
//! terminal tab, a popped-out panel — so the header lines up with the main window's
//! tab strip and the footer matches every other status bar.

use iced::widget::{column, container, text, Row, Space};
use iced::{Element, Length};

use crate::theme::tokens;
use crate::widgets::{status_bar, TAB_BAR_HEIGHT};

/// A surface band at [`TAB_BAR_HEIGHT`]: an inked `label` on the left, then the
/// `controls` (e.g. `button::ghost(…)`) pushed to the right edge. The fixed height
/// makes it line up with the main window's tab strip.
pub fn title_strip<'a, M: 'a>(label: &str, controls: Vec<Element<'a, M>>) -> Element<'a, M> {
    let p = tokens();
    let mut bar = Row::new()
        .align_y(iced::Alignment::Center)
        .push(text(label.to_string()).size(13).color(p.ink))
        .push(Space::new().width(Length::Fill));
    for c in controls {
        bar = bar.push(c);
    }
    container(bar)
        .width(Length::Fill)
        .height(Length::Fixed(TAB_BAR_HEIGHT))
        .padding([0, 12])
        .style(move |_| container::background(p.surface))
        .into()
}

/// A whole torn-off-window frame: a [`title_strip`] header (`title` + `controls`), a
/// `body` that fills the middle, and a [`status_bar`](crate::widgets::status_bar)
/// footer showing `status_left` / `status_right`, all on the window background. One
/// call renders the chrome of a detached / secondary window so every app's torn-off
/// windows match each other and the main window.
pub fn window_shell<'a, M: 'a>(
    title: &str,
    controls: Vec<Element<'a, M>>,
    body: impl Into<Element<'a, M>>,
    status_left: &str,
    status_right: &str,
) -> Element<'a, M> {
    let p = tokens();
    container(column![
        title_strip(title, controls),
        container(body.into())
            .width(Length::Fill)
            .height(Length::Fill),
        status_bar(status_left, status_right),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_| container::background(p.bg))
    .into()
}
