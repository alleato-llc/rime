# CLAUDE.md

Guidance for working in this repo. Keep this file current as the project evolves.

## What this is

rime — a small, consistent **component kit on top of iced**. It owns the reusable
visual vocabulary (buttons, cards, inputs, …) and the theming machinery, so a host
GUI writes `button::primary("Run", msg)` and never re-styles a widget. It was
extracted from the `embate` load-tester's GUI and is meant to be shared by other
Rust GUIs.

`README.md` is the user-facing pitch + quickstart. `COMPONENTS.md` is the contract
for adding a component — **read it before adding one.**

**iced is pinned at 0.14.** Before writing widget/app iced code, read
[`ICED.md`](./ICED.md) — the patterns & gotchas (custom-`Widget` skeleton,
`application` boot fn, `Quad.snap`, `Text` `align_x/y`, `Space` builder, focus,
subscriptions, the 0.13→0.14 diff). Don't re-derive usage; consult it (and append
to it on the next version bump).

## Map

One crate, two modules.

- `theme` — the visual language. `Palette` (nine semantic tokens), the palette
  channel (`enter`/`scope` set it, `tokens()` reads it — a thread-local, because
  `view()` is synchronous and single-threaded), built-in palettes
  (`DRACULA`/`GITHUB`), `ThemeChoice`, parameterized persistence (`load`/`save`
  take the path — the *host* owns where), and the style functions
  (`rounded`/`input_style`/`pick_style`/`editor_style`). Split across
  `theme/{mod,palettes,style,registry}.rs`. `theme` also carries the domain-free
  theming *machinery* a multi-theme app needs: `parse_color`/`color_hex`,
  `Palette::color`/`set` + `PALETTE_KEYS`, and `registry.rs`'s generic
  `ThemeRegistry<T>` + `NamedTheme` trait (built-ins + user TOML themes in a
  host-owned dir; (de)serialization delegated to the app). Domain colors
  themselves (editor caret, syntax) stay in the *app*, never here.
- `widgets` — one primitive per file (`button`, `card`, `input`, `pill`, `stat`,
  `status_bar`, `field`/`labeled`, `header`, `section`, `chart`/`line_chart`,
  `select`, `slider`, `color_field`, `tooltip`, `toggle`, `stepper`, `modal`, `dialog`,
  `banner`, `context_menu`, `menu` (`menu_bar` / `menu_bar_with_trailing` + `Submenu`
  flyouts), `tabs`, `settings`, `grid` (virtualized spreadsheet — a custom advanced
  `Widget`: frozen headers, selection rects, per-cell `Element` overlays, per-column
  widths + resize-drag), `bit_grid` (bit editor)), each generic over the message
  type, stateless, drawing from
  `theme::tokens()`. The "chrome" widgets (`menu`/`tabs`/`settings`) are stateless
  too: the host owns open-menu / active-tab / hovered / active-section state and
  passes it in, so one component backs several apps.

`iced` is re-exported at the crate root (`rime::iced`) so dependents share one
version.

## Quality gate (run before every commit)

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings   # must be clean
cargo test                                  # must be green
cargo run -p rime-demo                       # the only real visual check
```

A GUI can't be verified headlessly, so `rime-demo` (the `demo/` crate) — every
component on one screen, with a theme toggle — *is* the visual test. Any new or changed
component must appear there, and must re-color correctly when the theme is toggled
(proof no hardcoded color leaked).

## Conventions

- **The palette channel is the one piece of global state.** The host opens it once
  per render (`let _scope = theme::enter(pal);`); components only ever *read* it via
  `theme::tokens()`. Never set the palette from inside a component. Capture
  `tokens()` into draw-time `move` closures so styling is independent of when iced
  calls back (see `card.rs`).
- **Fixed token vocabulary.** The nine `Palette` fields are the portability
  contract — a component renders under *any* palette. Don't add a token for one
  app's need; that color is the app's (domain) concern.
- **Domain-free only.** No app types, no I/O, no state. See `COMPONENTS.md`.
- Match the surrounding comment density: doc-comment public items, explain *why* in
  prose, not *what*. Each component file opens with what-it-is / when-to-reach-for.
- Commits: imperative subject + a body explaining the change.

## Using rime from a host app

- Depend on it (`rime = { path = "../rime" }`, or a version once published).
- Open the palette once at the top of `view()`; map your theme choice to the iced
  `Theme` via `ThemeChoice::theme()` or `Palette::iced_theme(name)` and pass it to
  the iced application builder.
- Keep your *domain* visuals (a metrics chart, a per-category color scale) in the
  app, composing rime primitives. `embate`'s `style.rs` (a ~45-line adapter holding
  only its persistence path + a per-step chart palette) is the reference for how
  thin the app side becomes.

## Gotchas

- **No headless visual check** — build + clippy + `cargo test` cover the layers
  under the pixels; the rendering needs `cargo run -p rime-demo` on a machine
  with a display.
- **iced 0.13.** Styling is `fn(&Theme, Status) -> SomeStyle`, not objects; the
  five-slot `iced::theme::Palette` is *separate* from rime's nine-token `Palette`
  (the built-in widgets follow the former, rime's components the latter — keep them
  in step via `ThemeChoice`/`iced_theme`).
- `Cargo.lock` is git-ignored (this is a library); pin versions in `Cargo.toml`.
