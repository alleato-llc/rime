use super::*;

#[test]
fn starts_empty() {
    let secret = SecretHandle::new();
    assert_eq!(secret.len(), 0);
    assert!(secret.is_empty());
    secret.with_bytes(|bytes| assert!(bytes.is_empty()));
}

#[test]
fn insert_and_read_back() {
    let secret = SecretHandle::new();
    for (i, ch) in "hunter2".chars().enumerate() {
        assert!(secret.insert(i, ch));
    }
    assert_eq!(secret.len(), 7);
    secret.with_bytes(|bytes| assert_eq!(bytes, b"hunter2"));
}

#[test]
fn insert_at_cursor_positions() {
    let secret = SecretHandle::new();
    assert!(secret.insert(0, 'c'));
    assert!(secret.insert(0, 'a')); // front
    assert!(secret.insert(1, 'b')); // middle
    assert!(secret.insert(99, 'd')); // past the end clamps to append
    secret.with_bytes(|bytes| assert_eq!(bytes, b"abcd"));
}

#[test]
fn multibyte_characters_count_as_one() {
    let secret = SecretHandle::new();
    assert!(secret.push_str("pässwörd"));
    assert_eq!(secret.len(), 8); // characters, not bytes
    secret.with_bytes(|bytes| assert_eq!(bytes.len(), 10)); // two 2-byte chars
    assert!(secret.remove(1)); // remove the 2-byte 'ä'
    assert_eq!(secret.len(), 7);
    secret.with_bytes(|bytes| assert_eq!(bytes, "psswörd".as_bytes()));
}

#[test]
fn remove_out_of_range_is_rejected() {
    let secret = SecretHandle::new();
    assert!(!secret.remove(0));
    assert!(secret.push_str("ab"));
    assert!(!secret.remove(2));
    assert_eq!(secret.len(), 2);
}

#[test]
fn appends_beyond_capacity_are_rejected() {
    let secret = SecretHandle::new();
    let fill = "x".repeat(SECRET_CAPACITY);
    assert!(secret.push_str(&fill));
    // Full: a char insert and a further append both bounce, leaving the
    // contents untouched.
    assert!(!secret.insert(0, 'y'));
    assert!(!secret.push_str("y"));
    assert_eq!(secret.len(), SECRET_CAPACITY);
    secret.with_bytes(|bytes| assert!(bytes.iter().all(|&b| b == b'x')));
}

#[test]
fn multibyte_insert_that_does_not_fit_is_rejected() {
    let secret = SecretHandle::new();
    assert!(secret.push_str(&"x".repeat(SECRET_CAPACITY - 1)));
    // One byte free; a 2-byte char must not squeeze in.
    assert!(!secret.insert(0, 'ä'));
    assert!(secret.insert(0, 'y'));
    assert_eq!(secret.len(), SECRET_CAPACITY);
}

#[test]
fn push_str_is_all_or_nothing() {
    let secret = SecretHandle::new();
    assert!(secret.push_str(&"x".repeat(SECRET_CAPACITY - 2)));
    assert!(!secret.push_str("abc"));
    secret.with_bytes(|bytes| assert_eq!(bytes.len(), SECRET_CAPACITY - 2));
}

#[test]
fn never_reallocates() {
    let secret = SecretHandle::new();
    let base = secret.with_bytes(|bytes| bytes.as_ptr());
    assert!(secret.push_str(&"x".repeat(SECRET_CAPACITY)));
    // The storage address is unchanged after filling to capacity: the buffer
    // grew in place (it was pre-allocated), so no secret bytes were copied to
    // a new allocation by a realloc.
    assert_eq!(secret.with_bytes(|bytes| bytes.as_ptr()), base);
    secret.clear();
    assert_eq!(secret.with_bytes(|bytes| bytes.as_ptr()), base);
}

#[test]
fn clear_zeroizes_the_storage() {
    let secret = SecretHandle::new();
    assert!(secret.push_str("hunter2"));
    secret.clear();
    assert_eq!(secret.len(), 0);
    assert!(secret.is_empty());
    // Inspect the whole backing buffer (not just the occupied prefix, which is
    // now empty): every byte of the old secret must be gone.
    let buf = secret.lock();
    assert!(buf.bytes().iter().all(|&b| b == 0));
}

