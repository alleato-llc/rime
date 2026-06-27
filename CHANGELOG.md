# Changelog

All notable changes to **rime**. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); no tagged release has been cut yet,
so current work lives under **Unreleased**.

## [Unreleased]

### Added
- **`tooltip` widget** — wraps any element so hovering it reveals a short label in a
  surface-colored bubble (styled from the palette tokens, so it matches the active
  theme). Exported as `tooltip` plus `TooltipPosition` (re-exported from iced).
  Pairs naturally with `pill`: the chip says *what*, the tooltip says *what it
  means*. Shown in `rime-demo`.
- **`select` widget** — a single-select dropdown (a styled `pick_list` via
  `theme::pick_style`): `select(options, selected, Message::Pick)`. Shown in
  `rime-demo`.

### Changed
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
