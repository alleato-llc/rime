use super::*;

fn sample(rows: usize, columns: Vec<TableColumn>) -> Table<'static, ()> {
    table(rows, columns, |_, _| String::new())
}

fn body(width: f32, height: f32) -> Rectangle {
    Rectangle {
        x: 0.0,
        y: 0.0,
        width,
        height,
    }
}

#[test]
fn column_layout_keeps_fixed_widths_and_splits_fill_evenly() {
    let t = sample(
        1,
        vec![
            TableColumn::fixed("A", 40.0),
            TableColumn::fill("B"),
            TableColumn::fill("C"),
        ],
    );
    let cols = t.column_layout(240.0);
    assert_eq!(cols, vec![(0.0, 40.0), (40.0, 100.0), (140.0, 100.0)]);
}

#[test]
fn column_layout_with_no_fill_column_leaves_the_remainder_unclaimed() {
    let t = sample(1, vec![TableColumn::fixed("A", 40.0)]);
    let cols = t.column_layout(240.0);
    assert_eq!(cols, vec![(0.0, 40.0)]);
}

#[test]
fn max_offset_is_zero_when_content_fits() {
    let t = sample(5, vec![TableColumn::fill("A")]).metrics(TableMetrics {
        row_height: 20.0,
        header_height: 0.0,
    });
    assert_eq!(t.max_offset(body(200.0, 200.0)), 0.0);
}

#[test]
fn max_offset_is_content_minus_viewport() {
    let t = sample(50, vec![TableColumn::fill("A")]).metrics(TableMetrics {
        row_height: 20.0,
        header_height: 0.0,
    });
    // 50 rows * 20px = 1000px content, 200px viewport.
    assert_eq!(t.max_offset(body(200.0, 200.0)), 800.0);
}

#[test]
fn visible_rows_never_exceeds_the_logical_size() {
    let t = sample(3, vec![TableColumn::fill("A")]).metrics(TableMetrics {
        row_height: 20.0,
        header_height: 0.0,
    });
    let (first, last) = t.visible_rows(body(200.0, 200.0), 0.0);
    assert_eq!(first, 0);
    assert_eq!(last, 3);
}

#[test]
fn visible_rows_tracks_the_offset() {
    let t = sample(1000, vec![TableColumn::fill("A")]).metrics(TableMetrics {
        row_height: 20.0,
        header_height: 0.0,
    });
    let (first, last) = t.visible_rows(body(200.0, 100.0), 200.0);
    // offset 200 / row_height 20 = row 10; body fits 5 rows + 2 overscan.
    assert_eq!(first, 10);
    assert_eq!(last, 17);
}

#[test]
fn row_at_maps_a_position_to_the_underlying_row() {
    let t = sample(10, vec![TableColumn::fill("A")]).metrics(TableMetrics {
        row_height: 20.0,
        header_height: 0.0,
    });
    assert_eq!(t.row_at(0.0, Point::new(5.0, 25.0)), Some(1));
    assert_eq!(
        t.row_at(40.0, Point::new(5.0, 5.0)),
        Some(2),
        "offset shifts rows into view"
    );
}

#[test]
fn row_at_is_none_past_the_last_row_or_above_the_body() {
    let t = sample(2, vec![TableColumn::fill("A")]).metrics(TableMetrics {
        row_height: 20.0,
        header_height: 0.0,
    });
    assert_eq!(t.row_at(0.0, Point::new(5.0, -1.0)), None);
    assert_eq!(t.row_at(0.0, Point::new(5.0, 100.0)), None);
}
