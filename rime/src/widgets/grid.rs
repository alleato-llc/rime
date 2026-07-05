//! A virtualized spreadsheet grid — frozen row/column headers, a scrolling
//! body that draws only the cells in view, and a selection rectangle. Reach
//! for it when you need a large tabular surface (thousands of rows) that a
//! `column![row![…]]` of widgets would choke on.
//!
//! rime's first custom [`Widget`] (advanced API): it lays out to fill its
//! space and paints cells directly with `fill_quad`/`fill_text`, so cost is
//! bounded by the *viewport*, not the logical size. The generic
//! `fn(row, col) -> GridCell` cell provider keeps it domain-free — the host
//! decides what a cell says and how it's tinted.
//!
//! **State stays with the caller** (the rime rule): scroll `offset` and the
//! [`Selection`] are inputs, not fields the widget mutates. Wheel scrolling
//! reports a new clamped offset via `on_scroll`; a click reports `(row, col,
//! extend)` via `on_select` (`extend` = shift held). The host stores both and
//! passes them back next render — the Elm loop. The only widget-internal state
//! is the live keyboard modifiers, tracked so a click knows whether shift is
//! down.
//!
//! Perf invariants (ported from the SwiftUI grid they replace): the cell
//! provider must be cheap — it's called once per *visible* cell per frame, so
//! do no allocation-heavy work there; a selection change repaints, it does not
//! rebuild a widget tree; judge scrolling on `--release`.

use std::time::Instant;

use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::text::{self, Text};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{renderer, Clipboard, Shell, Widget};
use iced::{
    alignment, keyboard, mouse, Background, Border, Color, Element, Event, Length, Pixels, Point,
    Rectangle, Shadow, Size, Vector,
};

/// The window (in seconds) within which a second click on the same cell counts
/// as a double-click and fires `on_activate` (inline edit), not another select.
const DOUBLE_CLICK_SECS: f32 = 0.4;

/// Horizontal alignment of a cell's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// What one cell shows. The host builds these on demand from the cell
/// provider; everything but `text` has a sensible default so numbers can be
/// `GridCell::right("42")` and labels `GridCell::from("Total")`.
#[derive(Debug, Clone, Default)]
pub struct GridCell {
    pub text: String,
    pub align: CellAlign,
    /// Text color; `None` uses the palette's `ink`.
    pub text_color: Option<Color>,
    /// Cell fill; `None` uses the palette's `bg`.
    pub background: Option<Color>,
}

impl GridCell {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// A right-aligned cell — the natural default for numbers.
    pub fn right(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            align: CellAlign::Right,
            ..Self::default()
        }
    }

    pub fn align(mut self, align: CellAlign) -> Self {
        self.align = align;
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }
}

impl From<&str> for GridCell {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

/// A rectangular selection, in `(row, column)` cell coordinates. `anchor` is
/// where the selection started; `extent` is the opposite corner (equal to
/// `anchor` for a single cell). Corners normalize, so the two may be in any
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: (usize, usize),
    pub extent: (usize, usize),
}

impl Selection {
    pub fn cell(row: usize, col: usize) -> Self {
        Self {
            anchor: (row, col),
            extent: (row, col),
        }
    }

    /// The inclusive `(row_min, row_max, col_min, col_max)` span (corners
    /// normalize, so it's valid for any anchor/extent order).
    pub fn bounds(&self) -> (usize, usize, usize, usize) {
        (
            self.anchor.0.min(self.extent.0),
            self.anchor.0.max(self.extent.0),
            self.anchor.1.min(self.extent.1),
            self.anchor.1.max(self.extent.1),
        )
    }
}

/// Fixed sizing for the grid, in logical pixels. `column_width` is the *default*
/// column width; a caller can override individual columns with
/// [`Grid::column_widths`] (and let the user drag them via
/// [`Grid::on_resize_column`]), in which case this is the fallback for any
/// column the override vector doesn't cover.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub row_height: f32,
    pub column_width: f32,
    /// The frozen column-header strip's height.
    pub header_height: f32,
    /// The frozen row-header strip's width.
    pub header_width: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            row_height: 22.0,
            column_width: 90.0,
            header_height: 22.0,
            header_width: 48.0,
        }
    }
}

