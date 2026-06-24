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
use rime::widgets::{button, card, header_row, labeled, pill, section, stat, text_field};

#[derive(Default)]
struct Gallery {
    choice: ThemeChoice,
    name: String,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    Name(String),
    Noop,
}

impl Gallery {
    fn update(&mut self, message: Message) {
        match message {
            Message::ToggleTheme => self.choice = self.choice.toggled(),
            Message::Name(s) => self.name = s,
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
                section("Field + input"),
                labeled("Name", text_field("type here…", &self.name, Message::Name)),
                section("Stats"),
                row![
                    stat("p50", "12 ms".to_string()),
                    stat("p99", "84 ms".to_string()),
                    stat("rps", "1.2k".to_string()),
                ]
                .spacing(32),
                Space::with_height(8),
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
    iced::application("rime gallery", Gallery::update, Gallery::view)
        .theme(Gallery::theme)
        .window_size(iced::Size::new(760.0, 640.0))
        .run()
}
