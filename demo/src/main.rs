//! Every rime component on one screen — the visual smoke test. A GUI can't be
//! verified headlessly, so this is how you *look* at the kit:
//!
//! ```sh
//! cargo run --example gallery
//! ```
//!
//! Toggle the theme to confirm every component re-colors from the palette alone.

use iced::widget::{column, row, Space};
use iced::{Element, Length, Theme};
use rime::theme::{self, ThemeChoice};
use rime::widgets::{
    button, caption, card, header_row, labeled, line_chart, pill, section, shortcut_row, slider,
    stat, text_field, title_strip, tooltip, window_shell, LineChart, Series, TooltipPosition,
};

#[derive(Default)]
struct Gallery {
    choice: ThemeChoice,
    name: String,
    amount: f32,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Name(String),
    Amount(f32),
    Noop,
}

impl Gallery {
    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => self.choice = self.choice.toggled(),
            Message::Name(s) => self.name = s,
            Message::Amount(v) => self.amount = v,
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
