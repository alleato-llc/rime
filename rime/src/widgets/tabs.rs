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
    /// Fill the strip's background with the `surface` token (`true`, a raised bar — the
    /// window-level look) or leave it transparent (`false`, so the strip blends into
    /// whatever it sits on, e.g. a tab strip nested inside a bordered pane).
    pub filled: bool,
}

impl Default for TabBarStyle {
    fn default() -> Self {
        Self {
            highlight_active: true,
            text_size: 13.0,
            filled: true,
        }
    }
}

/// Tracks a drag-to-reorder gesture across a tab strip, browser-style: press a tab to
/// arm the drag, and as the pointer crosses other tabs the dragged tab follows. The
/// [`tabs`] widget is stateless and only reports gestures, so the *host* owns one of
/// these, arms it from the strip's `on_activate` (which fires on mouse-down), feeds it
/// the hovered index from `on_hover`, and applies each returned move to its own
/// collection with [`reorder_slice`]. Every strip that reorders — document tabs,
/// terminal tabs, nested pane tabs — drives the same three calls instead of
/// re-deriving the anchor bookkeeping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reorder {
    anchor: Option<usize>,
}

impl Reorder {
    /// Arm a drag on the pressed tab `index`.
    pub fn begin(&mut self, index: usize) {
        self.anchor = Some(index);
    }

    /// Whether a drag is currently armed.
    pub fn is_active(&self) -> bool {
        self.anchor.is_some()
    }

    /// The tab index the drag currently sits on, if armed.
    pub fn anchor(&self) -> Option<usize> {
        self.anchor
    }

    /// While dragging, the pointer entered tab `target`. Returns the `(from, to)` move
    /// the host should apply (and advances the anchor, so successive crossings keep
    /// moving the tab); `None` if no drag is armed or the target is already the anchor.
    pub fn drag_to(&mut self, target: usize) -> Option<(usize, usize)> {
        let from = self.anchor?;
        if from == target {
            return None;
        }
        self.anchor = Some(target);
        Some((from, target))
    }

    /// Clear the drag on pointer release, returning the final anchor (where the tab
    /// ended up) if a drag was armed.
    pub fn end(&mut self) -> Option<usize> {
        self.anchor.take()
    }
}

/// Move the element at `from` to `to`, shifting the elements in between — the list
/// mutation behind a browser-style tab reorder. Alloc-free (a subrange rotate). No-op
/// if either index is out of range or they are equal.
pub fn reorder_slice<T>(items: &mut [T], from: usize, to: usize) {
    let n = items.len();
    if from >= n || to >= n || from == to {
        return;
    }
    if from < to {
        items[from..=to].rotate_left(1);
    } else {
        items[to..=from].rotate_right(1);
    }
}

/// A strip of `tabs` with the `active` one highlighted. `on_activate(i)` fires
/// when a tab body is *pressed* (on mouse-down, so a host can begin a drag — tear-off
/// or reorder — from the press); `on_close(i)` when its close button is clicked. The
/// close button is shown only on the `hovered` tab; `on_hover(Some(i))` fires when
/// the pointer enters tab `i` and `on_hover(None)` when it leaves the strip.
/// `on_background_press` fires on a **double-click** of the empty area past the last
/// tab — the standard "new tab" gesture, so a stray single click never spawns one.
/// `on_right_press(i)`
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

        // The tab body is a plain (non-interactive) container so it does *not* capture
        // the press — activation is driven by the wrapping `mouse_area`'s `on_press`
        // below, which fires on mouse-*down*. That early arm is what lets a host begin a
        // drag (tear-off / reorder) on press; an iced `button` would only report on
        // mouse-*up*, by which point the drag gesture is already over.
        let body = container(
            text(label)
                .size(style.text_size)
                .color(color)
                .font(Font::MONOSPACE),
        )
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
                .on_press(on_activate(i))
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
    // tab (when they don't fill it) bubbles to this mouse_area. A *double*-click there
    // is the host's cue to open a new tab — a single stray click does nothing — while
    // tab bodies capture their own presses.
    let filled = style.filled;
    container(
        mouse_area(scroller)
            .on_double_click(on_background_press)
            .on_exit(on_hover(None)),
    )
    .width(Length::Fill)
    .style(move |_| {
        if filled {
            container::background(p.surface)
        } else {
            container::Style::default()
        }
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::{reorder_slice, Reorder};

    #[test]
    fn reorder_slice_moves_right_and_left() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        reorder_slice(&mut v, 0, 2); // a past b,c
        assert_eq!(v, ['b', 'c', 'a', 'd']);
        reorder_slice(&mut v, 3, 1); // d back between b and c
        assert_eq!(v, ['b', 'd', 'c', 'a']);
    }

    #[test]
    fn reorder_slice_ignores_bad_indices() {
        let mut v = vec![1, 2, 3];
        reorder_slice(&mut v, 1, 1); // same index
        reorder_slice(&mut v, 0, 9); // out of range
        reorder_slice(&mut v, 9, 0);
        assert_eq!(v, [1, 2, 3]);
    }

    #[test]
    fn reorder_tracker_follows_the_drag() {
        let mut r = Reorder::default();
        assert!(!r.is_active());
        r.begin(0);
        assert!(r.is_active());
        assert_eq!(r.drag_to(0), None); // already on the anchor
        assert_eq!(r.drag_to(2), Some((0, 2))); // 0 -> 2, anchor advances
        assert_eq!(r.anchor(), Some(2));
        assert_eq!(r.drag_to(1), Some((2, 1))); // keeps following
        assert_eq!(r.end(), Some(1));
        assert!(!r.is_active());
        assert_eq!(r.drag_to(0), None); // disarmed
    }
}