const TEXT_SIZE: f32 = 13.0;
const CELL_PAD: f32 = 6.0;
/// How close (px) the pointer must be to a column's right border, in the header
/// strip, to grab it for a resize drag.
const RESIZE_HANDLE: f32 = 4.0;
/// The narrowest a column can be dragged.
const MIN_COLUMN_WIDTH: f32 = 24.0;

/// An in-progress column-resize drag: the column being sized, the pointer x
/// where the drag began (widget-relative), and that column's width at the start.
#[derive(Clone, Copy)]
struct Resizing {
    col: usize,
    start_x: f32,
    start_width: f32,
}

/// Widget-internal transient state: the live modifiers (so a mouse press knows
/// whether shift is held — mouse events don't carry modifiers in iced), the
/// last click's time+cell for double-click detection, and any active
/// column-resize drag.
#[derive(Default)]
struct State {
    modifiers: keyboard::Modifiers,
    last_click: Option<(Instant, usize, usize)>,
    resizing: Option<Resizing>,
}

type CellFn<'a> = Box<dyn Fn(usize, usize) -> GridCell + 'a>;
type ScrollFn<'a, Message> = Box<dyn Fn(Vector) -> Message + 'a>;
type SelectFn<'a, Message> = Box<dyn Fn(usize, usize, bool) -> Message + 'a>;
type ActivateFn<'a, Message> = Box<dyn Fn(usize, usize) -> Message + 'a>;
type ResizeFn<'a, Message> = Box<dyn Fn(usize, f32) -> Message + 'a>;

/// A widget hosted over one cell: the grid lays it out on top of the cell at
/// `(row, col)` and forwards it events/focus, so a cell can host an inline text
/// editor or an interactive control (a slider, checkbox, …) in place. The host
/// supplies the element.
struct CellOverlay<'a, Message, Theme, Renderer> {
    row: usize,
    col: usize,
    element: Element<'a, Message, Theme, Renderer>,
}

/// The grid widget. Build it with [`grid`] and chain the setters; it converts
/// into an [`Element`] via `From`. The `Theme`/`Renderer` params carry the
/// optional inline editor's element (they default to iced's, so the common
/// leaf-grid call site is unchanged).
pub struct Grid<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    rows: usize,
    cols: usize,
    cell: CellFn<'a>,
    metrics: Metrics,
    offset: Vector,
    selection: Option<Selection>,
    palette: crate::theme::Palette,
    on_scroll: Option<ScrollFn<'a, Message>>,
    on_select: Option<SelectFn<'a, Message>>,
    on_activate: Option<ActivateFn<'a, Message>>,
    on_resize_column: Option<ResizeFn<'a, Message>>,
    /// Per-column width overrides, indexed by column. A missing/short entry
    /// falls back to `metrics.column_width`. `None` = every column uniform.
    column_widths: Option<Vec<f32>>,
    overlays: Vec<CellOverlay<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
}

/// A virtualized grid of `rows`×`cols`, drawing each visible cell from
/// `cell(row, col)`. Captures the palette at build time (per the rime rule),
/// so its colors are fixed the moment `view()` runs.
pub fn grid<'a, Message, Theme, Renderer>(
    rows: usize,
    cols: usize,
    cell: impl Fn(usize, usize) -> GridCell + 'a,
) -> Grid<'a, Message, Theme, Renderer> {
    Grid {
        rows,
        cols,
        cell: Box::new(cell),
        metrics: Metrics::default(),
        offset: Vector::ZERO,
        selection: None,
        palette: crate::theme::tokens(),
        on_scroll: None,
        on_select: None,
        on_activate: None,
        on_resize_column: None,
        column_widths: None,
        overlays: Vec::new(),
        width: Length::Fill,
        height: Length::Fill,
    }
}

