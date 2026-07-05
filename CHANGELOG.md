# Changelog

All notable changes to **rime**. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); no tagged release has been cut yet,
so current work lives under **Unreleased**.

## [Unreleased]

### Added
- **`LICENSE` file (MIT)** — the crate declared `MIT OR Apache-2.0` in metadata
  but shipped no license text. Now single-licensed **MIT**, with the file
  present, in preparation for making the repository public.
- **README "rime vs. raw iced" section** + inline `# Compared to raw iced`
  rustdoc blocks on `button`, `card`, and `stat` — concrete before/after diffs
  showing what boilerplate each component collapses, framing rime as an
  opinionated convenience layer (a facade) over iced.
- **CI** (`.github/workflows/ci.yml`) — fmt · clippy `-D warnings` · test ·
  release build of the whole workspace on every push/PR (Linux, with the
  iced/winit windowing dev deps the demo links against).
- **Theme-helper tests** — round-trip coverage for the public `parse_color` /
  `color_hex` pair, `Palette::color` / `set` by key + `PALETTE_KEYS`, and
  `builtin_themes()` catalog invariants (the domain-free machinery the README
  advertises to consumers). Lifts `theme/mod.rs` 32% → 82% and `palettes.rs`
  0% → 100%. (Overall crate coverage stays low by design: the rest is pure
  `view()`-building whose only meaningful check is the visual demo.)

### Changed
- **Whole workspace is now rustfmt-clean** (`grid.rs`, `slider.rs` had drifted)
  so the new CI fmt gate passes.
- **License is now MIT only** (was `MIT OR Apache-2.0` in metadata). `Cargo.toml`
  and the README footer updated to match.
- **`BitBand` owns its label** (`label: String`, was `&'a str`; the struct
  loses its lifetime parameter and `bit_grid` takes `Vec<BitBand>`).
  `BitBand::new` accepts `impl Into<String>`, so string literals are unchanged,
  but a host can now build a band label from per-render owned data (a decoded
  field readout like `owner rwx`) without fighting the borrow checker — the
  common case, since a host derives the layout from a value each frame.

### Added
- **`menu_bar_with_trailing`** — as `menu_bar`, but pins a caller-supplied
  element (a sidebar-toggle icon, …) to the right end of the bar strip, the way
  a macOS title bar carries a toolbar item on the right. `menu_bar` is now a thin
  wrapper that passes `None`. The bar's title row is vertically centered so the
  trailing element aligns with the menu titles.
- **`grid` per-column widths + resize-drag.** `grid(…).column_widths(vec)`
  overrides individual column widths (indexed by column; a short/absent/`0.0`
  entry falls back to `metrics.column_width`), and `.on_resize_column(|col,
  width| …)` fires while the user drags a column's right border in the header
  strip — the pointer shows the ↔ resize cursor over a border, the drag reports
  the new width already clamped to a 24px minimum, and the host stores it and
  feeds it back through `column_widths`. All the virtualization math (visible
  window, content size, hit-testing, overlay placement, header/selection
  geometry) now runs off a prefix-sum over the per-column widths; with no
  overrides it matches the old uniform arithmetic exactly. Built for the
  Rust/iced Soroban port's spreadsheet, domain-free.
- **`grid` hosted cell overlays + double-click activation.** The grid can host
  widgets over cells: `grid(…).overlay(row, col, element)` (called once per
  hosted cell; `.editor(…)` is an alias that reads clearly for the single
  focus-bearing text editor) lays each element out on top of its cell and
  forwards it events, focus (`operate`), and mouse-interaction — so a cell edits
  **in place** or hosts an interactive control (slider / checkbox / dropdown /
  stepper) inline, exactly where it lives. Clicks inside an overlay no longer
  move the selection. A new `.on_activate(|row, col| …)` fires on a
  **double-click** (400ms window, tracked in widget state). The grid gained the
  `Theme`/`Renderer` type params needed to hold child elements — they default to
  iced's, so existing leaf-grid call sites (`grid(rows, cols, cell)`) are
  unchanged. This is what the Rust/iced Soroban port needs for spreadsheet-style
  in-cell editing and inline controls; built here, domain-free.

