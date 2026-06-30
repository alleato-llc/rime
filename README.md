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

### Beyond the palette: a user-theme registry

For an app that ships several named themes and lets users add their own, `theme`
also provides the domain-free *machinery* so you don't reinvent it:

- `parse_color` / `color_hex` — `#rrggbb`(`aa`) ↔ `Color`.
- `Palette::color(key)` / `set(key, c)` + `PALETTE_KEYS` — read/write tokens by name
  (for a theme-editor's rows and `[ui]` serialization).
- `ThemeRegistry<T>` + the `NamedTheme` trait — built-ins plus user themes saved as
  TOML in a directory *you* own: list, resolve-by-name, save/delete/import/export,
  `is_builtin`. It delegates parsing (a `parse` fn you pass) and serialization
  (`NamedTheme::to_toml`) to you, so it's generic over *your* theme type — whether
  that's a palette plus syntax colors, or just a palette.
- `builtin_themes()` + the named palette consts (`DRACULA`, `NORD`, `GRUVBOX_DARK`,
  `SOLARIZED_DARK`, `SOLARIZED_LIGHT`, `GITHUB`, `NEON_NIGHTS`, `PHOSPHOR`) — the
  canonical chrome-palette catalog shared by every consumer, so `fed` and `tty` offer
  one identical theme list instead of each maintaining its own.

## Components

**Primitives** — `button` (primary/secondary/danger/ghost), `card`, `text_field`,
`labeled`, `select` (dropdown), `slider` (labelled value slider with a readout),
`color_field` (swatch + hex readout + R/G/B/A sliders), `header_row`, `pill`,
`section`, `stat`, `status_bar` (left/right footer bar), `line_chart`, `tooltip`,
`toggle` (switch row), `stepper` (− value +).

**Composite / chrome** — `modal` (dimmed overlay panel), `dialog` (titled modal +
message + action-button row — the alert/confirm shape), `banner` (dismissible
notification strip), `context_menu` (right-click popup, floated at a point),
`menu_bar` (top-level dropdown menus with optional submenu flyouts), `tabs`
(document tab strip with hover-reveal close + background-press hook), and
`settings` (a left-rail section shell apps fill with their own controls, with an
optional pinned footer slot).

These are stateless: the host owns selection / open / hover / active state and
passes it in, so the same component backs multiple apps. See them all on one
screen:

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
