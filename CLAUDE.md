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

One crate, three modules.

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
  widths + resize-drag), `bit_grid` (bit editor), `autocomplete` (`autocomplete_field`
  text input + popup, plus a standalone `suggestion_list` for hosts that float the
  popup above the input instead of below), `rename` (`rename_bar`, an inline
  "rename this tab" field), `shortcut` (`shortcut_row`, a chord + description
  reference row), `secure_input` (masked password entry over a caller-owned
  `SecretHandle` — a fixed-capacity, mlock'd, zeroized buffer; the secret never
  enters the message queue or the text shaper)), each generic over the message
  type, stateless, drawing from `theme::tokens()`. The "chrome" widgets (`menu`/`tabs`/`settings`) are stateless
  too: the host owns open-menu / active-tab / hovered / active-section state and
  passes it in, so one component backs several apps. `tabs` keeps even drag-reorder
  host-side — it exposes `Reorder` (a `begin`/`drag_to`/`end` tracker) + `reorder_slice`
  so the host applies each move to its own collection, and `TabBarStyle.filled` toggles
  the raised-band vs blended-into-its-container look.
- `icons` — a small embedded icon font (a Lucide subset, ISC-licensed), so hosts
  get glyphs that always render instead of depending on the platform's emoji/PUA
  coverage. A host loads `icons::FONT_BYTES` once via `.font(..)` on the iced
  application builder; `widgets::button::icon(icons::glyph::X, on_press)` then
  renders a borderless icon button. Add more glyphs by re-subsetting the upstream
  font and extending `icons::glyph`.

`iced` is re-exported at the crate root (`rime::iced`) so dependents share one
version.

## Quality gate (run before every commit)

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings   # must be clean
cargo test                                  # must be green
cargo run -p rime-demo                       # the only real visual check
```

Visual *judgment* can't be automated, so `rime-demo` (the `demo/` crate) — every
component on one screen, with a theme toggle — *is* the visual test. Any new or changed
component must appear there, and must re-color correctly when the theme is toggled
(proof no hardcoded color leaked). `RIME_DEMO_SHOT=<path> cargo run -p rime-demo`
captures it to a PNG without a display for mechanical regression diffing, but
doesn't replace looking at it.

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

- **No headless *judgment* check** — build + clippy + `cargo test` cover the
  layers under the pixels, but confirming a component actually *looks* right
  (and re-colors correctly on theme toggle) needs a human looking at
  `cargo run -p rime-demo`. `demo/src/shot.rs` can capture the demo to a PNG
  headlessly (`RIME_DEMO_SHOT=<path> cargo run -p rime-demo`, via iced's own
  wgpu texture readback — no display or screen-recording permission needed)
  for regression diffing, but that's a mechanical capture, not a substitute
  for eyes on the render.
- **Content taller than the window needs `scrollable`.** Without one, iced
  doesn't just let a `Length::Shrink` column overflow past the window edge —
  past some height it silently stops rendering the remaining children. See
  the "Content taller than the window needs `scrollable`" section in
  [`ICED.md`](./ICED.md).
- **iced 0.14.** Styling is `fn(&Theme, Status) -> SomeStyle`, not objects; the
  five-slot `iced::theme::Palette` is *separate* from rime's nine-token `Palette`
  (the built-in widgets follow the former, rime's components the latter — keep them
  in step via `ThemeChoice`/`iced_theme`).
- `Cargo.lock` is git-ignored (this is a library); pin versions in `Cargo.toml`.