impl<'a, Message, Theme, Renderer> Grid<'a, Message, Theme, Renderer> {
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// The scroll offset in pixels (caller-owned). Clamped on wheel input
    /// before it's reported, but also clamped at draw time for safety.
    pub fn offset(mut self, offset: Vector) -> Self {
        self.offset = offset;
        self
    }

    pub fn selection(mut self, selection: Option<Selection>) -> Self {
        self.selection = selection;
        self
    }

    /// Fires with the new clamped offset when the wheel scrolls over the grid.
    pub fn on_scroll(mut self, f: impl Fn(Vector) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// Fires `(row, col, extend)` on a body click; `extend` is shift-held.
    pub fn on_select(mut self, f: impl Fn(usize, usize, bool) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    /// Fires `(row, col)` on a double-click — the host uses this to open an
    /// inline editor over that cell (see [`Self::editor`]).
    pub fn on_activate(mut self, f: impl Fn(usize, usize) -> Message + 'a) -> Self {
        self.on_activate = Some(Box::new(f));
        self
    }

    /// Per-column width overrides (indexed by column; a short/absent entry falls
    /// back to `metrics.column_width`). Pair with [`Self::on_resize_column`] to
    /// let the user drag column borders.
    pub fn column_widths(mut self, widths: Vec<f32>) -> Self {
        self.column_widths = Some(widths);
        self
    }

    /// Fires `(col, new_width)` while the user drags a column's right border in
    /// the header strip — the host stores the width and feeds it back through
    /// [`Self::column_widths`]. Widths report already clamped to a sane minimum.
    pub fn on_resize_column(mut self, f: impl Fn(usize, f32) -> Message + 'a) -> Self {
        self.on_resize_column = Some(Box::new(f));
        self
    }

    /// Host `element` over the cell at `(row, col)` — the grid lays it out on top
    /// of that cell and forwards it events + focus. Use it for an inline text
    /// editor (the cell edits in place) or an interactive control (a slider,
    /// checkbox, …). Call it once per hosted cell; later overlays draw on top.
    pub fn overlay(
        mut self,
        row: usize,
        col: usize,
        element: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.overlays.push(CellOverlay {
            row,
            col,
            element: element.into(),
        });
        self
    }

    /// Alias of [`Self::overlay`] that reads clearly at the call site for the
    /// single focus-bearing inline text editor.
    pub fn editor(
        self,
        row: usize,
        col: usize,
        element: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.overlay(row, col, element)
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// The body viewport (the area below the column headers and right of the
    /// row headers), in absolute coordinates.
    fn body(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x + self.metrics.header_width,
            y: bounds.y + self.metrics.header_height,
            width: (bounds.width - self.metrics.header_width).max(0.0),
            height: (bounds.height - self.metrics.header_height).max(0.0),
        }
    }

    /// This column's width — its override if any, else the uniform default.
    fn col_width(&self, col: usize) -> f32 {
        self.column_widths
            .as_ref()
            .and_then(|widths| widths.get(col).copied())
            .filter(|w| *w > 0.0)
            .unwrap_or(self.metrics.column_width)
    }

    /// The left edge of column `col` in content space (sum of prior widths).
    /// `col == self.cols` yields the total content width.
    fn col_left(&self, col: usize) -> f32 {
        match &self.column_widths {
            None => col as f32 * self.metrics.column_width,
            Some(_) => (0..col.min(self.cols)).map(|c| self.col_width(c)).sum(),
        }
    }

    /// Total width of all columns.
    fn content_width(&self) -> f32 {
        self.col_left(self.cols)
    }

    /// The column containing content-space x (clamped to the last column).
    fn col_at(&self, content_x: f32) -> usize {
        match &self.column_widths {
            None => (content_x / self.metrics.column_width).floor() as usize,
            Some(_) => {
                let mut acc = 0.0;
                for col in 0..self.cols {
                    acc += self.col_width(col);
                    if content_x < acc {
                        return col;
                    }
                }
                self.cols.saturating_sub(1)
            }
        }
    }

    /// The largest legal scroll offset given the body viewport — content size
    /// minus the visible window, never negative.
    fn max_offset(&self, body: Rectangle) -> Vector {
        let content_w = self.content_width();
        let content_h = self.rows as f32 * self.metrics.row_height;
        Vector::new(
            (content_w - body.width).max(0.0),
            (content_h - body.height).max(0.0),
        )
    }

    fn clamped_offset(&self, body: Rectangle) -> Vector {
        let max = self.max_offset(body);
        Vector::new(
            self.offset.x.clamp(0.0, max.x),
            self.offset.y.clamp(0.0, max.y),
        )
    }

    /// The half-open range of columns to draw for the body window at `offset`:
    /// the first column at the left edge through the last touching the right
    /// edge, plus a 2-column trailing overscan so a fast scroll never flashes an
    /// unpainted edge.
    fn visible_cols(&self, body: Rectangle, offset: Vector) -> (usize, usize) {
        let first = self.col_at(offset.x);
        let last = self.col_at(offset.x + body.width);
        (first.min(self.cols), (last + 3).min(self.cols))
    }

    fn visible_rows(&self, body: Rectangle, offset: Vector) -> (usize, usize) {
        let first = (offset.y / self.metrics.row_height).floor() as usize;
        let count = (body.height / self.metrics.row_height).ceil() as usize + 2;
        (first.min(self.rows), (first + count).min(self.rows))
    }

    /// The column whose right border sits under the pointer in the column-header
    /// strip, if resize is enabled (`on_resize_column` set) and the pointer is
    /// within [`RESIZE_HANDLE`] of that border. Drives both the grab and the
    /// resize cursor.
    fn resize_handle_at(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        self.on_resize_column.as_ref()?;
        let position = cursor.position_in(bounds)?;
        // The column-header strip only (right of the row header, above the body).
        if position.y > self.metrics.header_height || position.x < self.metrics.header_width {
            return None;
        }
        let body = self.body(bounds);
        let offset = self.clamped_offset(body);
        let (col0, col1) = self.visible_cols(body, offset);
        (col0..col1).find(|&col| {
            let border = self.metrics.header_width + self.col_left(col + 1) - offset.x;
            (position.x - border).abs() <= RESIZE_HANDLE
        })
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Grid<'_, Message, Theme, Renderer>
where
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.overlays
            .iter()
            .map(|overlay| Tree::new(&overlay.element))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let elements: Vec<&Element<'_, Message, Theme, Renderer>> = self
            .overlays
            .iter()
            .map(|overlay| &overlay.element)
            .collect();
        tree.diff_children(&elements);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        if self.overlays.is_empty() {
            return Node::new(size);
        }
        let (metrics, offset, rows) = (self.metrics, self.offset, self.rows);
        let content_w = self.content_width();
        let content_h = rows as f32 * metrics.row_height;
        // Precompute each overlay's column geometry before the mutable borrow.
        let geometry: Vec<(f32, f32, usize)> = self
            .overlays
            .iter()
            .map(|overlay| {
                (
                    self.col_left(overlay.col),
                    self.col_width(overlay.col),
                    overlay.row,
                )
            })
            .collect();
        let children = self
            .overlays
            .iter_mut()
            .zip(tree.children.iter_mut())
            .enumerate()
            .map(|(i, (overlay, child_tree))| {
                let (col_left, col_w, row) = geometry[i];
                let origin =
                    overlay_origin(size, metrics, offset, content_w, content_h, col_left, row);
                let cell_limits = Limits::new(Size::ZERO, Size::new(col_w, metrics.row_height));
                overlay
                    .element
                    .as_widget_mut()
                    .layout(child_tree, renderer, &cell_limits)
                    .move_to(origin)
            })
            .collect();
        Node::with_children(size, children)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        for ((overlay, child_tree), child_layout) in self
            .overlays
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            overlay
                .element
                .as_widget_mut()
                .operate(child_tree, child_layout, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let body = self.body(bounds);

        // Hosted overlays (an inline editor, controls) see events first — their
        // own clicks/typing/drags. If one consumes an event, the grid ignores it,
        // so interacting with an overlay never moves the selection.
        for ((overlay, child_tree), child_layout) in self
            .overlays
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            overlay.element.as_widget_mut().update(
                child_tree,
                event,
                child_layout,
                cursor,
                _renderer,
                _clipboard,
                shell,
                _viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }

        let state = tree.state.downcast_mut::<State>();

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }

            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return;
                }
                let Some(on_scroll) = &self.on_scroll else {
                    return;
                };
                // Lines scroll by one row/column; pixels pass through.
                let (dx, dy) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        (x * self.metrics.column_width, y * self.metrics.row_height)
                    }
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                };
                let current = self.clamped_offset(body);
                let max = self.max_offset(body);
                // Natural direction: wheel-up (positive y) moves content down,
                // i.e. decreases the offset.
                let next = Vector::new(
                    (current.x - dx).clamp(0.0, max.x),
                    (current.y - dy).clamp(0.0, max.y),
                );
                if next != current {
                    shell.publish(on_scroll(next));
                    shell.request_redraw();
                }
                shell.capture_event();
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                // A press on a column-header border begins a resize drag.
                if let Some(col) = self.resize_handle_at(bounds, cursor) {
                    if let Some(position) = cursor.position_in(bounds) {
                        state.resizing = Some(Resizing {
                            col,
                            start_x: position.x,
                            start_width: self.col_width(col),
                        });
                        shell.capture_event();
                        return;
                    }
                }
                let Some(position) = cursor.position_in(body) else {
                    return;
                };
                let offset = self.clamped_offset(body);
                let col = self.col_at(position.x + offset.x);
                let row = ((position.y + offset.y) / self.metrics.row_height).floor() as usize;
                if row >= self.rows || col >= self.cols {
                    return;
                }
                // A second click on the same cell within the window activates it
                // (inline edit); otherwise it's a plain select.
                let now = Instant::now();
                let is_double = state.last_click.is_some_and(|(when, r, c)| {
                    r == row
                        && c == col
                        && now.duration_since(when).as_secs_f32() < DOUBLE_CLICK_SECS
                });
                if is_double {
                    state.last_click = None;
                    if let Some(on_activate) = &self.on_activate {
                        shell.publish(on_activate(row, col));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                } else {
                    state.last_click = Some((now, row, col));
                    if let Some(on_select) = &self.on_select {
                        shell.publish(on_select(row, col, state.modifiers.shift()));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                }
            }

            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(resizing) = state.resizing else {
                    return;
                };
                let Some(on_resize) = &self.on_resize_column else {
                    return;
                };
                let delta = (position.x - bounds.x) - resizing.start_x;
                let width = (resizing.start_width + delta).max(MIN_COLUMN_WIDTH);
                shell.publish(on_resize(resizing.col, width));
                shell.request_redraw();
                shell.capture_event();
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.resizing.take().is_some() =>
            {
                shell.request_redraw();
                shell.capture_event();
            }

            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        // A live resize drag, or hovering a column border, shows the ↔ cursor
        // (and takes priority over anything underneath while dragging).
        let state = tree.state.downcast_ref::<State>();
        if state.resizing.is_some() || self.resize_handle_at(layout.bounds(), cursor).is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }

