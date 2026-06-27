# rime

A small, consistent component kit on top of [iced](https://iced.rs).

`rime` is the answer to "I don't want to style every button." The corner radius,
padding, surface colors, and text sizes live in one place, so a screen writes
`button::primary("Run", Message::Run)` and never repeats `.padding(..).style(..)`.
Components are pure builders — generic over your `Message` type, returning
`iced::Element`s — that hold no state and know nothing about your domain.

```toml
[dependencies]
rime = { path = "../rime" } # or a version, once published
```

## Quickstart

Open a palette once per render, then build from the components. They read the
active palette themselves, so call sites stay clean:

```rust
use rime::theme::{self, ThemeChoice};
use rime::widgets::{button, card, header_row, stat};

fn view(&self) -> iced::Element<'_, Message> {
    // Open the palette for this whole render pass; it restores on drop.
    let _scope = theme::enter(self.theme_choice.palette());

    card(iced::widget::column![
        header_row("Dashboard", ""),
        stat("p50", "12 ms".to_string()),
        button::primary("Refresh", Message::Refresh),
    ]).into()
}

// and wire the iced theme for the built-in widgets:
fn theme(&self) -> iced::Theme { self.theme_choice.theme() }
```

## The palette channel

Components draw with nine semantic tokens — `bg`, `surface`, `ink`, `muted`,
`hairline`, `accent`, `success`, `warn`, `danger` — more than iced's five-slot
`Palette` carries. Rather than thread the palette through every call, the host
opens it once with `theme::enter` (RAII) or `theme::scope` (closure); components
read the active one via `theme::tokens()`. `view()` is synchronous and
single-threaded, so the channel is a thread-local: set by the host, only ever read
by components.

`rime` owns the theming *system* and ships defaults (`DRACULA`, `GITHUB`); your app
overrides the *content* — construct your own `Palette`, pick which is active, and
choose where the choice persists (`theme::load` / `theme::save` take the path).
The token *vocabulary* is fixed: that is exactly what lets a component render
correctly under any palette.

## Components

`button` (primary/secondary/danger/ghost), `card`, `text_field`, `labeled`,
`header_row`, `pill`, `section`, `stat`. See them all on one screen:

```sh
cargo run -p rime-demo
```

## What belongs here

Domain-free, reusable visual primitives. Anything that knows about *your* data —
a chart that understands your metrics, a per-category color scale — stays in your
app and composes rime primitives. See `COMPONENTS.md` for the contract a new
component must meet.

## Development

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p rime-demo        # the only real visual check
```

License: MIT OR Apache-2.0.