### Changed
- **`grid::Selection::bounds()` is now public** — the inclusive
  `(row_min, row_max, col_min, col_max)` span (corners normalized), so a host
  can read a selection's rectangle for copy/paste or fill operations.
- **`slider` collapses its label gutter when the label is empty.** A non-empty
  label still reserves the fixed 170px gutter (so stacked sliders align); an
  empty label omits the gutter entirely, so the slider fits a tight space — e.g.
  hosted inside a spreadsheet cell. No change for labelled call sites.
- **`slider` takes an owned label (`impl Into<String>`)** instead of a borrowed
  `&'a str`, so a caller can pass a computed label (e.g. a control's name built
  per frame) without fighting the returned element's lifetime — matching its
  siblings `stepper`/`toggle`, which already own their labels. `&str` literals
  still work unchanged.
- **`tabs` opens a new tab on a *double*-click of the empty bar area**, not a single
  click — `on_background_press` now fires from the wrapping `mouse_area`'s
  `on_double_click`, matching the widget's long-documented intent so a stray single
  click never spawns a tab. Consumers that had hand-rolled their own double-click
  timing on top of the single-press signal (fed / fed-ide) can drop it.
- **`tabs` activates on press, not release** — each tab body is now a plain container
  and the wrapping `mouse_area`'s `on_press` fires `on_activate(i)` on mouse-*down*
  (an iced `button` only reports on mouse-*up*). This lets a host begin a drag gesture
  from the press — tab **tear-off** and **reorder** both arm on press and had silently
  no-op'd before, since the drag was only armed once the gesture had already ended. The
  `×` close button still captures its own press, and the strip's background-press hook
  is unchanged. No API or pixel change (container matches `button::text`).

### Added
- **`autocomplete_field` widget** (`autocomplete` module) —
  `autocomplete_field(placeholder, value, suggestions, highlighted, on_input,
  on_accept)`: a text input with a suggestion popup. Unlike iced's `combo_box`,
  it does **not** filter — the caller computes the candidates (an engine's
  completion pass, a fuzzy matcher, a history scan) and passes the finished
  `Vec<Suggestion>` plus which row is `highlighted`; the widget draws them and
  reports a click via `on_accept(index)`. Keyboard is the host's too (a
  single-line input ignores ↑/↓, so the host drives the highlight — the
  "suggestions when open, history when closed" dual role). `Suggestion` owns its
  text + optional dim hint. Shown in `rime-demo` (prefix-matching a function
  vocabulary). Generalizes `text_field`; fed/tty want this too.
- **`bit_grid` widget** (`bit_grid` module) — a macOS-Calculator-style bit
  editor: clickable bit cells that light up in their field's color when set,
  grouped into nibbles. `bit_grid(bits, bands, on_toggle)` takes the bits as a
  `Vec<bool>` (LSB-first, drawn high→low) and optional named `BitBand` ranges
  (a `[hi:lo]` legend below); clicking a cell emits `on_toggle(bit_index)`.
  Domain-free and stateless — the host owns the value and decodes it to bits (in
  Soroban the `BinaryView`/`BitFormat` model; the planned `rust/kit` for Tama,
  whose core this is). Shown in `rime-demo` as an editable RGB565 register.
  Enum/numeric field pickers are a planned follow-up.
- **`grid` widget** (`grid` module) — a virtualized spreadsheet grid, rime's first
  custom `Widget` (advanced API). `grid(rows, cols, cell_fn)` paints only the cells
  in view with `fill_quad`/`fill_text`, so cost is bounded by the viewport, not the
  logical size; frozen row/column headers, a `GridSelection` rectangle (anchor +
  extent, corners normalize), and a `GridCell` provider (`text` + `CellAlign` +
  optional per-cell colors) keep it domain-free. Stateless per the rime rule: scroll
  `offset` and the selection are caller-owned inputs — the wheel reports a new clamped
  offset via `on_scroll`, a click reports `(row, col, extend)` via `on_select`
  (`extend` = shift held); only the live modifiers are tracked in widget state.
  `Metrics` sets the default cell/header sizes (per-column widths + resize-drag
  landed later on the same viewport math — see the entry above). Shown in
  `rime-demo` (a 200×26 table).
  This is the grid the Rust/iced Soroban port needs; built here first, domain-free.
