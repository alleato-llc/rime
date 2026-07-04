//! Every rime component on one screen — the visual smoke test. A GUI can't be
//! verified headlessly, so this is how you *look* at the kit:
//!
//! ```sh
//! cargo run --example gallery
//! ```
//!
//! Toggle the theme to confirm every component re-colors from the palette alone.

use iced::widget::{column, container, row, Space};
use iced::{Element, Length, Theme, Vector};
use rime::theme::{self, ThemeChoice};
use rime::widgets::{
    button, caption, card, grid, header_row, labeled, line_chart, pill, rename_bar, section,
    shortcut_row, slider, stat, text_field, title_strip, tooltip, window_shell, CellAlign,
    GridCell, GridSelection, LineChart, Series, TooltipPosition,
};

// The demo grid's logical size — big enough to show virtualization + scroll.
const GRID_ROWS: usize = 200;
const GRID_COLS: usize = 26;

#[derive(Default)]
struct Gallery {
    choice: ThemeChoice,
    name: String,
    amount: f32,
    grid_offset: Vector,
    grid_selection: Option<GridSelection>,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Name(String),
    Amount(f32),
    GridScrolled(Vector),
    GridSelected(usize, usize, bool),
    Noop,
}

impl Gallery {
    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => self.choice = self.choice.toggled(),
            Message::Name(s) => self.name = s,
            Message::Amount(v) => self.amount = v,
            Message::GridScrolled(offset) => self.grid_offset = offset,
            Message::GridSelected(row, col, extend) => {
                // The caller owns selection: extend keeps the anchor and moves
                // the opposite corner; a plain click starts fresh.
                self.grid_selection = Some(match (extend, self.grid_selection) {
                    (true, Some(current)) => GridSelection {
                        anchor: current.anchor,
                        extent: (row, col),
                    },
                    _ => GridSelection::cell(row, col),
                });
            }
            Message::Noop => {}
        }
    }

    fn theme(&self) -> Theme {
        self.choice.theme()
    }

    fn view(&self) -> Element<'_, Message> {
        // Open the palette once for the whole render pass.
        let _scope = theme::enter(self.choice.palette());
        let t = theme::tokens();

        let body = card(
            column![
                header_row("rime gallery", "every component, one screen"),
                section("Buttons"),
                row![
                    button::primary("Primary", Message::Noop),
                    button::secondary("Secondary", Message::Noop),
                    button::danger("Danger", Message::Noop),
                    button::ghost("Ghost", Message::Noop),
                ]
                .spacing(8),
                section("Pills"),
                row![
                    pill("running", t.accent),
                    pill("done", t.success),
                    pill("queued", t.muted),
                    pill("failed", t.danger),
                ]
                .spacing(8),
                section("Tooltip"),
                row![
                    tooltip(
                        pill("hover me", t.accent),
                        "A tooltip explains the thing under the cursor — pairs with a pill.",
                        TooltipPosition::Top,
                    ),
                    tooltip(
                        button::secondary("or me", Message::Noop),
                        "Works on any element, not just pills.",
                        TooltipPosition::Right,
                    ),
                ]
                .spacing(8),
                section("Field + input"),
                labeled("Name", text_field("type here…", &self.name, Message::Name)),
                section("Rename bar"),
                rename_bar(
                    "Rename tab",
                    "Tab name…",
                    &self.name,
                    Message::Name,
                    Message::Noop
                ),
                section("Slider"),
                slider(
                    "Amount",
                    0.0..=1.0,
                    self.amount,
                    format!("{}%", (self.amount * 100.0).round() as i32),
                    Message::Amount,
                ),
                section("Stats"),
                row![
                    stat("p50", "12 ms".to_string()),
                    stat("p99", "84 ms".to_string()),
                    stat("rps", "1.2k".to_string()),
                ]
                .spacing(32),
                section("Shortcuts"),
                caption("BINDINGS"),
                shortcut_row("⌘T / ⌘N", "New tab"),
                shortcut_row("⌥⌘ + arrows", "Split the focused pane"),
                shortcut_row("right-click / ⌃-click", "Context menu"),
                section("Window shell"),
                iced::widget::container(title_strip(
                    "title_strip.rs",
                    vec![button::ghost("Reattach", Message::Noop).into()],
                ))
                .width(Length::Fill),
                iced::widget::container(window_shell(
                    "detached.rs",
                    vec![button::ghost("Reattach", Message::Noop).into()],
                    iced::widget::container(iced::widget::text("…window body…").color(t.muted),)
                        .padding(12),
                    "detached.rs",
                    "120×40",
                ))
                .height(Length::Fixed(160.0)),
                section("Grid"),
                caption("SCROLL · CLICK TO SELECT · SHIFT-CLICK TO EXTEND"),
                container(
                    grid(GRID_ROWS, GRID_COLS, |r, c| {
                        // A multiplication table: right-aligned numbers, with
                        // the first column tinted as a row label.
                        if c == 0 {
                            GridCell::new(format!("row {}", r + 1)).align(CellAlign::Left)
                        } else {
                            GridCell::right(((r + 1) * (c + 1)).to_string())
                        }
                    })
                    .offset(self.grid_offset)
                    .selection(self.grid_selection)
                    .on_scroll(Message::GridScrolled)
                    .on_select(Message::GridSelected),
                )
                .width(Length::Fill)
                .height(Length::Fixed(200.0)),
                section("Chart"),
                line_chart(
                    LineChart {
                        title: "demo series".to_string(),
                        series: vec![
                            Series {
                                points: vec![
                                    (0.0, 2.0),
                                    (1.0, 5.0),
                                    (2.0, 3.0),
                                    (3.0, 8.0),
                                    (4.0, 6.0)
                                ],
                                color: t.accent,
                            },
                            Series {
                                points: vec![
                                    (0.0, 1.0),
                                    (1.0, 2.0),
                                    (2.0, 4.0),
                                    (3.0, 3.0),
                                    (4.0, 5.0)
                                ],
                                color: t.success,
                            },
                        ],
                    },
                    160.0,
                ),
                Space::new().height(8),
                button::secondary("Toggle theme", Message::ToggleTheme),
            ]
            .spacing(16),
        );

        iced::widget::container(body)
            .padding(24)
            .max_width(720)
            .center_x(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn main() -> iced::Result {
    iced::application(Gallery::default, Gallery::update, Gallery::view)
        .title("rime gallery")
        .theme(Gallery::theme)
        .window_size(iced::Size::new(760.0, 640.0))
        .run()
}