#[test]
fn remove_zeroizes_the_vacated_tail() {
    let secret = SecretHandle::new();
    assert!(secret.push_str("abc"));
    assert!(secret.remove(0));
    let buf = secret.lock();
    assert_eq!(&buf.bytes()[..buf.byte_len], b"bc");
    // The stale third byte left behind by the shift must be wiped.
    assert_eq!(buf.bytes()[2], 0);
}

#[test]
fn drop_wipes_via_the_same_path_clear_uses() {
    // Zeroize-on-drop itself frees the memory, so it cannot be observed from
    // safe code; instead verify the Drop body's operation (a whole-capacity
    // wipe) through an in-place stand-in.
    let mut buf = SecretBuf::default();
    assert!(buf.push_str("secret"));
    buf.storage.zeroize(); // what Drop runs
    assert!(buf.storage.iter().all(|&b| b == 0));
}

#[test]
fn the_locked_window_is_page_aligned_and_page_sized() {
    // mlock/munlock act on whole pages, so the window must be page-aligned and
    // a whole number of pages; otherwise locking would reach into a neighbour's
    // allocation and unlocking on drop would strip its protection.
    let page = region::page::size();
    let secret = SecretHandle::new();
    let buf = secret.lock();
    assert_eq!(buf.bytes().as_ptr().align_offset(page), 0);
    assert_eq!(buf.window % page, 0);
    assert!(buf.window >= SECRET_CAPACITY);
    // The window lies wholly inside the backing allocation.
    assert!(buf.offset + buf.window <= buf.storage.len());
}

#[test]
fn distinct_secrets_never_share_a_locked_page() {
    let page = region::page::size();
    let handles: Vec<_> = (0..8).map(|_| SecretHandle::new()).collect();
    let mut windows: Vec<(usize, usize)> = handles
        .iter()
        .map(|h| {
            let buf = h.lock();
            (buf.bytes().as_ptr() as usize, buf.window)
        })
        .collect();
    windows.sort_unstable();
    for pair in windows.windows(2) {
        let ((start, len), (next, _)) = (pair[0], pair[1]);
        assert!(
            start + len <= next,
            "windows overlap: {start:#x}+{len:#x} runs into {next:#x}"
        );
        assert_eq!(start % page, 0);
    }
}

#[test]
fn clones_share_one_buffer() {
    let secret = SecretHandle::new();
    let alias = secret.clone();
    assert!(secret.push_str("abc"));
    assert_eq!(alias.len(), 3);
    alias.clear();
    assert!(secret.is_empty());
}

#[test]
fn char_index_at_snaps_to_the_nearest_boundary() {
    type Input = SecureInput<'static, ()>;
    assert_eq!(Input::char_index_at(-5.0, 8), 0);
    assert_eq!(Input::char_index_at(0.0, 8), 0);
    // Just under half an advance rounds down; just over rounds up.
    assert_eq!(Input::char_index_at(BULLET_ADVANCE * 0.4, 8), 0);
    assert_eq!(Input::char_index_at(BULLET_ADVANCE * 0.6, 8), 1);
    assert_eq!(Input::char_index_at(BULLET_ADVANCE * 3.5, 8), 4);
    // Clamped to the character count.
    assert_eq!(Input::char_index_at(BULLET_ADVANCE * 100.0, 8), 8);
}

#[test]
fn scroll_offset_keeps_the_caret_in_view() {
    type Input = SecureInput<'static, ()>;
    let bounds = Rectangle {
        x: 0.0,
        y: 0.0,
        width: BULLET_ADVANCE * 10.0,
        height: 20.0,
    };
    // Unfocused, or content that fits: no scroll.
    assert_eq!(Input::scroll_offset(bounds, 50, 100, false), 0.0);
    assert_eq!(Input::scroll_offset(bounds, 5, 8, true), 0.0);
    // Caret past the right edge scrolls just enough to show it.
    let offset = Input::scroll_offset(bounds, 100, 100, true);
    let caret_x = 100.0 * BULLET_ADVANCE;
    assert!(caret_x - offset <= bounds.width);
    // Never scrolls past the content's end.
    assert!(offset <= (100.0 * BULLET_ADVANCE - bounds.width).max(0.0));
}
