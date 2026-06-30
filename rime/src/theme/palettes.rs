//! The built-in palettes rime ships so a new GUI looks good for free. An app that
//! wants a bespoke look constructs its own [`Palette`](super::Palette) instead.

use iced::Color;

use super::Palette;

/// Dracula (dark) — rime's default.
pub const DRACULA: Palette = Palette {
    bg: Color::from_rgb(0.157, 0.165, 0.212),     // #282a36
    surface: Color::from_rgb(0.227, 0.235, 0.31), // #3a3c4e
    ink: Color::from_rgb(0.973, 0.973, 0.949),    // #f8f8f2
    muted: Color::from_rgb(0.541, 0.6, 0.769),    // #8a99c4
    hairline: Color::from_rgb(0.30, 0.31, 0.40),
    accent: Color::from_rgb(0.741, 0.576, 0.976), // #bd93f9
    success: Color::from_rgb(0.314, 0.98, 0.482), // #50fa7b
    warn: Color::from_rgb(1.0, 0.722, 0.424),     // #ffb86c
    danger: Color::from_rgb(1.0, 0.333, 0.333),   // #ff5555
};

/// GitHub (light).
pub const GITHUB: Palette = Palette {
    bg: Color::from_rgb(0.965, 0.973, 0.98),        // #f6f8fa
    surface: Color::from_rgb(1.0, 1.0, 1.0),        // #ffffff
    ink: Color::from_rgb(0.122, 0.137, 0.157),      // #1f2328
    muted: Color::from_rgb(0.396, 0.427, 0.463),    // #656d76
    hairline: Color::from_rgb(0.816, 0.843, 0.871), // #d0d7de
    accent: Color::from_rgb(0.035, 0.412, 0.855),   // #0969da
    success: Color::from_rgb(0.102, 0.498, 0.216),  // #1a7f37
    warn: Color::from_rgb(0.604, 0.404, 0.0),       // #9a6700
    danger: Color::from_rgb(0.812, 0.133, 0.18),    // #cf222e
};

/// Nord (dark).
pub const NORD: Palette = Palette {
    bg: Color::from_rgb8(0x2e, 0x34, 0x40),
    surface: Color::from_rgb8(0x3b, 0x42, 0x52),
    ink: Color::from_rgb8(0xd8, 0xde, 0xe9),
    muted: Color::from_rgb8(0x4c, 0x56, 0x6a),
    hairline: Color::from_rgb8(0x43, 0x4c, 0x5e),
    accent: Color::from_rgb8(0x88, 0xc0, 0xd0),
    success: Color::from_rgb8(0xa3, 0xbe, 0x8c),
    warn: Color::from_rgb8(0xeb, 0xcb, 0x8b),
    danger: Color::from_rgb8(0xbf, 0x61, 0x6a),
};

/// Solarized (dark).
pub const SOLARIZED_DARK: Palette = Palette {
    bg: Color::from_rgb8(0x00, 0x2b, 0x36),
    surface: Color::from_rgb8(0x07, 0x36, 0x42),
    ink: Color::from_rgb8(0x83, 0x94, 0x96),
    muted: Color::from_rgb8(0x58, 0x6e, 0x75),
    hairline: Color::from_rgb8(0x07, 0x36, 0x42),
    accent: Color::from_rgb8(0x26, 0x8b, 0xd2),
    success: Color::from_rgb8(0x85, 0x99, 0x00),
    warn: Color::from_rgb8(0xb5, 0x89, 0x00),
    danger: Color::from_rgb8(0xdc, 0x32, 0x2f),
};

/// Solarized (light).
pub const SOLARIZED_LIGHT: Palette = Palette {
    bg: Color::from_rgb8(0xfd, 0xf6, 0xe3),
    surface: Color::from_rgb8(0xee, 0xe8, 0xd5),
    ink: Color::from_rgb8(0x65, 0x7b, 0x83),
    muted: Color::from_rgb8(0x93, 0xa1, 0xa1),
    hairline: Color::from_rgb8(0xee, 0xe8, 0xd5),
    accent: Color::from_rgb8(0x26, 0x8b, 0xd2),
    success: Color::from_rgb8(0x85, 0x99, 0x00),
    warn: Color::from_rgb8(0xb5, 0x89, 0x00),
    danger: Color::from_rgb8(0xdc, 0x32, 0x2f),
};

/// Gruvbox (dark, medium contrast).
pub const GRUVBOX_DARK: Palette = Palette {
    bg: Color::from_rgb8(0x28, 0x28, 0x28),
    surface: Color::from_rgb8(0x3c, 0x38, 0x36),
    ink: Color::from_rgb8(0xeb, 0xdb, 0xb2),
    muted: Color::from_rgb8(0xa8, 0x99, 0x84),
    hairline: Color::from_rgb8(0x50, 0x49, 0x45),
    accent: Color::from_rgb8(0xfa, 0xbd, 0x2f),
    success: Color::from_rgb8(0xb8, 0xbb, 0x26),
    warn: Color::from_rgb8(0xfe, 0x80, 0x19),
    danger: Color::from_rgb8(0xfb, 0x49, 0x34),
};

/// The built-in named palettes rime ships, in display order: `(name, palette,
/// is_dark)`. The shared source of truth for app chrome — `tty` lists these in its
/// theme picker and `patina` builds its richer editor themes on the matching ones.
pub fn builtin_themes() -> &'static [(&'static str, Palette, bool)] {
    &[
        ("Dracula", DRACULA, true),
        ("Nord", NORD, true),
        ("Gruvbox Dark", GRUVBOX_DARK, true),
        ("Solarized Dark", SOLARIZED_DARK, true),
        ("Solarized Light", SOLARIZED_LIGHT, false),
        ("GitHub", GITHUB, false),
    ]
}