        // Over a hosted overlay, defer to it (a text I-beam, a slider grab, …);
        // else the grid's own cell cursor.
        for ((overlay, child_tree), child_layout) in self
            .overlays
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
        {
            if cursor.is_over(child_layout.bounds()) {
                return overlay.element.as_widget().mouse_interaction(
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                );
            }
        }
        if cursor.is_over(self.body(layout.bounds())) {
            mouse::Interaction::Cell
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let body = self.body(bounds);
        let offset = self.clamped_offset(body);
        let palette = self.palette;
        let metrics = self.metrics;
        let font = renderer.default_font();

        // Whole-widget background.
        fill(renderer, bounds, palette.bg);

        let (row0, row1) = self.visible_rows(body, offset);
        let (col0, col1) = self.visible_cols(body, offset);

        let cell_rect = |row: usize, col: usize| Rectangle {
            x: body.x + self.col_left(col) - offset.x,
            y: body.y + row as f32 * metrics.row_height - offset.y,
            width: self.col_width(col),
            height: metrics.row_height,
        };

        // Body cells, clipped to the viewport so partial edge cells don't spill
        // into the header strips.
        renderer.with_layer(body, |renderer| {
            for row in row0..row1 {
                for col in col0..col1 {
                    let rect = cell_rect(row, col);
                    let cell = (self.cell)(row, col);
                    stroked(
                        renderer,
                        rect,
                        cell.background.unwrap_or(palette.bg),
                        palette.hairline,
                    );
                    draw_text(
                        renderer,
                        &cell.text,
                        rect,
                        cell.align,
                        cell.text_color.unwrap_or(palette.ink),
                        font,
                    );
                }
            }

            // Selection rectangle over the body.
            if let Some(selection) = &self.selection {
                let (r0, r1, c0, c1) = selection.bounds();
                if r0 < self.rows && c0 < self.cols {
                    let top_left = cell_rect(r0, c0);
                    let rect = Rectangle {
                        x: top_left.x,
                        y: top_left.y,
                        width: self.col_left(c1 + 1) - self.col_left(c0),
                        height: (r1 - r0 + 1) as f32 * metrics.row_height,
                    };
                    let mut tint = palette.accent;
                    tint.a = 0.18;
                    stroked(renderer, rect, tint, palette.accent);
                }
            }
        });

        // Frozen column headers (top strip), scrolling horizontally with the
        // body but pinned vertically.
        let top_strip = Rectangle {
            x: body.x,
            y: bounds.y,
            width: body.width,
            height: metrics.header_height,
        };
        let selected_cols = self.selection.map(|s| {
            let (_, _, c0, c1) = s.bounds();
            (c0, c1)
        });
        renderer.with_layer(top_strip, |renderer| {
            for col in col0..col1 {
                let rect = Rectangle {
                    x: body.x + self.col_left(col) - offset.x,
                    y: bounds.y,
                    width: self.col_width(col),
                    height: metrics.header_height,
                };
                let highlighted = selected_cols.is_some_and(|(c0, c1)| col >= c0 && col <= c1);
                let fill_color = if highlighted {
                    palette.accent
                } else {
                    palette.surface
                };
                let text_color = if highlighted {
                    palette.bg
                } else {
                    palette.muted
                };
                stroked(renderer, rect, fill_color, palette.hairline);
                draw_text(
                    renderer,
                    &column_name(col),
                    rect,
                    CellAlign::Center,
                    text_color,
                    font,
                );
            }
        });

        // Frozen row headers (left strip).
        let left_strip = Rectangle {
            x: bounds.x,
            y: body.y,
            width: metrics.header_width,
            height: body.height,
        };
        let selected_rows = self.selection.map(|s| {
            let (r0, r1, _, _) = s.bounds();
            (r0, r1)
        });
        renderer.with_layer(left_strip, |renderer| {
            for row in row0..row1 {
                let rect = Rectangle {
                    x: bounds.x,
                    y: body.y + row as f32 * metrics.row_height - offset.y,
                    width: metrics.header_width,
                    height: metrics.row_height,
                };
                let highlighted = selected_rows.is_some_and(|(r0, r1)| row >= r0 && row <= r1);
                let fill_color = if highlighted {
                    palette.accent
                } else {
                    palette.surface
                };
                let text_color = if highlighted {
                    palette.bg
                } else {
                    palette.muted
                };
                stroked(renderer, rect, fill_color, palette.hairline);
                draw_text(
                    renderer,
                    &(row + 1).to_string(),
                    rect,
                    CellAlign::Center,
                    text_color,
                    font,
                );
            }
        });

        // The top-left corner box.
        let corner = Rectangle {
            x: bounds.x,
            y: bounds.y,
            width: metrics.header_width,
            height: metrics.header_height,
        };
        stroked(renderer, corner, palette.surface, palette.hairline);

        // Hosted overlays (inline editor, controls), on top of everything and
        // clipped to the body so they never spill into the header strips.
        if !self.overlays.is_empty() {
            renderer.with_layer(body, |renderer| {
                for ((overlay, child_tree), child_layout) in self
                    .overlays
                    .iter()
                    .zip(tree.children.iter())
                    .zip(layout.children())
                {
                    overlay.element.as_widget().draw(
                        child_tree,
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        viewport,
                    );
                }
            });
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Grid<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(grid: Grid<'a, Message, Theme, Renderer>) -> Self {
        Element::new(grid)
    }
}

/// A hosted overlay's top-left relative to the grid's own origin, for
/// positioning it in `layout`. A free function (not a method) so it can be
/// called with `self.overlays` mutably borrowed — the caller passes the
/// content dimensions and the column's precomputed left edge.
fn overlay_origin(
    size: Size,
    metrics: Metrics,
    offset: Vector,
    content_w: f32,
    content_h: f32,
    col_left: f32,
    row: usize,
) -> Point {
    let body_w = (size.width - metrics.header_width).max(0.0);
    let body_h = (size.height - metrics.header_height).max(0.0);
    let max_x = (content_w - body_w).max(0.0);
    let max_y = (content_h - body_h).max(0.0);
    let off_x = offset.x.clamp(0.0, max_x);
    let off_y = offset.y.clamp(0.0, max_y);
    Point::new(
        metrics.header_width + col_left - off_x,
        metrics.header_height + row as f32 * metrics.row_height - off_y,
    )
}

/// Bijective base-26 column name: 0→A, 25→Z, 26→AA, …
fn column_name(mut index: usize) -> String {
    let mut name = Vec::new();
    loop {
        name.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    name.reverse();
    String::from_utf8(name).expect("ASCII column name")
}

fn fill<Renderer: renderer::Renderer>(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
        Background::Color(color),
    );
}

/// A filled rectangle with a 1px hairline border — one grid cell.
fn stroked<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    fill: Color,
    border: Color,
) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border {
                color: border,
                width: 1.0,
                radius: 0.0.into(),
            },
            shadow: Shadow::default(),
            snap: true,
        },
        Background::Color(fill),
    );
}

fn draw_text<Renderer: text::Renderer>(
    renderer: &mut Renderer,
    content: &str,
    rect: Rectangle,
    align: CellAlign,
    color: Color,
    font: Renderer::Font,
) {
    if content.is_empty() {
        return;
    }
    let (align_x, x) = match align {
        CellAlign::Left => (text::Alignment::Left, rect.x + CELL_PAD),
        CellAlign::Center => (text::Alignment::Center, rect.x + rect.width / 2.0),
        CellAlign::Right => (text::Alignment::Right, rect.x + rect.width - CELL_PAD),
    };
    renderer.fill_text(
        Text {
            content: content.to_string(),
            bounds: Size::new((rect.width - 2.0 * CELL_PAD).max(0.0), rect.height),
            size: Pixels(TEXT_SIZE),
            line_height: text::LineHeight::default(),
            font,
            align_x,
            align_y: alignment::Vertical::Center,
            shaping: text::Shaping::Basic,
            wrapping: text::Wrapping::None,
        },
        Point::new(x, rect.y + rect.height / 2.0),
        color,
        rect,
    );
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
