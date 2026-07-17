//! A plain, virtualized data table — a header row + a scrolling body, with
//! whole-*row* selection/highlight. The general-purpose "list of records"
//! counterpart to [`grid`](super::grid)'s spreadsheet semantics (per-cell
//! selection, an inline cell editor, resizable columns, column-letter/
//! row-number chrome). Reach for `table` for anything that reads as rows of
//! text — logs, search results, file lists — and `grid` only when you
//! actually need spreadsheet behavior.
//!
//! Virtualized like `grid` (cost bounded by the viewport, not the row count)
//! since a table can reasonably hold thousands of rows (e.g. terminal
//! scrollback). **State stays with the caller** (the rime rule): `offset` and
//! `selected` are inputs, not fields the widget mutates — a click reports the
//! row via `on_select`, a double-click via `on_activate`, a right-click via
//! `on_right_click` (e.g. to anchor a copy/clear context menu); the host
//! stores whatever it needs and passes it back next render.

use std::time::Instant;

use iced::advanced::layout::{Layout, Limits, Node};
use iced::advanced::text::{self, Text};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{renderer, Clipboard, Shell, Widget};
use iced::{
    alignment, keyboard, mouse, Background, Border, Color, Element, Event, Font, Length, Pixels,
    Point, Rectangle, Shadow, Size,
};

use super::grid::CellAlign;

/// The window (in seconds) within which a second click on the same row counts
/// as a double-click and fires `on_activate`, not another `on_select`.
const DOUBLE_CLICK_SECS: f32 = 0.4;
const TEXT_SIZE: f32 = 13.0;
const CELL_PAD: f32 = 8.0;

/// A column's width: a fixed pixel width, or a share of whatever's left after
/// the fixed columns (evenly split among every `Fill` column — typically
/// there's just one, stretching to the table's width).
#[derive(Debug, Clone, Copy)]
pub enum ColumnWidth {
    Fixed(f32),
    Fill,
}

#[derive(Debug, Clone)]
pub struct TableColumn {
    pub header: String,
    pub width: ColumnWidth,
    pub align: CellAlign,
}

impl TableColumn {
    pub fn fixed(header: impl Into<String>, width: f32) -> Self {
        Self {
            header: header.into(),
            width: ColumnWidth::Fixed(width),
            align: CellAlign::Left,
        }
    }

    /// A column that stretches to fill the width left over after the fixed
    /// columns — the common case for a single "the rest of the row" column.
    pub fn fill(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            width: ColumnWidth::Fill,
            align: CellAlign::Left,
        }
    }

    pub fn align(mut self, align: CellAlign) -> Self {
        self.align = align;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TableMetrics {
    pub row_height: f32,
    /// The header row's height; `0.0` omits the header entirely.
    pub header_height: f32,
}

impl Default for TableMetrics {
    fn default() -> Self {
        Self {
            row_height: 22.0,
            header_height: 26.0,
        }
    }
}

/// Widget-internal transient state: the last click's time + row (for double-click
/// detection, mirrors `grid`'s `State`) and the live keyboard modifiers (so a
/// Ctrl+click — macOS's secondary-click convention, which arrives as a *left*
/// press with Control held, never a real right button — can be treated as a
/// right-click; see the `ButtonPressed(Left)` arm in `update`).
#[derive(Default)]
struct State {
    last_click: Option<(Instant, usize)>,
    modifiers: keyboard::Modifiers,
}

type CellFn<'a> = Box<dyn Fn(usize, usize) -> String + 'a>;
type ScrollFn<'a, Message> = Box<dyn Fn(f32) -> Message + 'a>;
type SelectFn<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;
type ActivateFn<'a, Message> = Box<dyn Fn(usize) -> Message + 'a>;

/// The table widget. Build it with [`table`] and chain the setters; it
/// converts into an [`Element`] via `From`.
pub struct Table<'a, Message> {
    rows: usize,
    columns: Vec<TableColumn>,
    cell: CellFn<'a>,
    metrics: TableMetrics,
    offset: f32,
    selected: Option<usize>,
    palette: crate::theme::Palette,
    font: Option<Font>,
    on_scroll: Option<ScrollFn<'a, Message>>,
    on_select: Option<SelectFn<'a, Message>>,
    on_activate: Option<ActivateFn<'a, Message>>,
    on_right_click: Option<SelectFn<'a, Message>>,
    width: Length,
    height: Length,
}

