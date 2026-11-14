use super::axes::{tick_label_visible_rect, x_tick_label_position, y_tick_label_position};
use crate::charts::advanced::geom::{ScreenPoint, ScreenRect};

#[test]
fn removed_series_releases_its_decimation_cache() {
    use super::build_series;
    use crate::charts::advanced::backend::state::PlotUiState;
    use crate::charts::advanced::render::RenderList;
    use crate::charts::advanced::transform::Transform;
    use crate::charts::advanced::{Plot, Range, Series, Viewport};

    let mut plot = Plot::new();
    let mut series = Series::line("series");
    series.extend_y([1.0, 2.0]).unwrap();
    plot.add_series(&series);
    let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
    let viewport = Viewport::new(Range::new(0.0, 1.0), Range::new(0.0, 3.0));
    let transform = Transform::new(viewport, rect).unwrap();
    let mut state = PlotUiState::default();
    build_series(&mut RenderList::new(), &plot, &mut state, &transform, rect);
    assert_eq!(state.series_cache.len(), 1);
    plot.series_mut().clear();
    build_series(&mut RenderList::new(), &plot, &mut state, &transform, rect);
    assert!(state.series_cache.is_empty());
}

#[test]
fn x_tick_label_position_keeps_tick_center_anchor() {
    let plot_rect = ScreenRect::new(
        ScreenPoint::new(100.0, 20.0),
        ScreenPoint::new(300.0, 180.0),
    );
    let size = (40.0, 12.0);

    let pos = x_tick_label_position(92.0, size, plot_rect);

    assert_eq!(pos.x, 72.0);
    assert_eq!(pos.y, 192.0);
}

#[test]
fn y_tick_label_position_keeps_tick_center_anchor() {
    let plot_rect = ScreenRect::new(
        ScreenPoint::new(100.0, 20.0),
        ScreenPoint::new(300.0, 180.0),
    );
    let size = (30.0, 10.0);

    let pos = y_tick_label_position(16.0, size, plot_rect);

    assert_eq!(pos.x, 58.0);
    assert_eq!(pos.y, 11.0);
}

#[test]
fn x_tick_label_visible_rect_clips_to_x_axis_bounds() {
    let axis_rect = ScreenRect::new(
        ScreenPoint::new(100.0, 180.0),
        ScreenPoint::new(300.0, 220.0),
    );
    let label_rect = ScreenRect::new(
        ScreenPoint::new(80.0, 192.0),
        ScreenPoint::new(140.0, 204.0),
    );

    let visible = tick_label_visible_rect(label_rect, axis_rect).expect("visible rect");

    assert_eq!(visible.min.x, 100.0);
    assert_eq!(visible.max.x, 140.0);
    assert_eq!(visible.min.y, 192.0);
    assert_eq!(visible.max.y, 204.0);
}

#[test]
fn tick_label_visible_rect_returns_none_when_fully_outside() {
    let axis_rect = ScreenRect::new(
        ScreenPoint::new(100.0, 180.0),
        ScreenPoint::new(300.0, 220.0),
    );
    let label_rect = ScreenRect::new(ScreenPoint::new(20.0, 192.0), ScreenPoint::new(60.0, 204.0));

    assert!(tick_label_visible_rect(label_rect, axis_rect).is_none());
}

#[test]
fn linked_cursor_tooltip_shifts_below_legend_on_narrow_overlap() {
    use super::hud::avoid_legend_overlap;

    let plot_rect = ScreenRect::new(ScreenPoint::new(20.0, 10.0), ScreenPoint::new(220.0, 180.0));
    let legend_rect = ScreenRect::new(ScreenPoint::new(110.0, 18.0), ScreenPoint::new(212.0, 62.0));
    let shifted = avoid_legend_overlap(
        ScreenPoint::new(120.0, 20.0),
        (80.0, 34.0),
        Some(legend_rect),
        plot_rect,
    );
    assert_eq!(shifted.y, 68.0);

    let unshifted = avoid_legend_overlap(
        ScreenPoint::new(24.0, 20.0),
        (70.0, 34.0),
        Some(legend_rect),
        plot_rect,
    );
    assert_eq!(unshifted.y, 20.0);
}
