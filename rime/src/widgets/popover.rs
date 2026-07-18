//! A floating **popover**: the non-modal cousin of [`modal`](super::modal::modal).
//! Where a modal dims the world and centers a panel, a popover stays out of the
//! way — the surface behind it keeps working — and the card is movable and
//! resizable. It carries no position state of its own: the caller owns where the
//! card sits and how big it is (so it can persist that, cascade stacks, anchor to
//! a status bar, etc.), and supplies the messages a drag emits. This module only
//! builds the view: the whole card as a move handle, invisible border strips to
//! resize, and an `opaque` seal so a press on the card does not fall through to
//! whatever is behind it.
//!
//! Two entry points:
//! - [`popover`] wraps a self-contained card (its controls live inside it) into a
//!   ready-to-place draggable, resizable layer.
//! - [`resize_edges`] adds just the resize strips, for a card that also overlays
//!   its own controls (which must sit *above* the strips) and does its own drag
//!   wrapping.

use iced::mouse::Interaction;
use iced::widget::{container, mouse_area, opaque, stack, Space};
use iced::{Element, Length};

/// Which border of a floating card a drag-resize grabs. The caller tracks the
/// drag itself (press starts it, pointer-move updates the size, release ends it);
/// [`ResizeEdge::axes`] says which dimensions a given grab changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    /// Drag the right edge: width only.
    Right,
    /// Drag the bottom edge: height only.
    Bottom,
    /// Drag the bottom-right corner: both axes.
    Corner,
}

impl ResizeEdge {
    /// `(adjust_width, adjust_height)` — which axes this grab resizes.
    pub fn axes(self) -> (bool, bool) {
        match self {
            Self::Right => (true, false),
            Self::Bottom => (false, true),
            Self::Corner => (true, true),
        }
    }
}

/// How thick the invisible edge / corner grab strips are, in logical pixels.
const EDGE: f32 = 8.0;
const CORNER: f32 = 16.0;

/// Overlay invisible resize strips along `content`'s right and bottom edges and
/// its bottom-right corner, so the card resizes by dragging its own borders (each
/// showing the matching resize cursor) rather than a separate grip. Each strip's
/// press emits `on_resize(edge)`; the strips paint nothing, so the card's own
/// border is the visible edge the user grabs. `iced`'s `stack` sizes to its first
/// child, so the strips span exactly `content`, not the whole window.
///
/// Use this directly when the card also overlays its own control cluster (close,
/// expand, …): stack those controls *over* the returned element so they win the
/// hit test against the strips. For a plain card, prefer [`popover`].
pub fn resize_edges<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    on_resize: impl Fn(ResizeEdge) -> M + 'a,
) -> Element<'a, M> {
    use iced::alignment::{Horizontal::Right, Vertical::Bottom};

    let strip = |edge: ResizeEdge, w: Length, h: Length, cursor: Interaction| -> Element<'a, M> {
        mouse_area(Space::new().width(w).height(h))
            .on_press(on_resize(edge))
            .interaction(cursor)
            .into()
    };
    let right = container(strip(
        ResizeEdge::Right,
        Length::Fixed(EDGE),
        Length::Fill,
        Interaction::ResizingHorizontally,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Right);
    let bottom = container(strip(
        ResizeEdge::Bottom,
        Length::Fill,
        Length::Fixed(EDGE),
        Interaction::ResizingVertically,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Bottom);
    let corner = container(strip(
        ResizeEdge::Corner,
        Length::Fixed(CORNER),
        Length::Fixed(CORNER),
        Interaction::ResizingDiagonallyDown,
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Right)
    .align_y(Bottom);

    stack![content.into(), right, bottom, corner].into()
}

/// Wrap a self-contained `card` into a ready-to-place popover layer: the whole
/// card is the move handle (a press emits `on_move`; the caller tracks the drag),
/// [`resize_edges`] add the border grips, and `opaque` stops a press on the card
/// reaching whatever is behind it. Inner controls, fields, and rows still win the
/// hit test because a child captures the press before the card-wide handle sees it.
///
/// The returned element sizes to the card; the caller positions it (padding /
/// alignment) and stacks it over its base, because anchoring differs per use (a
/// status-bar drill-in floats above the bar, an inspector at an absolute point).
pub fn popover<'a, M: Clone + 'a>(
    card: impl Into<Element<'a, M>>,
    on_move: M,
    on_resize: impl Fn(ResizeEdge) -> M + 'a,
) -> Element<'a, M> {
    opaque(mouse_area(resize_edges(card, on_resize)).on_press(on_move))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_map_each_edge_to_its_dimensions() {
        assert_eq!(ResizeEdge::Right.axes(), (true, false));
        assert_eq!(ResizeEdge::Bottom.axes(), (false, true));
        assert_eq!(ResizeEdge::Corner.axes(), (true, true));
    }
}
