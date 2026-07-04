//! A dropdown menu bar — `File`, `Edit`, … top-level titles, each opening a panel
//! of action items (with optional shortcut hints) and separators.
//!
//! Stateless: the host owns *which* menu is open (an `Option<usize>`) and toggles
//! it via `on_toggle`. The component renders a full-window layer — a thin bar at
//! the top whose empty remainder passes clicks through to whatever is beneath it,
//! plus (when a menu is open) a backdrop that closes the menu on an outside click
//! and the open menu's dropdown panel anchored beneath its title.
//!
//! ```ignore
//! // host: Stack the menu bar over the body (body padded down by `BAR_HEIGHT`).
//! stack![body, menu_bar(menus, self.menu_open, Message::ToggleMenu)]
//! ```
//! Selecting an item emits that item's own message; the host should clear its
//! open-menu state when it handles one (the backdrop only closes on *outside*
//! clicks).

use iced::widget::{button, column, container, mouse_area, opaque, row, stack, text, Space};
use iced::{Border, Element, Length};

use crate::theme::tokens;

/// The bar's height in logical pixels — the host pads its body down by this so the
/// bar doesn't overlap content.
pub const BAR_HEIGHT: f32 = 34.0;

/// Top-level menus are laid out left-to-right at this fixed width, so each
/// dropdown can be anchored under its title without measuring.
const MENU_WIDTH: f32 = 56.0;

/// The fixed width of a dropdown panel — also used to anchor submenu flyouts to
/// the right of their parent.
const PANEL_WIDTH: f32 = 200.0;

/// Approximate heights used to anchor a submenu flyout to its parent row without
/// measuring (an action row vs. a separator).
const ROW_H: f32 = 27.0;
const SEP_H: f32 = 3.0;
const PANEL_PAD: f32 = 4.0;

/// One entry inside a menu's dropdown panel.
pub enum Item<M> {
    /// A clickable action: a `label`, an optional right-aligned `shortcut` hint,
    /// and the message it emits.
    Action {
        /// The action's label.
        label: String,
        /// An optional shortcut hint shown muted on the right (e.g. "⌘S").
        shortcut: Option<String>,
        /// The message emitted when the item is selected.
        on_select: M,
    },
    /// A nested submenu that opens as a flyout to the right on hover. `expanded`
    /// (host-owned) controls whether the flyout is shown; `on_hover` is emitted
    /// when the pointer enters the row so the host can mark it expanded.
    Submenu {
        /// The row label (a `▸` is appended to signal expandability).
        label: String,
        /// The flyout's entries.
        items: Vec<Item<M>>,
        /// Whether the flyout is currently shown (host state).
        expanded: bool,
        /// Emitted on pointer-enter so the host can expand this submenu.
        on_hover: M,
    },
    /// A horizontal divider between groups of actions.
    Separator,
}

impl<M> Item<M> {
    /// An action item with no shortcut hint.
    pub fn action(label: impl Into<String>, on_select: M) -> Self {
        Item::Action {
            label: label.into(),
            shortcut: None,
            on_select,
        }
    }

    /// An action item with a muted shortcut hint on the right.
    pub fn shortcut(label: impl Into<String>, shortcut: impl Into<String>, on_select: M) -> Self {
        Item::Action {
            label: label.into(),
            shortcut: Some(shortcut.into()),
            on_select,
        }
    }

    /// A submenu that flies out to the right when `expanded`; `on_hover` expands it.
    pub fn submenu(
        label: impl Into<String>,
        items: Vec<Item<M>>,
        expanded: bool,
        on_hover: M,
    ) -> Self {
        Item::Submenu {
            label: label.into(),
            items,
            expanded,
            on_hover,
        }
    }

    /// A divider between groups.
    pub fn separator() -> Self {
        Item::Separator
    }

    /// The row's approximate laid-out height (for anchoring a flyout).
    fn height(&self) -> f32 {
        match self {
            Item::Separator => SEP_H,
            _ => ROW_H,
        }
    }
}

/// One top-level menu: a `title` and the `items` in its dropdown.
pub struct Menu<M> {
    /// The bar title (e.g. "File").
    pub title: String,
    /// The dropdown's entries, top to bottom.
    pub items: Vec<Item<M>>,
}

impl<M> Menu<M> {
    /// A menu with the given title and items.
    pub fn new(title: impl Into<String>, items: Vec<Item<M>>) -> Self {
        Self {
            title: title.into(),
            items,
        }
    }
}

/// Render the menu bar as a full-window layer. `open` is the index of the
/// currently-open menu (if any); `on_toggle(Some(i))` should open menu `i`,
/// `on_toggle(None)` should close.
pub fn menu_bar<'a, M: Clone + 'a>(
    menus: Vec<Menu<M>>,
    open: Option<usize>,
    on_toggle: impl Fn(Option<usize>) -> M + 'a,
) -> Element<'a, M> {
    menu_bar_with_trailing(menus, open, on_toggle, None)
}

