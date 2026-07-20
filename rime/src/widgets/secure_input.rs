//! A masked password input whose secret never enters iced. Reach for it when a
//! security-conscious host needs password entry; for ordinary text, use
//! [`text_field`](crate::widgets::text_field).
//!
//! # Why not `text_input(..).secure(true)`?
//!
//! iced's `TextInput` is a controlled widget: the value round-trips as a plain
//! `String` through the message queue on every keystroke, and iced clones it
//! into internal grapheme/paragraph buffers every frame. None of those copies
//! are wiped and nothing is locked out of swap, so a password typed into it is
//! scattered across the heap for the allocator to reuse. `secure_input` keeps
//! the secret in exactly one app-owned buffer — a [`SecretHandle`] — that is
//! fixed-capacity (never reallocated, so no stale copy is left behind by a
//! `realloc`), `mlock`'d into RAM best-effort (out of swap), and zeroized on
//! drop. The widget mutates that buffer in place during event handling and
//! emits only *unit* messages (`on_edit` / `on_submit`); the secret bytes never
//! enter the message queue, the widget tree, or the text shaper (the field
//! renders one uniform mask bullet per character, drawn as plain quads).
//!
//! # Deliberate omissions
//!
//! - **No copy-out** (Cmd/Ctrl+C/X do nothing): exporting a password to the
//!   system clipboard is an anti-feature — every clipboard listener would see
//!   it, and the copy cannot be wiped.
//! - **No selection**: selection exists to support copy; without copy it is
//!   dead weight, so cursor motion is plain (arrows / Home / End).
//! - **No reveal toggle**: showing the real characters would push the secret
//!   into iced's text shaper, exactly the leak this widget exists to avoid. A
//!   future opt-in could accept that trade explicitly.
//!
//! Paste *in* (Cmd/Ctrl+V) is supported — pasting from a password manager is
//! the healthy flow — and the intermediate clipboard `String` is zeroized
//! best-effort after its characters are moved into the buffer.
//!
//! # Honest limitations
//!
//! The same residual risk class a locked, wiped CLI password buffer accepts:
//! the OS keyboard/IME path and the compositor see keystrokes before this
//! widget does; winit's event structs briefly hold the typed character; a paste
//! source keeps its own copy in the system clipboard; and a debugger or core
//! dump can read the buffer while it is live. What the widget removes is the
//! *long-lived, unwiped, swappable* copies inside the GUI toolkit.
//!
//! The lock is also best-effort in a second sense: `mlock` can be refused
//! outright (by `RLIMIT_MEMLOCK`, or in a container), in which case the buffer
//! still works and is still wiped, it is simply swappable. Hibernation writes
//! locked pages to disk regardless of any `mlock`.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout, Limits, Node};
use iced::advanced::text::{Renderer as _, Text};
use iced::advanced::widget::{operation, tree, Id, Operation, Tree};
use iced::advanced::{clipboard, renderer, Clipboard, Renderer as _, Shell, Widget};
use iced::widget::text_input;
use iced::{
    alignment, keyboard, mouse, window, Border, Element, Event, Length, Padding, Point, Rectangle,
    Shadow, Size,
};
use zeroize::Zeroize;

use crate::theme::input_style;

/// The fixed capacity of a [`SecretHandle`]'s buffer, in bytes (UTF-8). Appends
/// beyond it are rejected rather than growing the buffer, because a `realloc`
/// would copy the secret to a new allocation and leave the old bytes unwiped.
pub const SECRET_CAPACITY: usize = 4096;

/// Layout mirrors `text_field`: `[8, 10]` padding around a default-size line.
const PADDING: [u16; 2] = [8, 10];
/// Horizontal advance of one mask bullet, in logical pixels (uniform width, so
/// cursor math is arithmetic, not shaping).
const BULLET_ADVANCE: f32 = 11.0;
/// Diameter of one mask bullet, in logical pixels.
const BULLET_DIAMETER: f32 = 7.0;
/// The caret blinks on this period while focused (mirrors iced's `TextInput`).
const CURSOR_BLINK_INTERVAL_MILLIS: u128 = 500;

