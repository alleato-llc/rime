# Changelog

All notable changes to **rime**. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); no tagged release has been cut yet,
so current work lives under **Unreleased**.

## [Unreleased]

### Added
- **Theming machinery beyond the palette** (`theme` module), so a second GUI
  doesn't reinvent it — extracted from fed's `patina` when a second consumer
  appeared:
  - **`parse_color` / `color_hex`** — `#rrggbb`/`#rrggbbaa` ↔ `Color`.
  - **`Palette::color(key)` / `set(key, c)` + `PALETTE_KEYS`** — read/write tokens
    by name, for a theme editor's rows and `[ui]` serialization.
  - **`ThemeRegistry<T>` + the `NamedTheme` trait** — built-in themes plus user
    themes saved as TOML in a host-owned directory: list/resolve-by-name,
    save/delete/import/export, `is_builtin`. Domain-free: it delegates parsing to
    a `parse` fn and serialization to `NamedTheme`, so it works for any theme type
    (fed's chrome+editor+syntax `Theme`, or a palette-only one).
- **More chrome from the `fed` editor**, all stateless like the rest of the kit:
  - **`dialog`** — a `modal` preset: a titled surface card with a message and an
    action-button row (the alert/confirm shape). Backdrop click → dismiss.
  - **`banner`** — a dismissible inline notification strip (a message + a close
    affordance), for non-blocking status the host wants acknowledged.
  - **`context_menu`** — a right-click popup: floats `menu::Item`s at a `Point`
    over a base element; off-click emits dismiss. Reuses the `menu` item model.
  - **`color_field`** — a swatch + read-only hex readout + R/G/B/A sliders, the
    theme-editor control; `on_change` reports the edited `Color`.
  - **`status_bar`** — a footer bar that takes its left/right ends as `&str` and
    owns the typography (uniform size + muted color across apps), with a hairline
    separating it from the content above.
  - **`settings` footer slot** — `settings` gained an optional `footer` element
    pinned to the bottom of the left rail (e.g. an "Edit settings file…" action).
- **Chrome widgets for editor-style apps**, all stateless (the host owns the
  interaction state and passes it in, so one component backs several GUIs):
  - **`modal`** — `content` on a centered surface panel over a dimmed backdrop;
    clicking the backdrop emits a dismiss message.
  - **`menu_bar`** — top-level dropdown menus (`Menu` / `MenuItem`) rendered as a
    full-window layer. Items carry an optional shortcut hint; `MenuItem::submenu`
    opens a flyout to the right. The host owns the open-menu index (`Option<usize>`).
  - **`tabs`** — a document tab strip: active highlight, dirty dot, a close button
    that appears only on the hovered tab, an `on_background_press` hook (hosts use
    a double-click there to open a tab), and horizontal scrolling when tabs
    overflow. Host owns the active + hovered index.
  - **`settings`** — a settings-panel shell: a left-rail section nav beside a
    content pane over a backdrop. The host supplies the section names and the active
    section's body, so every app's settings share one layout.
- **`toggle`** — a labelled on/off switch row (`toggle(label, value, on_flip)`).
- **`stepper`** — a `label … [−] value [+]` numeric stepper; the host owns the value
  and formats it.
- **`tooltip` widget** — wraps any element so hovering it reveals a short label in a
  surface-colored bubble (styled from the palette tokens, so it matches the active
  theme). Exported as `tooltip` plus `TooltipPosition` (re-exported from iced).
  Pairs naturally with `pill`: the chip says *what*, the tooltip says *what it
  means*. Shown in `rime-demo`.
- **`select` widget** — a single-select dropdown (a styled `pick_list` via
  `theme::pick_style`): `select(options, selected, Message::Pick)`. Shown in
  `rime-demo`.

### Changed
- **`settings` panel is now responsive** — it fills the window (capped at
  1100×860, inset by a margin) instead of a fixed 680×460, so resizing the host
  window resizes the settings panel.
- **Chrome refinement** — the menu bar is a touch taller (30→34px) with larger
  titles and a bottom hairline; the footer (`status_bar`) is taller with a top
  hairline. Both bars now read as crisply separated strips.
- Documented the full component roster in `README.md` (it had also been missing
  `line_chart`) and noted `tooltip` / `select` in `CLAUDE.md`.

## Earlier (pre-changelog)

History before this file was started, newest first:

- **Workspace restructure** — split into the `rime` library crate and a `demo`
  crate (`rime-demo`, the component showcase).
- **`line_chart`** — a generic line-chart component.
- **Docs** — `CLAUDE.md` (working guidance) and `COMPONENTS.md` (the contract for
  adding a component).
- **Initial scaffold** — the consistent iced component kit: the nine-token palette
  channel and the first widgets (button, card, input, pill, stat, field, header,
  section).
