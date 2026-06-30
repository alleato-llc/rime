//! A horizontal tab strip: one entry per open document, the active one inked with
//! the accent, each with a close affordance that appears on hover. Stateless — the
//! host owns which tab is active, which is hovered, and what each tab is named; the
//! strip just renders and reports clicks/hovers. Screens write
//! `tabs(labels, active, hovered, on_activate, on_close, on_hover)` instead of
//! re-deriving the row, the dirty dot, and the close button at every call site.

use iced::widget::{button, container, mouse_area, scrollable, text, Row, Space};
use iced::{Element, Font, Length};

use crate::theme::tokens;

/// The strip's rendered height in logical pixels — hosts that lay content beside
/// the tabs (e.g. a sidebar) can pad by this to align their content tops.
pub const TAB_BAR_HEIGHT: f32 = 33.0;

/// One tab's display state: its `label` and whether the document is `dirty`
/// (unsaved). A dirty tab is marked with a leading dot.
#[derive(Debug, Clone)]
pub struct Tab {
    /// The tab's visible name (e.g. a file name or "untitled").
    pub label: String,
    /// Whether the underlying document has unsaved changes.
    pub dirty: bool,
}

impl Tab {
    /// A clean tab with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            dirty: false,
        }
    }

    /// Mark this tab dirty (unsaved changes).
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }
}

/// Host-tunable look for the tab strip. Everything else stays palette-driven; these
/// are the knobs an app may want to vary (e.g. expose as a setting). `default()` is the
/// standard look: the active tab inked with the accent, 13px labels.
#[derive(Debug, Clone, Copy)]
pub struct TabBarStyle {
    /// Ink the active tab with the accent (`true`) or with normal ink (`false`, a
    /// subtler emphasis — it still reads as active versus the muted inactive tabs).
    pub highlight_active: bool,
    /// Tab label text size, in logical pixels.
    pub text_size: f32,
}

impl Default for TabBarStyle {
    fn default() -> Self {
        Self {
            highlight_active: true,
            text_size: 13.0,
        }
    }
}

/// A strip of `tabs` with the `active` one highlighted. `on_activate(i)` fires
/// when a tab body is clicked; `on_close(i)` when its close button is clicked. The
/// close button is shown only on the `hovered` tab; `on_hover(Some(i))` fires when
/// the pointer enters tab `i` and `on_hover(None)` when it leaves the strip.
/// `on_background_press` fires when the empty area past the last tab is clicked
/// (hosts typically treat a double-click there as "new tab"). `on_right_press(i)`
/// fires on a secondary click on tab `i` (hosts open a context menu).
#[allow(clippy::too_many_arguments)]
pub fn tabs<'a, M: Clone + 'a>(
    tabs: Vec<Tab>,
    active: usize,
    hovered: Option<usize>,
    on_activate: impl Fn(usize) -> M + 'a,
    on_close: impl Fn(usize) -> M + 'a,
    on_hover: impl Fn(Option<usize>) -> M + 'a,
    on_right_press: impl Fn(usize) -> M + 'a,
    on_background_press: M,
    style: TabBarStyle,
) -> Element<'a, M> {
    let p = tokens();

    let mut strip = Row::new().spacing(2).padding(4);
    for (i, tab) in tabs.into_iter().enumerate() {
        let label = if tab.dirty {
            format!("● {}", tab.label)
        } else {
            tab.label
        };
        let color = if i == active {
            if style.highlight_active {
                p.accent
            } else {
                p.ink
            }
        } else {
            p.muted
        };

        let body = button(
            text(label)
                .size(style.text_size)
                .color(color)
                .font(Font::MONOSPACE),
        )
        .on_press(on_activate(i))
        .style(button::text)
        .padding([4, 8]);
        // The close affordance only appears on the hovered tab; otherwise it's a
        // fixed-width spacer so tabs don't jump as the pointer moves across them.
        let close: Element<'a, M> = if hovered == Some(i) {
            button(text("×").size(14).color(p.muted).font(Font::MONOSPACE))
                .on_press(on_close(i))
                .style(button::text)
                .padding([4, 6])
                .into()
        } else {
            Space::new().width(Length::Fixed(20.0)).into()
        };
        // Wrap each tab so entering it reports the hover index, and a right-press
        // reports the tab index (the host anchors a context menu there).
        strip = strip.push(
            mouse_area(Row::new().push(body).push(close))
                .on_enter(on_hover(Some(i)))
                .on_right_press(on_right_press(i)),
        );
    }

    // The row keeps its *natural* width (tabs at full size, never compressed), and a
    // horizontal scrollable lets it overflow by scrolling rather than squeezing the
    // last tab until its label wraps. A thin scrollbar keeps the bar slim.
    let strip = strip.align_y(iced::Alignment::Center);
    let scroller =
        scrollable(strip)
            .width(Length::Fill)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            ));

    // The scrollable fills the bar width, so a press in the empty area past the last
    // tab (when they don't fill it) bubbles to this mouse_area — the host's cue to
    // open a new tab — while tab buttons capture their own clicks.
    container(
        mouse_area(scroller)
            .on_press(on_background_press)
            .on_exit(on_hover(None)),
    )
    .width(Length::Fill)
    .style(move |_| container::background(p.surface))
    .into()
}
