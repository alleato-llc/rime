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
