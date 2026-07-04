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

    /// The inclusive `(row_min, row_max, col_min, col_max)` span.
    fn bounds(&self) -> (usize, usize, usize, usize) {
        (
            self.anchor.0.min(self.extent.0),
            self.anchor.0.max(self.extent.0),
            self.anchor.1.min(self.extent.1),
            self.anchor.1.max(self.extent.1),
        )
    }
}

/// Fixed sizing for the grid, in logical pixels. Uniform cell size keeps the
/// virtualization arithmetic exact; per-column widths and resize-drag are a
/// planned extension (they ride the same viewport math).
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

/// Widget-internal transient state: the live modifiers (so a mouse press knows
/// whether shift is held — mouse events don't carry modifiers in iced) and the
/// last click's time+cell, for double-click detection.
#[derive(Default)]
struct State {
    modifiers: keyboard::Modifiers,
    last_click: Option<(Instant, usize, usize)>,
}

type CellFn<'a> = Box<dyn Fn(usize, usize) -> GridCell + 'a>;
type ScrollFn<'a, Message> = Box<dyn Fn(Vector) -> Message + 'a>;
type SelectFn<'a, Message> = Box<dyn Fn(usize, usize, bool) -> Message + 'a>;
type ActivateFn<'a, Message> = Box<dyn Fn(usize, usize) -> Message + 'a>;

/// An inline editor hosted over one cell: the widget lays it out on top of the
/// cell at `(row, col)` and forwards it events/focus, so a cell can be edited
/// in place (the host supplies a `text_input` or any element).
struct CellEditor<'a, Message, Theme, Renderer> {
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
    editor: Option<CellEditor<'a, Message, Theme, Renderer>>,
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
        editor: None,
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

    /// Host an inline editor `element` over the cell at `(row, col)` — the grid
    /// lays it out on top of that cell and forwards it events + focus, so the
    /// cell edits in place. Omit it (the default) for a read-only/point-select
    /// grid.
    pub fn editor(
        mut self,
        row: usize,
        col: usize,
        element: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.editor = Some(CellEditor {
            row,
            col,
            element: element.into(),
        });
        self
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

    /// The largest legal scroll offset given the body viewport — content size
    /// minus the visible window, never negative.
    fn max_offset(&self, body: Rectangle) -> Vector {
        let content_w = self.cols as f32 * self.metrics.column_width;
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

    /// The inclusive range of columns touching the body window at `offset`.
    fn visible_cols(&self, body: Rectangle, offset: Vector) -> (usize, usize) {
        let first = (offset.x / self.metrics.column_width).floor() as usize;
        let count = (body.width / self.metrics.column_width).ceil() as usize + 2;
        (first.min(self.cols), (first + count).min(self.cols))
    }

    fn visible_rows(&self, body: Rectangle, offset: Vector) -> (usize, usize) {
        let first = (offset.y / self.metrics.row_height).floor() as usize;
        let count = (body.height / self.metrics.row_height).ceil() as usize + 2;
        (first.min(self.rows), (first + count).min(self.rows))
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
        match &self.editor {
            Some(editor) => vec![Tree::new(&editor.element)],
            None => vec![],
        }
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.editor {
            Some(editor) => tree.diff_children(std::slice::from_ref(&editor.element)),
            None => tree.diff_children(&[] as &[Element<'_, Message, Theme, Renderer>]),
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let size = limits.resolve(self.width, self.height, Size::ZERO);
        match &mut self.editor {
            Some(editor) => {
                let origin = grid_cell_origin(
                    size,
                    self.metrics,
                    self.offset,
                    self.rows,
                    self.cols,
                    editor.row,
                    editor.col,
                );
                let cell_limits = Limits::new(
                    Size::ZERO,
                    Size::new(self.metrics.column_width, self.metrics.row_height),
                );
                let child = editor.element.as_widget_mut().layout(
                    &mut tree.children[0],
                    renderer,
                    &cell_limits,
                );
                Node::with_children(size, vec![child.move_to(origin)])
            }
            None => Node::new(size),
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(editor) = &mut self.editor {
            if let Some(child_layout) = layout.children().next() {
                editor.element.as_widget_mut().operate(
                    &mut tree.children[0],
                    child_layout,
                    renderer,
                    operation,
                );
            }
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

        // The inline editor sees events first (typing, its own clicks). If it
        // consumes one, the grid ignores it — so a click inside the editor never
        // moves the selection.
        if let Some(editor) = &mut self.editor {
            if let Some(child_layout) = layout.children().next() {
                editor.element.as_widget_mut().update(
                    &mut tree.children[0],
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
                let Some(position) = cursor.position_in(body) else {
                    return;
                };
                let offset = self.clamped_offset(body);
                let col = ((position.x + offset.x) / self.metrics.column_width).floor() as usize;
                let row = ((position.y + offset.y) / self.metrics.row_height).floor() as usize;
                if row >= self.rows || col >= self.cols {
                    return;
                }
                // A second click on the same cell within the window activates it
                // (inline edit); otherwise it's a plain select.
                let now = Instant::now();
                let is_double = state.last_click.is_some_and(|(when, r, c)| {
                    r == row && c == col && now.duration_since(when).as_secs_f32() < DOUBLE_CLICK_SECS
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
        // Over the editor, defer to it (text I-beam); else the grid's cell cursor.
        if let Some(editor) = &self.editor {
            if let Some(child_layout) = layout.children().next() {
                if cursor.is_over(child_layout.bounds()) {
                    return editor.element.as_widget().mouse_interaction(
                        &tree.children[0],
                        child_layout,
                        cursor,
                        viewport,
                        renderer,
                    );
                }
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
            x: body.x + col as f32 * metrics.column_width - offset.x,
            y: body.y + row as f32 * metrics.row_height - offset.y,
            width: metrics.column_width,
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
                        width: (c1 - c0 + 1) as f32 * metrics.column_width,
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
                    x: body.x + col as f32 * metrics.column_width - offset.x,
                    y: bounds.y,
                    width: metrics.column_width,
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

        // The inline editor, on top of everything, clipped to the body so it
        // never spills into the header strips.
        if let Some(editor) = &self.editor {
            if let Some(child_layout) = layout.children().next() {
                renderer.with_layer(body, |renderer| {
                    editor.element.as_widget().draw(
                        &tree.children[0],
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        viewport,
                    );
                });
            }
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

/// The `(row, col)` cell's top-left relative to the grid's own origin, for
/// positioning the inline editor in `layout`. A free function (not a method) so
/// it can be called while `self.editor` is mutably borrowed.
fn grid_cell_origin(
    size: Size,
    metrics: Metrics,
    offset: Vector,
    rows: usize,
    cols: usize,
    row: usize,
    col: usize,
) -> Point {
    let body_w = (size.width - metrics.header_width).max(0.0);
    let body_h = (size.height - metrics.header_height).max(0.0);
    let max_x = (cols as f32 * metrics.column_width - body_w).max(0.0);
    let max_y = (rows as f32 * metrics.row_height - body_h).max(0.0);
    let off_x = offset.x.clamp(0.0, max_x);
    let off_y = offset.y.clamp(0.0, max_y);
    Point::new(
        metrics.header_width + col as f32 * metrics.column_width - off_x,
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