/// The single app-owned home of a secret: a fixed-capacity heap buffer that is
/// `mlock`'d into RAM at creation (best-effort — creation still succeeds if the
/// OS refuses the lock, and the bytes are wiped regardless) and zeroized on
/// drop. It never reallocates; see [`SECRET_CAPACITY`].
///
/// Cloning the handle is cheap and shares the same buffer (`Arc`); the host
/// keeps one in its state and passes clones to [`secure_input`] each `view`.
/// All access goes through the internal lock, so a background job may read the
/// bytes (via [`with_bytes`](Self::with_bytes)) while the UI holds the handle.
#[derive(Clone, Default)]
pub struct SecretHandle(Arc<Mutex<SecretBuf>>);

impl SecretHandle {
    /// A new, empty, locked (best-effort) secret buffer.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, SecretBuf> {
        // A poisoned lock only means another thread panicked mid-access; the
        // buffer itself (plain bytes + lengths) cannot be left inconsistent in
        // a way worth propagating the panic for.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The number of *characters* held (what the widget renders as bullets).
    /// The byte length is visible via [`with_bytes`](Self::with_bytes).
    pub fn len(&self) -> usize {
        self.lock().char_len
    }

    /// Whether the buffer holds no secret.
    pub fn is_empty(&self) -> bool {
        self.lock().byte_len == 0
    }

    /// Zeroize the occupied bytes and reset to empty.
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Run `f` over the secret's UTF-8 bytes while holding the lock. The guard
    /// scope is explicit: the slice is only valid inside `f`, so no unwiped
    /// copy can outlive the call unless `f` makes one on purpose (a caller
    /// handing the bytes to a KDF should copy into its own wiped buffer).
    pub fn with_bytes<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        let buf = self.lock();
        f(&buf.bytes()[..buf.byte_len])
    }

    /// Insert `ch` before character `index` (clamped to the end). Returns
    /// `false` — leaving the buffer unchanged — if the encoded character does
    /// not fit in the remaining capacity.
    pub fn insert(&self, index: usize, ch: char) -> bool {
        self.lock().insert(index, ch)
    }

    /// Remove the character at `index`. Returns `false` if out of range. The
    /// vacated tail bytes are zeroized.
    pub fn remove(&self, index: usize) -> bool {
        self.lock().remove(index)
    }

    /// Append `s`, all or nothing: returns `false` — leaving the buffer
    /// unchanged — if it does not fit. For hosts seeding a password
    /// programmatically (tests, screenshot harnesses); wipe your source copy
    /// after the call if it is itself secret.
    pub fn push_str(&self, s: &str) -> bool {
        self.lock().push_str(s)
    }
}

/// The buffer behind [`SecretHandle`]: `SECRET_CAPACITY` heap bytes, a length,
/// a character count, and the best-effort `mlock` guard.
///
/// `mlock`/`munlock` act on whole pages, so a buffer that merely *sat* on some
/// page would drag its neighbours into the locked region, and unlocking on drop
/// would strip protection from whatever else shared those pages (plausibly
/// another secret). Instead the storage is over-allocated by one page and the
/// secret lives in a page-aligned, whole-page-sized *window* inside it, which no
/// other allocation can overlap. Doing it this way (rather than a custom
/// `Layout`) keeps the crate free of `unsafe`.
///
/// Field order matters: `_guard` is declared first, so it drops *before*
/// `storage` and `munlock` runs while the pages are still mapped.
struct SecretBuf {
    /// Keeps the window's pages locked into RAM (out of swap) while it lives.
    /// `None` when the OS refused the lock; everything else still works.
    _guard: Option<region::LockGuard>,
    /// The backing allocation: `window` bytes of usable space plus up to one
    /// page of leading slack. Allocated once, never grown.
    storage: Box<[u8]>,
    /// Offset of the page-aligned window within `storage`.
    offset: usize,
    /// Length of the window, in bytes: `SECRET_CAPACITY` rounded up to a whole
    /// number of pages. At least `SECRET_CAPACITY`.
    window: usize,
    /// Occupied prefix, in bytes.
    byte_len: usize,
    /// Occupied prefix, in characters (tracked so the widget's per-character
    /// cursor math never rescans under the render path).
    char_len: usize,
}

