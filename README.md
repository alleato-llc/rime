# rime

A small, consistent component kit on top of [iced](https://iced.rs) — an
opinionated convenience layer (a facade) that trades iced's full flexibility for
consistency and a fraction of the call-site boilerplate.

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

## rime vs. raw iced

Every rime component is a thin builder over the exact iced you'd otherwise write
by hand — it just writes the styling, padding, and sizing once so your call sites
don't. The point is the diff:

**A button.** Raw iced makes you re-specify size, padding, and style at every call:

```rust
// raw iced
use iced::widget::{button, text};
button(text("Run").size(13))
    .on_press(Message::Run)
    .padding([7, 16])
    .style(rounded(button::primary))   // your own house style fn, threaded everywhere

// rime
use rime::widgets::button;
button::primary("Run", Message::Run)
```

**A card surface.** The raw version is a style closure you copy-paste onto every
container; rime bakes the border, radius, shadow, and palette-aware surface in:

```rust
// raw iced
use iced::widget::container;
use iced::{Border, Shadow};
container(content)
    .padding(16)
    .style(|_| container::Style {
        background: Some(surface.into()),
        border: Border { color: hairline, width: 1.0, radius: 12.0.into() },
        shadow: Shadow { /* offset, blur, palette-aware alpha … */ },
        ..Default::default()
    })

// rime
use rime::widgets::card;
card(content)
```

**A stat readout.** A big number over a muted caption — two `text`s, two sizes,
two palette colors, one `column`:

```rust
// raw iced
use iced::widget::{column, text};
column![
    text(value).size(22).color(ink),
    text(label).size(12).color(muted),
].spacing(2)

// rime
use rime::widgets::stat;
stat("p50", "12 ms".to_string())
```

The trade is deliberate: rime **narrows** iced's API. When a component's fixed
sizing/style is wrong for a call site, drop back to raw iced (or reach for the
lower-level seam the component is built on — `theme::rounded`, `theme::tokens`) —
rime returns plain `iced::Element`s, so the two compose freely in the same tree.
`iced` is re-exported as `rime::iced` so dependents pin one version.

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
`section`, `caption` (muted sub-heading), `shortcut_row` (chord + description
reference row), `rename_bar` (inline "rename this tab" field), `stat`,
`status_bar` (left/right footer bar), `line_chart`, `grid` (virtualized
spreadsheet grid — frozen row/column headers, anchor+extent selection rectangles,
a `fn(row, col) -> GridCell` factory, per-cell `Element` overlays for in-place
editors/controls, double-click activation, and per-column widths with
resize-drag), `bit_grid` (macOS-Calculator-style bit editor — labeled bit
buttons + colored `BitBand`s that own their label, so a host passes a
per-render decode like `owner rwx`; per-field editors are the host's job),
`tooltip`, `toggle` (switch row),
`stepper` (− value +).

**Composite / chrome** — `modal` (dimmed overlay panel), `dialog` (titled modal +
message + action-button row — the alert/confirm shape), `banner` (dismissible
notification strip), `context_menu` (right-click popup, floated at a point),
`menu_bar` (top-level dropdown menus with optional submenu flyouts, plus a
`_with_trailing` variant that pins a toolbar item to the right of the bar), `tabs`
(document tab strip with hover-reveal close + background-press hook), `title_strip`
(tab-bar-height header band: label + trailing controls) and `window_shell`
(title_strip + body + status_bar — the chrome of a torn-off / detached window), and
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

License: [MIT](LICENSE).
