//! A macOS-Calculator-style bit editor: a row of clickable bit cells that
//! "light up" when set, grouped into nibbles and tinted by named field bands.
//! Reach for it to edit an integer bit-by-bit, or to show how a register's
//! bits divide into fields (flags, opcodes, headers).
//!
//! **Domain-free and stateless.** It takes the bits as a `Vec<bool>` (index 0
//! = bit 0, the LSB; drawn high→low like a register) and an optional list of
//! [`BitBand`]s describing named ranges; it draws them and reports a click via
//! `on_toggle(bit_index)`. The host owns the value and the layout — decoding an
//! integer into bits, or a field spec into bands, is the app's job (in Soroban,
//! the `BinaryView`/`BitFormat` model; the planned `rust/kit` for Tama). The
//! widget invents no number type — owned inputs (not borrowed slices) because
//! a host usually derives the bits from an integer per render.
//!
//! Enum/numeric field *pickers* (editing a multi-bit field as a value rather
//! than bit-by-bit) are a planned addition that pairs each band with an
//! optional editor; today a band is a visual grouping plus a legend entry.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Border, Color, Element, Length};

use crate::theme::tokens;

const CELL_WIDTH: f32 = 26.0;
const CELL_HEIGHT: f32 = 30.0;
const NIBBLE: usize = 4;

/// A named contiguous range of bits — one field in the layout. `start` is the
/// low bit (0 = LSB); the range is `start ..= start + len - 1`. `color` tints
/// the field's cells; `None` takes a distinct default from the palette.
#[derive(Debug, Clone)]
pub struct BitBand<'a> {
    pub label: &'a str,
    pub start: usize,
    pub len: usize,
    pub color: Option<Color>,
}

impl<'a> BitBand<'a> {
    pub fn new(label: &'a str, start: usize, len: usize) -> Self {
        Self {
            label,
            start,
            len,
            color: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The inclusive high/low bit indices, for a `[hi:lo]` legend label.
    fn high_low(&self) -> (usize, usize) {
        (self.start + self.len.saturating_sub(1), self.start)
    }
}

/// The index of the band containing `bit`, if any (first match wins).
fn band_of(bands: &[BitBand<'_>], bit: usize) -> Option<usize> {
    bands
        .iter()
        .position(|b| bit >= b.start && bit < b.start + b.len)
}

/// The tint for band `index`: its explicit color, else a distinct default
/// rotated through the palette's accent tokens.
fn band_color(bands: &[BitBand<'_>], index: usize, palette: &crate::theme::Palette) -> Color {
    if let Some(color) = bands[index].color {
        return color;
    }
    let defaults = [
        palette.accent,
        palette.success,
        palette.warn,
        palette.danger,
    ];
    defaults[index % defaults.len()]
}

/// A bit editor over `bits` (LSB-first), grouped into nibbles and tinted by
/// `bands`. A set bit lights up in its field's color; clicking any cell emits
/// `on_toggle(bit_index)`. Pass an empty `bands` for a plain register.
pub fn bit_grid<'a, M: Clone + 'a>(
    bits: Vec<bool>,
    bands: Vec<BitBand<'a>>,
    on_toggle: impl Fn(usize) -> M + Copy + 'a,
) -> Element<'a, M> {
    let palette = tokens();
    let width = bits.len();

    // Bit cells + their position labels, high → low, with a gap between nibbles.
    let mut cells = row![].spacing(2);
    let mut labels = row![].spacing(2);
    for display in 0..width {
        let bit = width - 1 - display; // MSB first
        if display > 0 && bit % NIBBLE == NIBBLE - 1 {
            cells = cells.push(Space::new().width(Length::Fixed(8.0)));
            labels = labels.push(Space::new().width(Length::Fixed(8.0)));
        }

        let set = bits[bit];
        let tint = band_of(&bands, bit).map(|i| band_color(&bands, i, &palette));
        let lit = tint.unwrap_or(palette.accent);

        cells = cells.push(
            button(
                text(if set { "1" } else { "0" })
                    .size(14)
                    .center()
                    .width(Length::Fill),
            )
            .on_press(on_toggle(bit))
            .width(Length::Fixed(CELL_WIDTH))
            .height(Length::Fixed(CELL_HEIGHT))
            .padding(0)
            .style(move |_theme, status| {
                let hovered = matches!(status, button::Status::Hovered);
                let background = if set {
                    lit
                } else if hovered {
                    palette.surface
                } else {
                    palette.bg
                };
                button::Style {
                    background: Some(background.into()),
                    text_color: if set { palette.bg } else { palette.muted },
                    border: Border {
                        color: palette.hairline,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..button::Style::default()
                }
            }),
        );

        labels = labels.push(
            container(text(bit.to_string()).size(9).color(palette.muted))
                .width(Length::Fixed(CELL_WIDTH))
                .center_x(Length::Fixed(CELL_WIDTH)),
        );
    }

    let mut grid = column![cells, labels].spacing(2);

    // Field legend: a colored chip + name + [hi:lo] per band.
    if !bands.is_empty() {
        let mut legend = row![].spacing(12);
        for (index, band) in bands.iter().enumerate() {
            let color = band_color(&bands, index, &palette);
            let (hi, lo) = band.high_low();
            legend = legend.push(
                row![
                    container(
                        Space::new()
                            .width(Length::Fixed(10.0))
                            .height(Length::Fixed(10.0))
                    )
                    .style(move |_theme| container::Style {
                        background: Some(color.into()),
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }),
                    text(format!("{} [{hi}:{lo}]", band.label))
                        .size(12)
                        .color(palette.ink),
                ]
                .spacing(6),
            );
        }
        grid = grid.push(Space::new().height(Length::Fixed(4.0)));
        grid = grid.push(legend);
    }

    grid.into()
}

#[cfg(test)]
#[path = "bit_grid_tests.rs"]
mod tests;
