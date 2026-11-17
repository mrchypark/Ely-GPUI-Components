use crate::charts::advanced::datasource::*;

use crate::charts::advanced::{AppendError, Point, Range};

#[test]
fn indexed_range_matches_indices() {
    let data = AppendOnlyData::from_iter_y([1.0, 2.0, 3.0, 4.0]);
    let range = data.range_by_x(Range::new(1.0, 2.1));
    let slice = &data.points()[range];
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].x, 1.0);
    assert_eq!(slice[1].x, 2.0);
}

#[test]
fn indexed_range_respects_fractional_bounds() {
    let data = AppendOnlyData::from_iter_y([1.0, 2.0, 3.0, 4.0, 5.0]);
    let range = data.range_by_x(Range::new(1.2, 3.8));
    let slice = &data.points()[range];
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].x, 2.0);
    assert_eq!(slice[1].x, 3.0);
}

#[test]
fn explicit_range_uses_binary_search() {
    let points = [
        Point::new(0.0, 1.0),
        Point::new(1.0, 2.0),
        Point::new(2.0, 3.0),
        Point::new(3.0, 4.0),
    ];
    let data = AppendOnlyData::from_iter_points(points);
    let range = data.range_by_x(Range::new(0.5, 2.5));
    let slice = &data.points()[range];
    assert_eq!(slice.len(), 2);
    assert_eq!(slice[0].x, 1.0);
    assert_eq!(slice[1].x, 2.0);
}

#[test]
fn non_monotonic_explicit_marks_flag() {
    let mut data = AppendOnlyData::explicit();
    let _ = data.push_point(Point::new(1.0, 1.0));
    let result = data.push_point(Point::new(0.5, 2.0));
    assert_eq!(result, Err(AppendError::NonMonotonicX));
    assert!(!data.is_monotonic());
}

#[test]
fn extend_y_appends_multiple_values() {
    let mut data = AppendOnlyData::indexed();
    let added = data.extend_y([1.0, 2.0, 3.0]).unwrap();
    assert_eq!(added, 3);
    assert_eq!(data.point(0), Some(Point::new(0.0, 1.0)));
    assert_eq!(data.point(2), Some(Point::new(2.0, 3.0)));
}

#[test]
fn extend_points_non_monotonic_still_appends_batch() {
    let mut data = AppendOnlyData::explicit();
    let _ = data.extend_points([Point::new(1.0, 1.0), Point::new(2.0, 2.0)]);
    let result = data.extend_points([Point::new(1.5, 3.0), Point::new(4.0, 4.0)]);

    assert_eq!(result, Err(AppendError::NonMonotonicX));
    assert_eq!(data.len(), 4);
    assert_eq!(data.point(2), Some(Point::new(1.5, 3.0)));
    assert_eq!(data.point(3), Some(Point::new(4.0, 4.0)));
    assert!(!data.is_monotonic());
}

#[test]
fn extend_points_wrong_mode_does_not_append() {
    let mut data = AppendOnlyData::indexed();
    let result = data.extend_points([Point::new(0.0, 1.0)]);
    assert_eq!(result, Err(AppendError::WrongMode));
    assert!(data.is_empty());
}

#[test]
fn nearest_index_for_indexed_data_rounds() {
    let data = AppendOnlyData::from_iter_y([0.0, 1.0, 2.0, 3.0]);
    assert_eq!(data.nearest_index_by_x(2.4), Some(2));
    assert_eq!(data.nearest_index_by_x(2.6), Some(3));
    assert_eq!(data.nearest_index_by_x(-2.0), Some(0));
    assert_eq!(data.nearest_index_by_x(99.0), Some(3));
}

#[test]
fn nearest_index_for_monotonic_explicit_data_uses_binary_search() {
    let data = AppendOnlyData::from_iter_points([
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
        Point::new(3.0, 3.0),
        Point::new(10.0, 4.0),
    ]);
    assert_eq!(data.nearest_index_by_x(2.2), Some(2));
    assert_eq!(data.nearest_index_by_x(8.0), Some(3));
    assert_eq!(data.nearest_index_by_x(-5.0), Some(0));
}

#[test]
fn nearest_index_for_non_monotonic_explicit_data_falls_back_to_linear_scan() {
    let mut data = AppendOnlyData::explicit();
    let _ = data.extend_points([
        Point::new(0.0, 0.0),
        Point::new(5.0, 1.0),
        Point::new(2.0, 2.0),
        Point::new(10.0, 3.0),
    ]);
    assert_eq!(data.nearest_index_by_x(2.1), Some(2));
}
