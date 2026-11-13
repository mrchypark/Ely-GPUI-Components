use gpui::MouseButton;

use crate::charts::advanced::geom::{ScreenPoint, ScreenRect};
use crate::charts::advanced::interaction::pan_viewport;
use crate::charts::advanced::transform::Transform;
use crate::charts::advanced::view::{Range, Viewport};

use super::super::state::DragMode;
use super::input::is_drag_button_held;
use super::scroll::{pinch_zoom_scale, scroll_zoom_factor};

#[test]
fn drag_requires_matching_button() {
    assert!(is_drag_button_held(DragMode::Pan, Some(MouseButton::Left)));
    assert!(is_drag_button_held(
        DragMode::ZoomX,
        Some(MouseButton::Left)
    ));
    assert!(is_drag_button_held(
        DragMode::ZoomY,
        Some(MouseButton::Left)
    ));
    assert!(is_drag_button_held(
        DragMode::ZoomRect,
        Some(MouseButton::Right)
    ));
    assert!(!is_drag_button_held(
        DragMode::Pan,
        Some(MouseButton::Right)
    ));
    assert!(!is_drag_button_held(DragMode::ZoomRect, None));
}

#[test]
fn positive_scroll_delta_zooms_in_after_reversal() {
    let factor = scroll_zoom_factor(120.0);
    assert!(factor < 1.0, "expected zoom-in factor, got {factor}");
}

#[test]
fn negative_scroll_delta_zooms_out_after_reversal() {
    let factor = scroll_zoom_factor(-120.0);
    assert!(factor > 1.0, "expected zoom-out factor, got {factor}");
}

#[test]
fn positive_pinch_delta_zooms_in_and_touch_pan_moves_opposite_drag() {
    assert!(pinch_zoom_scale(0.1) > 1.0);
    assert!(pinch_zoom_scale(-0.1) < 1.0);
    assert_eq!(pinch_zoom_scale(f32::NAN), 1.0);

    let viewport = Viewport::new(Range::new(100.0, 200.0), Range::new(0.0, 10.0));
    let rect = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
    let transform = Transform::new(viewport, rect).unwrap();
    let panned = pan_viewport(viewport, ScreenPoint::new(10.0, 0.0), &transform).unwrap();
    assert!(panned.x.min < viewport.x.min);
    assert_eq!(panned.y, viewport.y);
}