/// A virtualized table of `rows` × `columns.len()`, drawing each visible cell's
/// text from `cell(row, col)`. Captures the palette at build time (per the
/// rime rule), so its colors are fixed the moment `view()` runs.
pub fn table<'a, Message>(
    rows: usize,
    columns: Vec<TableColumn>,
    cell: impl Fn(usize, usize) -> String + 'a,
) -> Table<'a, Message> {
    Table {
        rows,
        columns,
        cell: Box::new(cell),
        metrics: TableMetrics::default(),
        offset: 0.0,
        selected: None,
        palette: crate::theme::tokens(),
        font: None,
        on_scroll: None,
        on_select: None,
        on_activate: None,
        on_right_click: None,
        width: Length::Fill,
        height: Length::Fill,
    }
}

impl<'a, Message> Table<'a, Message> {
    pub fn metrics(mut self, metrics: TableMetrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// The vertical scroll offset in pixels (caller-owned).
    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    /// The selected row, highlighted with an accent tint.
    pub fn selected(mut self, row: Option<usize>) -> Self {
        self.selected = row;
        self
    }

    /// Override the default UI font — e.g. a monospace face for tabular data
    /// where columns need to line up (log lines, code, numbers).
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Fires with the new clamped offset when the wheel scrolls over the table.
    pub fn on_scroll(mut self, f: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// Fires `row` on a single click.
    pub fn on_select(mut self, f: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    /// Fires `row` on a double-click (the host's cue to e.g. copy that row).
    pub fn on_activate(mut self, f: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_activate = Some(Box::new(f));
        self
    }

    /// Fires `row` on a right-click (the host's cue to open a context menu
    /// anchored at that row, e.g. copy/clear).
    pub fn on_right_click(mut self, f: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_right_click = Some(Box::new(f));
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

    /// Resolved `(left_x, width)` per column for `available_width` — fixed
    /// columns keep their width; the rest splits evenly among `Fill` columns.
    fn column_layout(&self, available_width: f32) -> Vec<(f32, f32)> {
        let fixed_total: f32 = self
            .columns
            .iter()
            .filter_map(|c| match c.width {
                ColumnWidth::Fixed(w) => Some(w),
                ColumnWidth::Fill => None,
            })
            .sum();
        let fill_count = self
            .columns
            .iter()
            .filter(|c| matches!(c.width, ColumnWidth::Fill))
            .count();
        let fill_width = if fill_count > 0 {
            ((available_width - fixed_total) / fill_count as f32).max(20.0)
        } else {
            0.0
        };
        let mut x = 0.0;
        self.columns
            .iter()
            .map(|c| {
                let w = match c.width {
                    ColumnWidth::Fixed(w) => w,
                    ColumnWidth::Fill => fill_width,
                };
                let left = x;
                x += w;
                (left, w)
            })
            .collect()
    }

    /// The scrolling body — below the header strip, full width.
    fn body(&self, bounds: Rectangle) -> Rectangle {
        Rectangle {
            x: bounds.x,
            y: bounds.y + self.metrics.header_height,
            width: bounds.width,
            height: (bounds.height - self.metrics.header_height).max(0.0),
        }
    }

    fn max_offset(&self, body: Rectangle) -> f32 {
        (self.rows as f32 * self.metrics.row_height - body.height).max(0.0)
    }

    fn clamped_offset(&self, body: Rectangle) -> f32 {
        self.offset.clamp(0.0, self.max_offset(body))
    }

    /// The half-open row range to draw for the body window at `offset`, plus a
    /// 2-row trailing overscan so a fast scroll never flashes an unpainted edge.
    fn visible_rows(&self, body: Rectangle, offset: f32) -> (usize, usize) {
        let first = (offset / self.metrics.row_height).floor() as usize;
        let count = (body.height / self.metrics.row_height).ceil() as usize + 2;
        (first.min(self.rows), (first + count).min(self.rows))
    }

    /// The row under body-relative `position`, if any (clamps to `None` above
    /// the body, at the bottom overscan, or past the last row).
    fn row_at(&self, offset: f32, position: Point) -> Option<usize> {
        if position.y < 0.0 {
            return None;
        }
        let row = ((position.y + offset) / self.metrics.row_height).floor() as usize;
        (row < self.rows).then_some(row)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Table<'_, Message>
where
    Renderer: text::Renderer<Font = Font>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        Node::new(limits.resolve(self.width, self.height, Size::ZERO))
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
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * self.metrics.row_height,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };
                let current = self.clamped_offset(body);
                let max = self.max_offset(body);
                let next = (current - dy).clamp(0.0, max);
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
                let Some(row) = self.row_at(offset, position) else {
                    return;
                };
                // Ctrl+click is macOS's secondary-click gesture (Control held on a
                // *left* press, never a real right button) — treat it exactly like
                // `ButtonPressed(Right)` below rather than also selecting/toggling.
                if state.modifiers.control() {
                    if let Some(on_right_click) = &self.on_right_click {
                        shell.publish(on_right_click(row));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    return;
                }
                let now = Instant::now();
                let is_double = state.last_click.is_some_and(|(when, r)| {
                    r == row && now.duration_since(when).as_secs_f32() < DOUBLE_CLICK_SECS
                });
                if is_double {
                    state.last_click = None;
                    if let Some(on_activate) = &self.on_activate {
                        shell.publish(on_activate(row));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                } else {
                    state.last_click = Some((now, row));
                    if let Some(on_select) = &self.on_select {
                        shell.publish(on_select(row));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let Some(position) = cursor.position_in(body) else {
                    return;
                };
                let offset = self.clamped_offset(body);
                let Some(row) = self.row_at(offset, position) else {
                    return;
                };
                if let Some(on_right_click) = &self.on_right_click {
                    shell.publish(on_right_click(row));
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(self.body(layout.bounds())) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let body = self.body(bounds);
        let offset = self.clamped_offset(body);
        let palette = self.palette;
        let metrics = self.metrics;
        let font = self.font.unwrap_or_else(|| renderer.default_font());
        let cols = self.column_layout(bounds.width);

        fill(renderer, bounds, palette.bg);

        if metrics.header_height > 0.0 {
            let header_rect = Rectangle {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width,
                height: metrics.header_height,
            };
            fill(renderer, header_rect, palette.surface);
            for (col, (left, width)) in self.columns.iter().zip(cols.iter()) {
                let rect = Rectangle {
                    x: bounds.x + left,
                    y: bounds.y,
                    width: *width,
                    height: metrics.header_height,
                };
                draw_text(renderer, &col.header, rect, col.align, palette.muted, font);
            }
            hline(
                renderer,
                bounds.x,
                bounds.x + bounds.width,
                bounds.y + metrics.header_height,
                palette.hairline,
            );
        }

        let (row0, row1) = self.visible_rows(body, offset);
        renderer.with_layer(body, |renderer| {
            for row in row0..row1 {
                let y = body.y + row as f32 * metrics.row_height - offset;
                let row_rect = Rectangle {
                    x: bounds.x,
                    y,
                    width: bounds.width,
                    height: metrics.row_height,
                };
                if self.selected == Some(row) {
                    let mut tint = palette.accent;
                    tint.a = 0.18;
                    fill(renderer, row_rect, tint);
                } else if row % 2 == 1 {
                    // A faint zebra stripe — helps the eye track a row across
                    // wide content without fighting the selection tint.
                    let mut tint = palette.surface;
                    tint.a = 0.4;
                    fill(renderer, row_rect, tint);
                }
                for (col_idx, (left, width)) in cols.iter().enumerate() {
                    let text = (self.cell)(row, col_idx);
                    if text.is_empty() {
                        continue;
                    }
                    let rect = Rectangle {
                        x: bounds.x + left,
                        y,
                        width: *width,
                        height: metrics.row_height,
                    };
                    let align = self
                        .columns
                        .get(col_idx)
                        .map(|c| c.align)
                        .unwrap_or_default();
                    draw_text(renderer, &text, rect, align, palette.ink, font);
                }
                hline(
                    renderer,
                    bounds.x,
                    bounds.x + bounds.width,
                    y + metrics.row_height,
                    palette.hairline,
                );
            }
        });
    }
}

impl<'a, Message, Theme, Renderer> From<Table<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = Font>,
{
    fn from(table: Table<'a, Message>) -> Self {
        Element::new(table)
    }
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

fn hline<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    x0: f32,
    x1: f32,
    y: f32,
    color: Color,
) {
    fill(
        renderer,
        Rectangle {
            x: x0,
            y,
            width: (x1 - x0).max(0.0),
            height: 1.0,
        },
        color,
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
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        },
        Point::new(x, rect.y + rect.height / 2.0),
        color,
        rect,
    );
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
