//! The reusable visual vocabulary — one primitive per file. Screens import these
//! and the style functions in [`crate::theme`] instead of re-deriving padding,
//! radius, and color at every call site.
//!
//! Every primitive is generic over the message type `M`, returns an
//! `iced::Element` (or a concrete builder when callers need to chain), holds no
//! state, and reads its colors from [`crate::theme::tokens`] — never a hardcoded
//! color. See `COMPONENTS.md` for the contract a new primitive must meet.

pub mod button;
mod card;
pub mod chart;
mod field;
mod header;
mod input;
mod pill;
mod section;
mod stat;

pub use card::card;
pub use chart::{line_chart, LineChart, Series};
pub use field::labeled;
pub use header::header_row;
pub use input::text_field;
pub use pill::pill;
pub use section::section;
pub use stat::stat;