impl Default for SecretBuf {
    fn default() -> Self {
        let page = region::page::size();
        let window = SECRET_CAPACITY.next_multiple_of(page);
        let storage = vec![0u8; page + window].into_boxed_slice();
        // `align_offset` returns a value below `page` whenever alignment is
        // achievable, which it always is for a byte pointer and a power-of-two
        // page size. The guard is belt-and-braces: an unaligned window is still
        // correct, it just gives up the page-exclusivity property.
        let offset = storage.as_ptr().align_offset(page);
        let offset = if offset < page { offset } else { 0 };
        let guard = region::lock(storage[offset..].as_ptr(), window).ok();
        SecretBuf {
            _guard: guard,
            storage,
            offset,
            window,
            byte_len: 0,
            char_len: 0,
        }
    }
}

impl Drop for SecretBuf {
    fn drop(&mut self) {
        // Wipe the whole allocation, not just the occupied prefix, so bytes
        // from any earlier, longer secret are gone too. `Drop::drop` runs
        // before any field drops, so the pages are still locked and mapped
        // while they are cleared.
        self.storage.zeroize();
    }
}

impl SecretBuf {
    /// The page-aligned window the secret lives in.
    fn bytes(&self) -> &[u8] {
        &self.storage[self.offset..self.offset + self.window]
    }

    /// The page-aligned window the secret lives in, mutably.
    fn bytes_mut(&mut self) -> &mut [u8] {
        let (offset, window) = (self.offset, self.window);
        &mut self.storage[offset..offset + window]
    }

    /// The byte offset of character `index` (equal to `len` at/past the end).
    fn byte_of_char(&self, index: usize) -> usize {
        core::str::from_utf8(&self.bytes()[..self.byte_len])
            .expect("buffer holds only whole UTF-8 characters")
            .char_indices()
            .nth(index)
            .map_or(self.byte_len, |(offset, _)| offset)
    }

    fn clear(&mut self) {
        let byte_len = self.byte_len;
        self.bytes_mut()[..byte_len].zeroize();
        self.byte_len = 0;
        self.char_len = 0;
    }

    fn insert(&mut self, index: usize, ch: char) -> bool {
        let mut encoded = [0u8; 4];
        let encoded = ch.encode_utf8(&mut encoded).as_bytes();
        if self.byte_len + encoded.len() > SECRET_CAPACITY {
            return false;
        }
        let at = self.byte_of_char(index.min(self.char_len));
        let byte_len = self.byte_len;
        // Shift the tail right in place (within the one allocation), then drop
        // the character into the gap.
        let window = self.bytes_mut();
        window.copy_within(at..byte_len, at + encoded.len());
        window[at..at + encoded.len()].copy_from_slice(encoded);
        self.byte_len += encoded.len();
        self.char_len += 1;
        true
    }

    fn remove(&mut self, index: usize) -> bool {
        if index >= self.char_len {
            return false;
        }
        let at = self.byte_of_char(index);
        let end = self.byte_of_char(index + 1);
        let removed = end - at;
        let byte_len = self.byte_len;
        let window = self.bytes_mut();
        window.copy_within(end..byte_len, at);
        // The shift left leaves a stale copy of the tail's last bytes; wipe it.
        window[byte_len - removed..byte_len].zeroize();
        self.byte_len -= removed;
        self.char_len -= 1;
        true
    }

    fn push_str(&mut self, s: &str) -> bool {
        if self.byte_len + s.len() > SECRET_CAPACITY {
            return false;
        }
        let byte_len = self.byte_len;
        self.bytes_mut()[byte_len..byte_len + s.len()].copy_from_slice(s.as_bytes());
        self.byte_len += s.len();
        self.char_len += s.chars().count();
        true
    }
}

