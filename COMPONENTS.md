# Adding a component

A component earns a place in `rime` only if it is **domain-free** and **reused or
reusable**. If it would force a domain type (a `TrialSummary`, an `Order`) or a new
palette token into the kit, it belongs in the *app*, composing rime primitives —
not here. When in doubt, build it in your app first; promote it to rime the second
GUI that wants it.

## The contract

Every component meets all of these. They are what let a screen drop one in without
re-deriving padding, color, or radius — and what keep the kit portable.

1. **One file per component**, `src/widgets/<name>.rs`. The doc-comment opens with
   *what it is* and *when to reach for it* (not how it's built).
2. **Generic over the message type** `M`, with the *minimal* bound — add
   `M: Clone` only if iced clones the value (as `button` does for its press
   message); a display-only component needs no bound.
3. **Returns an `Element<'a, M>`** — or a concrete iced builder (`Button<'a, M>`,
   `TextInput<'a, M>`) when callers need to chain `.width()`/`.on_press()` etc.
   Never return a bare `Row`/`Column`.
4. **Colors come from `theme::tokens()` or an explicit argument** — never a
   hardcoded `Color`. Documented layout constants (radii, padding, text sizes) are
   fine and *should* live as named consts.
5. **Capture the palette into the draw closure.** Read `theme::tokens()` once at
   build time and move the captured `Palette` into any `move |_theme| …` style
   closure (see `card.rs`), so styling doesn't depend on *when* iced calls back.
6. **No app state, no I/O, no domain types.** A component takes data it defines
   itself (`&[TreeNode]`), not the app's structs. Expansion/selection/scroll state
   is owned by the *caller* — the kit stays stateless.
7. **Re-export it** from `rime/src/widgets/mod.rs` and **show it in the demo**
   (`demo/src/main.rs`). The demo is the only real visual test — a component
   that isn't in it is untested. (`demo/src/shot.rs`'s `RIME_DEMO_SHOT` env var
   can capture the demo to a PNG without a display, for mechanical regression
   diffing, but that doesn't replace a human looking at `cargo run -p
   rime-demo`.)

## The gate

Before committing a new component:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -p rime-demo        # look at it on both themes
```

The demo has a theme toggle: confirm the component re-colors correctly from the
palette alone (no hardcoded colors leaked).

**No inline test modules.** If a module `foo.rs` needs unit tests, put them in a
sibling `foo_tests.rs` and wire it up with `#[cfg(test)] #[path = "foo_tests.rs"] mod
tests;` (the test file opens `use super::*;`). Don't write `#[cfg(test)] mod tests
{ … }` inline in a source file — same rule as fed and tty.

## Worked example: a `tree`

Say you want a collapsible tree.

- **It qualifies**: a tree of labels with expand/collapse is domain-free and
  obviously reusable.
- **State stays with the caller**: the kit doesn't track which nodes are open.
  The component takes the nodes *and* their open-state, and reports toggles via a
  message — the app holds a `HashSet<NodeId>` and updates it.

```rust
// src/widgets/tree.rs
use iced::widget::{button, column, row, text, Space};
use iced::{Element, Length};
use crate::theme::{rounded, tokens};

/// One row in a [`tree`]. The caller owns the data and the open-state; rime only
/// draws and reports toggles.
pub struct Node<'a> {
    pub id: u64,
    pub label: &'a str,
    pub open: bool,
    pub children: Vec<Node<'a>>,
}

/// A collapsible tree. `on_toggle(id)` fires when a row's disclosure is clicked;
/// the caller flips that node's `open` and rebuilds.
pub fn tree<'a, M: Clone + 'a>(
    nodes: &'a [Node<'a>],
    on_toggle: impl Fn(u64) -> M + Copy + 'a,
) -> Element<'a, M> {
    let mut col = column![].spacing(2);
    for n in nodes {
        let glyph = if n.children.is_empty() {
            "  "
        } else if n.open {
            "▾ "
        } else {
            "▸ "
        };
        col = col.push(
            button(row![text(glyph), text(n.label)].spacing(4))
                .on_press(on_toggle(n.id))
                .style(rounded(button::text)),
        );
        if n.open && !n.children.is_empty() {
            // Indent children under their parent.
            col = col.push(
                row![Space::with_width(16.0), tree(&n.children, on_toggle)],
            );
        }
    }
    col.width(Length::Fill).into()
}
```

Then: re-export `pub use tree::{tree, Node as TreeNode};` from `widgets/mod.rs`,
add a small tree to the demo (`demo/src/main.rs`), and run the gate. Note it reads `tokens()` only
through `rounded` here; a richer tree drawing its own row backgrounds would capture
`tokens()` per rule 5. (`tokens` is imported above for that case.)

## When something is *almost* generic

`embate`'s `chart.rs` is the canonical example: a latency-vs-time canvas that knows
about p50/steps is **domain**. But the *plotting kernel* underneath — N series of
`(x, y)` points, axes, a legend — is generic and belongs in rime as a `chart`
component taking abstract series. Split the two: the generic half comes here, the
"what the axes mean" half stays in the app as a ~20-line adapter. That split is the
tell for every mixed case — extract the kernel, leave the meaning.
