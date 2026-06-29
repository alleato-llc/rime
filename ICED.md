# Using iced (patterns & gotchas)

The conventions for building on iced in **rime** and the GUIs that consume it
(fed, and future apps). It's a *patterns* reference, not a full API mirror — for
exhaustive detail see <https://docs.iced.rs>. Append a new section on each version
bump rather than rewriting; the goal is to never re-derive the same usage twice.

> **Current pin: iced 0.14.x.** (rime's `Cargo.toml` workspace dep; fed mirrors
> it.) When this changes, update this line and add a "diff" section at the bottom.

---

## Application wiring

`application` takes a **boot fn first** (builds initial state); the title is set
separately; there is no `run_with`.

```rust
iced::application(boot, update, view)   // boot: impl BootFn
    .title("My App")                    // or a fn(&State) -> String
    .theme(State::theme)
    .subscription(State::subscription)
    .window_size(iced::Size::new(900.0, 640.0))
    .run()
```

- **`BootFn` is `Fn` (not `FnOnce`).** If state is `Clone`, `move || State::new(arg.clone())`.
  If state is **not** `Clone` (consumed once), use a take-once cell so the closure
  stays `Fn`:
  ```rust
  let cfg = std::cell::RefCell::new(Some(cfg));
  iced::application(move || State::new(cfg.borrow_mut().take().expect("boot once")), …)
  ```
- `.title(...)` accepts a `&str`/`String` (static) or a `fn(&State) -> String`.

## Custom `Widget` (the advanced API)

The event method is **`update`**, taking the **event by reference** and returning
`()`; you signal "handled" with `shell.capture_event()`. `layout` takes `&mut self`.

```rust
impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for MyWidget {
    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node { … }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,                 // ← by reference
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {                                 // ← returns ()
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                // `delta` is `&ScrollDelta` (match ergonomics) → deref inner Copy fields
                let (dx, dy) = match delta {
                    mouse::ScrollDelta::Lines { x, y } => (*x, *y),
                    mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
                };
                shell.request_redraw();    // ← no args
                shell.capture_event();     // ← replaces `return event::Status::Captured`
            }
            _ => {}
        }
    }
}
```

- **`shell.request_redraw()`** takes no args (drop `RedrawRequest::NextFrame`).
- **`shell.capture_event()`** replaces returning `event::Status::Captured`; just
  *don't* call it to "ignore" (the old `event::Status::Ignored`).
- `Layout::children()` returns an `ExactSizeIterator`; `Overlay::is_over` is gone.

## Renderer `Quad`

Gained a **`snap: bool`** field (snap bounds to the pixel grid — `true` for crisp
1–2px UI elements like carets/borders/scrollbars):

```rust
renderer.fill_quad(
    Quad { bounds, border, shadow: Shadow::default(), snap: true },
    color,
);
```

## `Text` (advanced)

Alignment fields renamed: **`horizontal_alignment`/`vertical_alignment` →
`align_x`/`align_y`**, and `align_x` is a **`text::Alignment`** (not
`alignment::Horizontal`):

```rust
Text {
    // …
    align_x: text::Alignment::Left,        // was horizontal_alignment: alignment::Horizontal::Left
    align_y: alignment::Vertical::Top,     // unchanged type
}
```

## `Space`

Builder-style now (the `with_width`/`with_height`/2-arg `new` constructors are gone):

```rust
Space::new().width(Length::Fill)
Space::new().width(w).height(h)
// or the helpers: iced::widget::horizontal_space(), vertical_space()
```

## Widget focus / ids

- Widget ids are **`iced::advanced::widget::Id`** (e.g. `text_input::Id` no longer
  exists): `iced::advanced::widget::Id::new("my-input")`.
- Focus a widget by id with **`iced::widget::operation::focus(id) -> Task`**
  (the old `text_input::focus(id)` is gone).

## Subscriptions

A keyed stream subscription is **`Subscription::run_with(data, builder)`** — `data`
is hashed for identity (replaces the old explicit id string), and `builder` is a
**non-capturing fn pointer** `fn(&D) -> impl Stream`:

```rust
Subscription::run_with(root, |root: &PathBuf| {
    let root = root.clone();           // clone out of &data before moving into the stream
    iced::stream::channel(64, move |mut output: mpsc::Sender<Message>| async move { … })
})
```

- `Subscription::run(fn() -> impl Stream)` for the no-data case.
- Annotate the `channel` sender type (`mpsc::Sender<Message>`) — inference often
  needs it now.
- `rich_text` span vectors often need an explicit element type:
  `let mut spans: Vec<iced::advanced::text::Span<'_, Message>> = Vec::new();`

## Theme palette

`iced::theme::Palette` gained a **`warning`** slot (between `success` and
`danger`). rime maps its `warn` token there in `Palette::iced_theme`.

---

## 0.13 → 0.14 diff (quick index)

| 0.13 | 0.14 |
|------|------|
| `application(title, update, view).run_with(\|\| state)` | `application(boot, update, view).title(t).run()` |
| `Widget::on_event(self, event: Event, …) -> event::Status` | `Widget::update(self, event: &Event, …)` + `shell.capture_event()` |
| `Widget::layout(&self, …)` | `Widget::layout(&mut self, …)` |
| `shell.request_redraw(RedrawRequest::NextFrame)` | `shell.request_redraw()` |
| `Quad { bounds, border, shadow }` | `Quad { …, snap: bool }` |
| `Text { horizontal_alignment, vertical_alignment }` | `Text { align_x: text::Alignment, align_y }` |
| `Space::with_width(x)` / `Space::new(w, h)` | `Space::new().width(x).height(h)` |
| `text_input::Id` | `iced::advanced::widget::Id` |
| `text_input::focus(id)` | `iced::widget::operation::focus(id)` |
| `Subscription::run_with_id(id, stream)` | `Subscription::run_with(data, fn(&data) -> stream)` |
| `iced::theme::Palette { background, text, primary, success, danger }` | `+ warning` |