/// Non-secret, widget-internal UI state. The secret itself lives only in the
/// caller's [`SecretHandle`].
#[derive(Default)]
struct State {
    focus: Option<Focus>,
    /// Caret position in characters; clamped to the live length at use, since
    /// the host can mutate the handle (e.g. `clear`) between renders.
    cursor: usize,
    hovered: bool,
}

/// Focus bookkeeping for the blinking caret (mirrors iced's `TextInput`).
struct Focus {
    updated_at: Instant,
    now: Instant,
    is_window_focused: bool,
}

impl Focus {
    fn now() -> Self {
        let now = Instant::now();
        Focus {
            updated_at: now,
            now,
            is_window_focused: true,
        }
    }
}

impl operation::Focusable for State {
    fn is_focused(&self) -> bool {
        self.focus.is_some()
    }

    fn focus(&mut self) {
        self.focus = Some(Focus::now());
        // To the end; clamped against the live length wherever it is used.
        self.cursor = usize::MAX;
    }

    fn unfocus(&mut self) {
        self.focus = None;
    }
}

/// A masked password field over a caller-owned [`SecretHandle`]. Build it with
/// [`secure_input`]; it converts into an [`Element`] via `From`.
pub struct SecureInput<'a, M> {
    placeholder: &'a str,
    secret: SecretHandle,
    on_edit: M,
    on_submit: M,
    width: Length,
    id: Option<Id>,
}

/// A masked password input bound to `secret`, emitting the unit message
/// `on_edit` after any mutation (insert, delete, paste) and `on_submit` on
/// Enter. The secret never leaves the handle: the field renders one uniform
/// bullet per character and the messages carry no text. `placeholder` (plain,
/// non-secret text) shows while the field is empty and unfocused. Styled like
/// [`text_field`](crate::widgets::text_field) so the two sit in one form
/// seamlessly.
pub fn secure_input<'a, M: Clone>(
    placeholder: &'a str,
    secret: &SecretHandle,
    on_edit: M,
    on_submit: M,
) -> SecureInput<'a, M> {
    SecureInput {
        placeholder,
        secret: secret.clone(),
        on_edit,
        on_submit,
        width: Length::Fill,
        id: None,
    }
}

impl<M> SecureInput<'_, M> {
    /// Set the width. Defaults to `Length::Fill`.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Tag the field with a widget [`Id`], so a host can focus it with
    /// `iced::widget::operation::focus(id)`.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// The character index nearest to `x` (relative to the text area's left
    /// edge, already offset-corrected), clamped to `0..=len`.
    fn char_index_at(x: f32, len: usize) -> usize {
        if x <= 0.0 {
            return 0;
        }
        (((x / BULLET_ADVANCE) + 0.5).floor() as usize).min(len)
    }

    /// How far the bullet strip is scrolled left so the caret stays in view.
    fn scroll_offset(text_bounds: Rectangle, cursor: usize, len: usize, focused: bool) -> f32 {
        if !focused {
            return 0.0;
        }
        let caret_x = cursor.min(len) as f32 * BULLET_ADVANCE;
        let content = len as f32 * BULLET_ADVANCE;
        let max = (content - text_bounds.width).max(0.0);
        (caret_x - text_bounds.width + BULLET_ADVANCE)
            .max(0.0)
            .min(max)
    }
}

impl<M: Clone> Widget<M, iced::Theme, iced::Renderer> for SecureInput<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(&mut self, _tree: &mut Tree, renderer: &iced::Renderer, limits: &Limits) -> Node {
        // Mirror `text_input`'s single-line layout with `text_field`'s padding:
        // a text node inset by the padding, sized to one default-size line.
        let text_size = renderer.default_size();
        let padding = Padding::from(PADDING).fit(Size::ZERO, limits.max());
        let height = iced::widget::text::LineHeight::default().to_absolute(text_size);

        let limits = limits.width(self.width).shrink(padding);
        let text_bounds = limits.resolve(self.width, height, Size::ZERO);

