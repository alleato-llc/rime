//! A right-click context menu: a free-positioned popup of action `Item`s over a
//! full-window backdrop. Stateless — the host owns *whether* a menu is open, *where*
//! (a `Point`), and on *what*; it passes the items and gets `on_dismiss` when the
//! backdrop is clicked or the menu should close.
//!
//! Shares the [`Item`](crate::widgets::Menu) vocabulary and panel look with
//! [`menu_bar`](crate::widgets::menu_bar) — a context menu is the same dropdown,
//! anchored to a cursor instead of a bar.

use iced::widget::{column, mouse_area, opaque, row, stack, Space};
use iced::{Element, Length, Point};

use super::menu::{render_panel, Item};

/// Overlay a context menu of `items` at `at` (window coordinates) over `base`. A
/// click on the surrounding backdrop emits `on_dismiss`. Selecting an item emits
/// that item's own message; the host clears its open state in response.
///
/// `at` should already be clamped by the host to keep the panel on-screen (the
/// component can't see the viewport size at build time).
pub fn context_menu<'a, M: Clone + 'a>(
    base: impl Into<Element<'a, M>>,
    items: &[Item<M>],
    at: Point,
    on_dismiss: M,
) -> Element<'a, M> {
    // The panel, pushed to (at.x, at.y) with leading spacers.
    let anchored = column![
        Space::new().height(Length::Fixed(at.y.max(0.0))),
        row![
            Space::new().width(Length::Fixed(at.x.max(0.0))),
            render_panel(items),
        ],
    ];

    stack![
        base.into(),
        // Transparent full-window backdrop: an outside click dismisses.
        opaque(
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(on_dismiss)
        ),
        // The panel sits above the backdrop so its buttons stay clickable.
        anchored,
    ]
    .into()
}