/// As [`menu_bar`], plus a `trailing` element pinned to the right end of the bar
/// (e.g. a sidebar-toggle icon, the way a macOS title bar carries a toolbar item
/// on the right). It sits in the opaque bar strip, vertically centered.
pub fn menu_bar_with_trailing<'a, M: Clone + 'a>(
    menus: Vec<Menu<M>>,
    open: Option<usize>,
    on_toggle: impl Fn(Option<usize>) -> M + 'a,
    trailing: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let p = tokens();

    // The bar: one fixed-width title button per menu. Clicking the open menu's
    // title closes it; clicking another switches.
    let mut titles = row![].spacing(0).align_y(iced::Alignment::Center);
    let open_menu = open.and_then(|i| menus.get(i));
    let dropdown = open_menu.map(|m| render_panel(&m.items));
    // If the open menu has an expanded submenu, build its flyout + the y-offset to
    // anchor it next to its parent row.
    let flyout = open_menu.and_then(|m| {
        let mut y = BAR_HEIGHT + PANEL_PAD;
        for item in &m.items {
            if let Item::Submenu {
                items,
                expanded: true,
                ..
            } = item
            {
                return Some((render_panel(items), y));
            }
            y += item.height();
        }
        None
    });
    for (i, menu) in menus.iter().enumerate() {
        let is_open = open == Some(i);
        let next = if is_open { None } else { Some(i) };
        let color = if is_open { p.ink } else { p.muted };
        titles = titles.push(
            button(
                container(text(menu.title.clone()).size(14).color(color))
                    .center_x(Length::Fixed(MENU_WIDTH)),
            )
            .on_press(on_toggle(next))
            .style(button::text)
            .padding([8, 0]),
        );
    }

    // A trailing item (a sidebar toggle, …) pins to the right of the bar.
    if let Some(trailing) = trailing {
        titles = titles.push(Space::new().width(Length::Fill));
        titles = titles.push(container(trailing).padding([0, 8]));
    }

    let bar = container(titles)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .style(move |_| container::Style {
            background: Some(p.surface.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

    // The bar sits at the top; the remainder is transparent and click-through.
    let bar_layer = column![bar, Space::new().height(Length::Fill)];

    let Some(idx) = open else {
        return bar_layer.into();
    };
    let Some(panel) = dropdown else {
        return bar_layer.into();
    };

    // A transparent backdrop below the bar that closes the menu on an outside
    // click, and the dropdown panel anchored under the open title — layered above
    // the backdrop but below the bar (so bar titles stay clickable to switch).
    let backdrop = column![
        Space::new().height(Length::Fixed(BAR_HEIGHT)),
        opaque(
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(on_toggle(None))
        ),
    ];
    let base_x = idx as f32 * MENU_WIDTH;
    let anchored = column![
        Space::new().height(Length::Fixed(BAR_HEIGHT)),
        row![Space::new().width(Length::Fixed(base_x)), panel],
    ];

    let mut layers = stack![backdrop, anchored];
    if let Some((flyout, y)) = flyout {
        // Anchor the flyout to the right of the parent panel, beside its row.
        layers = layers.push(column![
            Space::new().height(Length::Fixed(y)),
            row![
                Space::new().width(Length::Fixed(base_x + PANEL_WIDTH)),
                flyout,
            ],
        ]);
    }
    layers.push(bar_layer).into()
}

/// Build a dropdown panel from a slice of items (used for both a top-level menu's
/// dropdown and a submenu's flyout).
pub(crate) fn render_panel<'a, M: Clone + 'a>(items: &[Item<M>]) -> Element<'a, M> {
    let p = tokens();
    let mut col = column![].spacing(1).padding(PANEL_PAD);

    for item in items {
        match item {
            Item::Action {
                label,
                shortcut,
                on_select,
            } => {
                let mut line = row![text(label.clone()).size(13).color(p.ink)]
                    .spacing(16)
                    .padding([0, 4]);
                if let Some(sc) = shortcut {
                    line = line.push(Space::new().width(Length::Fill));
                    line = line.push(text(sc.clone()).size(12).color(p.muted));
                }
                col = col.push(
                    button(line)
                        .on_press(on_select.clone())
                        .style(button::text)
                        .padding([5, 8])
                        .width(Length::Fill),
                );
            }
            Item::Submenu {
                label, on_hover, ..
            } => {
                let line = row![
                    text(label.clone()).size(13).color(p.ink),
                    Space::new().width(Length::Fill),
                    text("▸").size(12).color(p.muted),
                ]
                .spacing(16)
                .padding([0, 4]);
                col = col.push(
                    mouse_area(
                        button(line)
                            .on_press(on_hover.clone())
                            .style(button::text)
                            .padding([5, 8])
                            .width(Length::Fill),
                    )
                    .on_enter(on_hover.clone()),
                );
            }
            Item::Separator => {
                col = col.push(
                    container(Space::new().height(Length::Fixed(1.0)))
                        .width(Length::Fill)
                        .style(move |_| container::background(p.hairline)),
                );
            }
        }
    }

    container(col)
        .width(Length::Fixed(PANEL_WIDTH))
        .style(move |_| container::Style {
            background: Some(p.surface.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}
