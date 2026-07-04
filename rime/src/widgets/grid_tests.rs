use super::*;

fn sample(rows: usize, cols: usize) -> Grid<'static, ()> {
    grid(rows, cols, |_, _| GridCell::default())
}

fn body(width: f32, height: f32) -> Rectangle {
    // A body viewport at the origin; the header offsets don't affect the
    // viewport-relative math the tests exercise.
    Rectangle {
        x: 0.0,
        y: 0.0,
        width,
        height,
    }
}

#[test]
fn column_names_are_bijective_base_26() {
    assert_eq!(column_name(0), "A");
    assert_eq!(column_name(25), "Z");
    assert_eq!(column_name(26), "AA");
    assert_eq!(column_name(27), "AB");
    assert_eq!(column_name(51), "AZ");
    assert_eq!(column_name(701), "ZZ");
    assert_eq!(column_name(702), "AAA");
}

#[test]
fn selection_bounds_normalize_any_corner_order() {
    // extent above-left of anchor still yields min/max in order.
    let selection = Selection {
        anchor: (5, 7),
        extent: (2, 3),
    };
    assert_eq!(selection.bounds(), (2, 5, 3, 7));

    let single = Selection::cell(4, 9);
    assert_eq!(single.bounds(), (4, 4, 9, 9));
}

#[test]
fn max_offset_is_zero_when_content_fits() {
    let g = sample(3, 3); // 3*22 = 66 tall, 3*90 = 270 wide
    let max = g.max_offset(body(1000.0, 1000.0));
    assert_eq!(max.x, 0.0);
    assert_eq!(max.y, 0.0);
}

#[test]
fn max_offset_is_content_minus_viewport() {
    let g = sample(100, 100); // 100*22 = 2200 tall, 100*90 = 9000 wide
    let max = g.max_offset(body(300.0, 200.0));
    assert_eq!(max.x, 9000.0 - 300.0);
    assert_eq!(max.y, 2200.0 - 200.0);
}

#[test]
fn offset_clamps_into_the_legal_range() {
    let g = sample(100, 100).offset(Vector::new(-50.0, 999_999.0));
    let clamped = g.clamped_offset(body(300.0, 200.0));
    assert_eq!(clamped.x, 0.0); // negative clamps up
    assert_eq!(clamped.y, 2200.0 - 200.0); // overshoot clamps down
}

#[test]
fn visible_window_tracks_the_offset() {
    // 22px rows: an offset of 44 skips the first two rows; a 220px tall body
    // shows ~10 rows, plus the 2-cell overscan.
    let g = sample(1000, 1000).offset(Vector::new(0.0, 44.0));
    let (row0, row1) = g.visible_rows(body(300.0, 220.0), Vector::new(0.0, 44.0));
    assert_eq!(row0, 2);
    assert_eq!(row1, 2 + 10 + 2);

    // 90px columns: an offset of 180 skips the first two columns.
    let (col0, col1) = g.visible_cols(body(300.0, 220.0), Vector::new(180.0, 0.0));
    assert_eq!(col0, 2);
    // ceil(300/90) = 4 visible + 2 overscan = 6, from column 2.
    assert_eq!(col1, 2 + 4 + 2);
}

#[test]
fn visible_window_never_exceeds_the_logical_size() {
    let g = sample(5, 5);
    let (row0, row1) = g.visible_rows(body(10_000.0, 10_000.0), Vector::ZERO);
    let (col0, col1) = g.visible_cols(body(10_000.0, 10_000.0), Vector::ZERO);
    assert_eq!((row0, row1), (0, 5));
    assert_eq!((col0, col1), (0, 5));
}