- **`rename_bar` widget** (`rename` module) — `rename_bar(caption, placeholder,
  value, on_change, on_submit)`: an inline "rename this tab" field (muted caption
  beside a focused `text_field` on the surface), plus `rename_field_id()` so the host
  can focus it on open. Extracted from tty's hand-rolled rename bar so tty and
  fed-ide's terminal-tab rename share one implementation. Enables the `advanced`
  iced feature (for the field's `widget::Id`). Shown in `rime-demo`.
- **`window_shell` + `title_strip` widgets** (`shell` module) — chrome for a
  torn-off / secondary window: `title_strip(label, controls)` is a surface band at
  `TAB_BAR_HEIGHT` (label left, trailing controls right) that lines up with the main
  window's tab strip; `window_shell(title, controls, body, status_left, status_right)`
  stacks that strip over a body and a `status_bar` footer on the window background.
  Extracted from three near-identical hand-rolled copies — tty's detached terminal
  window and fed-ide's detached **editor** and **terminal** windows — which had begun
  to drift (strip height/padding). Shown in `rime-demo`.
- **`shortcut_row` widget** (`shortcut` module) — `shortcut_row(keys, description)`:
  a fixed-width monospace chord cell beside a muted description, for keyboard-shortcut
  reference lists. Replaces copy-pasted `chord | description` rows in tty's keys panel
  and fed's / fed-ide's keymap settings (which had drifted to 150- vs 160-px gutters).
  Shown in `rime-demo`.
- **`caption` widget** (`section` module) — `caption(label)`: a muted 11px
  sub-heading, the small group label above settings rows. Replaces the `section_label`
  helper hand-rolled identically in both `fed` and `fed-ide` (rime's `section` is a
  larger inked heading, a different visual). Shown in `rime-demo`.
- **`tabs` `TabBarStyle` parameter** — `tabs(…, style: TabBarStyle)` takes a
  host-tunable `TabBarStyle { highlight_active, text_size }` (`Default` = the prior
  look: accent-inked active tab, 13px labels). `highlight_active: false` swaps the
  accent for normal ink (a subtler active emphasis); `text_size` sizes the labels.
  tty exposes the highlight as a setting and uses 12px tabs. **Breaking:** existing
  call sites must pass a `TabBarStyle` (use `TabBarStyle::default()` to keep the look).
- **`slider` widget** — a labelled value slider with a right-aligned readout
  (`slider(label, range, value, readout, on_change)`). It sets an explicit
  hundredth-of-range `.step()` so a fractional `0.0..=1.0` range is fully
  draggable (iced's default integer step otherwise snaps such a range to its
  endpoints). tty's unfocused-transparency control uses it. Shown in `rime-demo`.
- **Shared built-in palette catalog** (`theme` module) — named chrome-palette
  consts `DRACULA`, `NORD`, `GRUVBOX_DARK`, `SOLARIZED_DARK`, `SOLARIZED_LIGHT`,
  `GITHUB`, `NEON_NIGHTS`, `PHOSPHOR`, plus `builtin_themes()` returning the
  canonical ordered set. This makes the palette catalog the single source of truth
  so `fed` (`patina`) and `tty` present one identical theme list instead of each
  maintaining its own.
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
- **`tabs` gained a right-press hook + a published bar height.** A new
  `on_tab_right_press(usize) -> M` callback fires on right-click (or macOS
  ⌃-click) of a tab, so a host can anchor a context menu to it (fed-ide's tab
  close-menu). The strip's row height is now exported as `TAB_BAR_HEIGHT`, so a
  host can align an adjacent pane's top to the tab strip.
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
