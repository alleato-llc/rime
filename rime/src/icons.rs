//! A small embedded **icon font** — a subset of [Lucide](https://lucide.dev)
//! (ISC-licensed) — so hosts get consistent, always-rendering glyphs instead of
//! relying on the platform's emoji/symbol coverage (iced's default text font
//! renders neither emoji like `📖` nor box-drawing/PUA symbols, so they show as
//! tofu). Load the bytes once at startup and render glyphs in [`FONT`]:
//!
//! ```ignore
//! iced::application(..).font(rime::icons::FONT_BYTES).run()
//! // then, anywhere in a view:
//! rime::widgets::button::icon(rime::icons::glyph::SETTINGS, Message::OpenSettings)
//! ```
//!
//! The glyphs live in the Private Use Area; the [`glyph`] consts name the ones
//! this kit ships. To add more, re-subset the upstream font (see the repo's
//! tooling) and extend [`glyph`].

use iced::widget::{text, Text};
use iced::Font;

/// The embedded icon-font bytes. Pass to `iced::application(..).font(..)` once
/// at startup so the [`FONT`] handle resolves.
pub const FONT_BYTES: &[u8] = include_bytes!("assets/rime-icons.ttf");

/// The font handle for rendering icon glyphs. Resolves only after
/// [`FONT_BYTES`] has been loaded into the iced application.
pub const FONT: Font = Font::with_name("Rime Icons");

/// The icon codepoints this kit ships (Lucide names in comments).
pub mod glyph {
    pub const CLOSE: char = '\u{e1b2}'; // x
    pub const GRID: char = '\u{e17d}'; // table
    pub const LOG: char = '\u{e181}'; // terminal
    pub const REFERENCE: char = '\u{e05f}'; // book-open
    pub const BITS: char = '\u{e1f2}'; // binary
    pub const NAMES: char = '\u{e431}'; // panel-right
    pub const SETTINGS: char = '\u{e154}'; // settings
    pub const UNDO: char = '\u{e19b}'; // undo
    pub const REDO: char = '\u{e143}'; // redo
    pub const MENU: char = '\u{e115}'; // menu
    pub const CHEVRON_LEFT: char = '\u{e06e}'; // chevron-left
    pub const CHEVRON_RIGHT: char = '\u{e06f}'; // chevron-right
    pub const FUNCTION: char = '\u{e22d}'; // square-function
    pub const VARIABLE: char = '\u{e473}'; // variable
    pub const COPY: char = '\u{e09e}'; // copy
    pub const CUT: char = '\u{e14e}'; // scissors
    pub const PASTE: char = '\u{e085}'; // clipboard
}

/// An icon `glyph` as a `Text` in the icon [`FONT`]. Size/color it like any
/// other `text` (`icon(glyph::CLOSE).size(14).color(..)`).
pub fn icon<'a>(glyph: char) -> Text<'a> {
    text(glyph.to_string()).font(FONT)
}
