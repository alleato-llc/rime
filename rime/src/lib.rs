//! rime — a small, consistent component kit on top of [iced].
//!
//! It is the answer to "I don't want to style every button" — the corner radius,
//! padding, surface colors, and text sizes live in one place, so a screen writes
//! [`widgets::button::primary`] and never repeats `.padding(..).style(..)`. The
//! components are pure builders, generic over your `Message` type, returning
//! `iced::Element`s; they hold no state and know nothing about your domain.
//!
//! # The palette channel
//!
//! Components draw with nine semantic [`theme::Palette`] tokens
//! (`bg`/`surface`/`ink`/`muted`/`hairline`/`accent`/`success`/`warn`/`danger`),
//! more than iced's five-slot [`iced::theme::Palette`] carries. Rather than thread
//! the palette through every call, the host opens it once per render with
//! [`theme::enter`] (or [`theme::scope`]); components read the active one via
//! [`theme::tokens`]. `view()` is synchronous and single-threaded, so the channel
//! is a thread-local — set by the host, only ever read by components.
//!
//! ```no_run
//! use rime::theme::{self, ThemeChoice};
//! use rime::widgets::{button, card};
//! # #[derive(Clone)] enum Message { Run }
//! # fn build() -> iced::Element<'static, Message> {
//! // at the top of your `view`:
//! let _scope = theme::enter(ThemeChoice::Dark.palette());
//! card(button::primary("Run", Message::Run)).into()
//! # }
//! ```
//!
//! `iced` itself is re-exported as [`rime::iced`](iced) so dependents share one
//! version.

pub use iced;

pub mod theme;
pub mod widgets;
