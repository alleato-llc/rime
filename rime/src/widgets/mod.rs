//! The reusable visual vocabulary — one primitive per file. Screens import these
//! and the style functions in [`crate::theme`] instead of re-deriving padding,
//! radius, and color at every call site.
//!
//! Every primitive is generic over the message type `M`, returns an
//! `iced::Element` (or a concrete builder when callers need to chain), holds no
//! state, and reads its colors from [`crate::theme::tokens`] — never a hardcoded
//! color. See `COMPONENTS.md` for the contract a new primitive must meet.

mod autocomplete;
mod banner;
mod bit_grid;
pub mod button;
mod card;
pub mod chart;
mod color_field;
mod context_menu;
mod dialog;
mod field;
pub mod grid;
mod header;
mod input;
pub mod menu;
mod modal;
mod pill;
mod rename;
mod section;
mod select;
mod settings;
mod shell;
mod shortcut;
mod slider;
mod stat;
mod status_bar;
mod stepper;
mod tabs;
mod toggle;
mod tooltip;

pub use autocomplete::{autocomplete_field, suggestion_list, Suggestion};
pub use banner::banner;
pub use bit_grid::{bit_grid, BitBand};
pub use card::card;
pub use chart::{line_chart, LineChart, Series};
pub use color_field::color_field;
pub use context_menu::context_menu;
pub use dialog::dialog;
pub use field::labeled;
pub use grid::{
    grid, CellAlign, Grid, GridCell, Metrics as GridMetrics, Selection as GridSelection,
};
pub use header::header_row;
pub use input::text_field;
pub use menu::{menu_bar, menu_bar_with_trailing, Item as MenuItem, Menu};
pub use modal::modal;
pub use pill::pill;
pub use rename::{rename_bar, rename_field_id};
pub use section::{caption, section};
pub use select::select;
pub use settings::settings;
pub use shell::{title_strip, window_shell};
pub use shortcut::shortcut_row;
pub use slider::slider;
pub use stat::stat;
pub use status_bar::status_bar;
pub use stepper::stepper;
pub use tabs::{tabs, Tab, TabBarStyle, TAB_BAR_HEIGHT};
pub use toggle::toggle;
pub use tooltip::{tooltip, Position as TooltipPosition};