        let text_node =
            layout::Node::new(text_bounds).move_to(Point::new(padding.left, padding.top));
        Node::with_children(text_bounds.expand(padding), vec![text_node])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.focusable(self.id.as_ref(), layout.bounds(), state);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(bounds) else {
                    // A click elsewhere blurs the field.
                    if state.focus.take().is_some() {
                        shell.request_redraw();
                    }
                    return;
                };
                let len = self.secret.len();
                let text_bounds = layout.children().next().expect("text child").bounds();
                let was_focused = state.focus.is_some();
                let offset = Self::scroll_offset(text_bounds, state.cursor, len, was_focused);
                state.focus = Some(Focus::now());
                state.cursor = Self::char_index_at(position.x - text_bounds.x + offset, len);
                shell.request_redraw();
                shell.capture_event();
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hovered = cursor.is_over(bounds);
                if state.hovered != hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
            }

            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modified_key,
                physical_key,
                modifiers,
                text,
                ..
            }) => {
                let Some(focus) = &mut state.focus else {
                    return;
                };
                let len = self.secret.len();
                state.cursor = state.cursor.min(len);

                // Command shortcuts first, as `text_input` orders them.
                match key.to_latin(*physical_key) {
                    // Paste in: the one clipboard direction a password field
                    // wants. The intermediate `String` is wiped best-effort.
                    Some('v') if modifiers.command() && !modifiers.alt() => {
                        let mut content = clipboard
                            .read(clipboard::Kind::Standard)
                            .unwrap_or_default();
                        let mut edited = false;
                        for ch in content.chars().filter(|c| !c.is_control()) {
                            if !self.secret.insert(state.cursor, ch) {
                                break; // full: saturate, keep what fit
                            }
                            state.cursor += 1;
                            edited = true;
                        }
                        content.zeroize();
                        if edited {
                            focus.updated_at = Instant::now();
                            shell.publish(self.on_edit.clone());
                            shell.request_redraw();
                        }
                        shell.capture_event();
                        return;
                    }
                    // Copy-out and select-all are deliberately dead (see the
                    // module docs); swallow them so nothing else reacts.
                    Some('c') | Some('x') | Some('a') if modifiers.command() => {
                        shell.capture_event();
                        return;
                    }
                    _ => {}
                }

                // Printable input.
                if !modifiers.command() {
                    if let Some(ch) = text
                        .as_ref()
                        .and_then(|t| t.chars().next())
                        .filter(|c| !c.is_control())
                    {
                        if self.secret.insert(state.cursor, ch) {
                            state.cursor += 1;
                            focus.updated_at = Instant::now();
                            shell.publish(self.on_edit.clone());
                            shell.request_redraw();
                        }
                        shell.capture_event();
                        return;
                    }
                }

                match modified_key.as_ref() {
                    keyboard::Key::Named(keyboard::key::Named::Enter) => {
                        shell.publish(self.on_submit.clone());
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::Backspace) => {
                        if state.cursor > 0 {
                            state.cursor -= 1;
                            if self.secret.remove(state.cursor) {
                                focus.updated_at = Instant::now();
                                shell.publish(self.on_edit.clone());
                                shell.request_redraw();
                            }
                        }
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::Delete) => {
                        if self.secret.remove(state.cursor) {
                            focus.updated_at = Instant::now();
                            shell.publish(self.on_edit.clone());
                            shell.request_redraw();
                        }
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::Home) => {
                        state.cursor = 0;
                        focus.updated_at = Instant::now();
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::End) => {
                        state.cursor = len;
                        focus.updated_at = Instant::now();
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                        state.cursor = if modifiers.macos_command() {
                            0
                        } else {
                            state.cursor.saturating_sub(1)
                        };
                        focus.updated_at = Instant::now();
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                        state.cursor = if modifiers.macos_command() {
                            len
                        } else {
                            (state.cursor + 1).min(len)
                        };
                        focus.updated_at = Instant::now();
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    keyboard::Key::Named(keyboard::key::Named::Escape) => {
                        // Blur, like `text_input`. The secret is kept: Escape
                        // is "leave the field", not "discard my password".
                        state.focus = None;
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    _ => {}
                }
            }

            Event::Window(window::Event::Unfocused) => {
                if let Some(focus) = &mut state.focus {
                    focus.is_window_focused = false;
                }
            }
            Event::Window(window::Event::Focused) => {
                if let Some(focus) = &mut state.focus {
                    focus.is_window_focused = true;
                    focus.updated_at = Instant::now();
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                // Keep the caret blinking: schedule the next redraw at the next
                // blink boundary (same cadence as `text_input`).
                if let Some(focus) = &mut state.focus {
                    if focus.is_window_focused {
                        focus.now = *now;
                        let millis_until_redraw = CURSOR_BLINK_INTERVAL_MILLIS
                            - (*now - focus.updated_at).as_millis() % CURSOR_BLINK_INTERVAL_MILLIS;
                        shell.request_redraw_at(
                            *now + Duration::from_millis(millis_until_redraw as u64),
                        );
                    }
                }
            }

            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let text_bounds = layout.children().next().expect("text child").bounds();
        let len = self.secret.len();
        let is_hovered = cursor.is_over(bounds);

        // Same style fn as `text_field`, so background/border/focus ring/
        // placeholder/value colors match it under every theme.
        let status = match &state.focus {
            Some(_) => text_input::Status::Focused { is_hovered },
            None if is_hovered => text_input::Status::Hovered,
            None => text_input::Status::Active,
        };
        let style = input_style(theme, status);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                shadow: Shadow::default(),
                snap: true,
            },
            style.background,
        );

        let focus = state.focus.as_ref().filter(|focus| focus.is_window_focused);

        if len == 0 && focus.is_none() && !self.placeholder.is_empty() {
            renderer.fill_text(
                Text {
                    content: self.placeholder.to_string(),
                    bounds: text_bounds.size(),
                    size: renderer.default_size(),
                    line_height: iced::widget::text::LineHeight::default(),
                    font: renderer.default_font(),
                    align_x: iced::advanced::text::Alignment::Left,
                    align_y: alignment::Vertical::Center,
                    shaping: iced::advanced::text::Shaping::Advanced,
                    wrapping: iced::advanced::text::Wrapping::None,
                },
                Point::new(text_bounds.x, text_bounds.center_y()),
                style.placeholder,
                text_bounds,
            );
        }

        let caret = state.cursor.min(len);
        let offset = Self::scroll_offset(text_bounds, caret, len, focus.is_some());

        // One uniform bullet per character — plain quads, never glyphs, so the
        // secret's bytes stay out of the shaper. Clipped to the text area.
        if len > 0 {
            renderer.with_layer(text_bounds, |renderer| {
                let first = (offset / BULLET_ADVANCE).floor() as usize;
                let last =
                    (((offset + text_bounds.width) / BULLET_ADVANCE).ceil() as usize).min(len);
                for i in first..last {
                    let x = text_bounds.x + i as f32 * BULLET_ADVANCE - offset
                        + (BULLET_ADVANCE - BULLET_DIAMETER) / 2.0;
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle {
                                x,
                                y: text_bounds.center_y() - BULLET_DIAMETER / 2.0,
                                width: BULLET_DIAMETER,
                                height: BULLET_DIAMETER,
                            },
                            border: Border {
                                radius: (BULLET_DIAMETER / 2.0).into(),
                                ..Border::default()
                            },
                            shadow: Shadow::default(),
                            snap: false,
                        },
                        style.value,
                    );
                }
            });
        }

        // The blinking caret, on `text_input`'s cadence.
        if let Some(focus) = focus {
            let visible =
                ((focus.now - focus.updated_at).as_millis() / CURSOR_BLINK_INTERVAL_MILLIS) % 2
                    == 0;
            if visible {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: (text_bounds.x + caret as f32 * BULLET_ADVANCE - offset).floor(),
                            y: text_bounds.y,
                            width: 1.0,
                            height: text_bounds.height,
                        },
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: true,
                    },
                    style.value,
                );
            }
        }
    }
}

impl<'a, M: Clone + 'a> From<SecureInput<'a, M>> for Element<'a, M> {
    fn from(input: SecureInput<'a, M>) -> Self {
        Element::new(input)
    }
}

#[cfg(test)]
#[path = "secure_input_tests.rs"]
mod tests;
