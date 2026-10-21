use super::*;
use gpui::{PinchEvent, point, px};

/// A pinch as the window has it, in the map's own coordinates.
fn pinch(delta: f32, phase: TouchPhase, at: (f32, f32)) -> PinchEvent {
    PinchEvent {
        position: window_at(at),
        delta,
        phase,
        modifiers: Modifiers::none(),
    }
}

/// A pinch zooms the map by its factor, about the point under the fingers.
#[gpui::test]
fn a_pinch_zooms_the_map_about_the_fingers(cx: &mut TestAppContext) {
    let (host, cx) = stage(map, cx);
    let about = (300.0, 60.0);
    let place = start().to_geo(about, SIZE, TILE);
    cx.simulate_event(pinch(0.0, TouchPhase::Started, about));
    cx.simulate_event(pinch(0.5, TouchPhase::Moved, about));
    settle(cx);
    let views = host.read_with(cx, |stage, _| stage.views.clone());
    assert_eq!(views.len(), 1, "one update per nonzero pinch");
    let view = views[0];
    assert!(
        (view.zoom - (2.0 + 1.5_f64.log2())).abs() < 1e-9,
        "the factor became levels: {}",
        view.zoom
    );
    assert!(
        near(view.to_view(place, SIZE, TILE), about),
        "the place stayed under the fingers: {:?}",
        view.to_view(place, SIZE, TILE)
    );
}

/// Each pinch adds its own factor, and the cancelled one adds nothing.
#[gpui::test]
fn pinches_add_their_factors_and_a_cancelled_one_adds_nothing(cx: &mut TestAppContext) {
    let (host, cx) = stage(map, cx);
    let about = (100.0, 150.0);
    cx.simulate_event(pinch(0.5, TouchPhase::Moved, about));
    cx.simulate_event(pinch(0.5, TouchPhase::Moved, about));
    settle(cx);
    let views = host.read_with(cx, |stage, _| stage.views.clone());
    assert_eq!(views.len(), 2);
    assert!((views[1].zoom - (2.0 + 2.0 * 1.5_f64.log2())).abs() < 1e-9);
    cx.simulate_event(pinch(1.0, TouchPhase::Cancelled, about));
    settle(cx);
    assert_eq!(
        host.read_with(cx, |stage, _| stage.views.clone()).len(),
        2,
        "a cancelled pinch moves nothing"
    );
}

/// A pinch that names no place leaves the map as it was.
#[gpui::test]
fn a_malformed_pinch_leaves_the_map_alone(cx: &mut TestAppContext) {
    let (host, cx) = stage(map, cx);
    for delta in [f32::NAN, f32::INFINITY, -1.0, -2.0] {
        cx.simulate_event(pinch(delta, TouchPhase::Moved, (100.0, 150.0)));
    }
    cx.simulate_event(PinchEvent {
        position: point(px(f32::NAN), px(150.0)),
        ..pinch(0.5, TouchPhase::Moved, (100.0, 150.0))
    });
    settle(cx);
    assert!(
        host.read_with(cx, |stage, _| stage.views.is_empty()),
        "a malformed pinch never moves the map"
    );
}

/// A pinch that begins over a drag lets go of the drag, so the map zooms and does not pan.
#[gpui::test]
fn a_pinch_ends_a_drag_underway(cx: &mut TestAppContext) {
    let (host, cx) = stage(map, cx);
    let none = Modifiers::none();
    cx.simulate_mouse_move(window_at((60.0, 200.0)), None, none);
    cx.simulate_mouse_down(window_at((60.0, 200.0)), MouseButton::Left, none);
    // The first move past the drag threshold starts the drag, and the drag
    // claims that very move, so the map has not panned yet. The next one does.
    cx.simulate_mouse_move(window_at((90.0, 190.0)), Some(MouseButton::Left), none);
    settle(cx);
    cx.simulate_mouse_move(window_at((105.0, 180.0)), Some(MouseButton::Left), none);
    settle(cx);
    assert!(moves(&host, cx) > 0, "the drag moved the map");
    cx.simulate_event(pinch(0.0, TouchPhase::Started, (60.0, 200.0)));
    cx.simulate_event(pinch(0.5, TouchPhase::Moved, (60.0, 200.0)));
    settle(cx);
    let zoomed = moves(&host, cx);
    cx.simulate_mouse_move(window_at((120.0, 180.0)), Some(MouseButton::Left), none);
    cx.simulate_mouse_up(window_at((120.0, 180.0)), MouseButton::Left, none);
    settle(cx);
    assert_eq!(
        moves(&host, cx),
        zoomed,
        "the pinch ended the drag: the map did not pan"
    );
    let views = host.read_with(cx, |stage, _| stage.views.clone());
    let last = views.last().expect("the map moved");
    assert!(last.zoom > 2.0, "the pinch zoomed: {}", last.zoom);
}
