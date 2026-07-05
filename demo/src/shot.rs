//! Headless review-screenshot capture — a permanent, env-gated dev affordance.
//! iced can capture its own window via wgpu texture readback
//! (`window::screenshot`), which sidesteps the macOS screen-recording TCC
//! prompt entirely — no display server involvement, no permission needed.
//! Mirrors the same harness in soroban's `shot.rs`.
//!
//! Inert unless `RIME_DEMO_SHOT` is set: [`configure`] returns `None` and
//! nothing subscribes or changes behavior.
//!
//! - `RIME_DEMO_SHOT=<path>` — capture the window to `<path>` (a `.png`) and
//!   exit, as soon as the first frame after window-open paints (i.e. exactly
//!   what a human sees on launch, before ever touching the window).

use iced::{window, Task};

use crate::Message;

pub struct Shot {
    path: String,
    window: Option<window::Id>,
    saved: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    WindowOpened(window::Id),
    Frame,
    Captured(window::Screenshot),
}

pub fn configure() -> Option<Shot> {
    let path = std::env::var("RIME_DEMO_SHOT").ok()?;
    Some(Shot {
        path,
        window: None,
        saved: false,
    })
}

pub fn handle(shot: &mut Shot, event: Event) -> Task<Message> {
    match event {
        Event::WindowOpened(id) => {
            shot.window = Some(id);
            Task::none()
        }
        Event::Frame => {
            if !shot.saved {
                if let Some(id) = shot.window {
                    return window::screenshot(id).map(|s| Message::Shot(Event::Captured(s)));
                }
            }
            Task::none()
        }
        Event::Captured(screenshot) => {
            shot.saved = true;
            save_png(&shot.path, &screenshot);
            iced::exit()
        }
    }
}

pub fn subscription(_shot: &Shot) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        window::open_events().map(|id| Message::Shot(Event::WindowOpened(id))),
        window::frames().map(|_| Message::Shot(Event::Frame)),
    ])
}

fn save_png(path: &str, screenshot: &window::Screenshot) {
    let file = std::fs::File::create(path).expect("create png");
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        screenshot.size.width,
        screenshot.size.height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("png header");
    writer
        .write_image_data(&screenshot.rgba)
        .expect("png image data");
}
