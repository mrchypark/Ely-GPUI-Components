use gpui::{AnyElement, Entity, IntoElement, TestAppContext};

use super::{Stage, said, say, settle, stage};
use crate::canvas::{AutoLayout, AutoLayoutControls, Flow, Frame, Place, Shape, ShapeKind, touch};

fn shape(key: &'static str, frame: Frame) -> Shape {
    Shape::new(key, key, ShapeKind::Rect, frame)
}

#[test]
fn a_small_shape_is_reached_by_a_finger_but_not_by_a_mouse() {
    let small = shape("a", Frame::new(100.0, 100.0, 10.0, 10.0));
    let shapes = [small];
    // Past the shape's own edge and past the mouse's reach, but inside the
    // touch target's reach.
    let near = (120.0, 105.0);
    assert_eq!(
        touch::hit_touch(&shapes, near, 1.0, false),
        None,
        "a mouse takes a shape by its own edge"
    );
    assert_eq!(
        touch::hit_touch(&shapes, near, 1.0, true),
        Some(0),
        "a finger reaches past the edge of a small shape"
    );
}

#[test]
fn a_shape_big_on_screen_keeps_its_own_edge() {
    let big = shape("a", Frame::new(0.0, 0.0, 200.0, 200.0));
    let shapes = [big];
    assert_eq!(touch::hit_touch(&shapes, (1.0, 1.0), 1.0, true), Some(0));
    assert_eq!(
        touch::hit_touch(&shapes, (-9.0, 100.0), 1.0, true),
        None,
        "a big shape does not grow into its neighbour's space"
    );
}

#[test]
fn a_locked_or_hidden_shape_stays_out_of_reach() {
    let mut locked = shape("a", Frame::new(0.0, 0.0, 10.0, 10.0));
    locked = locked.locked(true);
    let mut hidden = shape("b", Frame::new(100.0, 0.0, 10.0, 10.0));
    hidden = hidden.hidden(true);
    let shapes = [locked, hidden];
    assert_eq!(touch::hit_touch(&shapes, (4.0, 4.0), 1.0, true), None);
    assert_eq!(touch::hit_touch(&shapes, (104.0, 4.0), 1.0, true), None);
}

#[test]
fn the_reach_grows_with_the_zoom() {
    let frame = Frame::new(0.0, 0.0, 10.0, 10.0);
    let near = touch::reach(frame, 1.0);
    let far = touch::reach(frame, 4.0);
    assert!(near > 0.0 && far > 0.0);
    assert!(
        far < near,
        "a zoomed out shape needs less reach in canvas units: {far} vs {near}"
    );
    assert_eq!(touch::reach(frame, 0.0), 0.0);
    assert_eq!(touch::reach(frame, f32::NAN), 0.0);
}

fn places(_: &Stage, owner: Entity<Stage>) -> AnyElement {
    let layout = AutoLayout {
        flow: Flow::Row,
        gap: 8.0,
        padding: (8.0, 8.0),
        place: (Place::Start, Place::Start),
    };
    AutoLayoutControls::new("auto", layout, move |next, _, cx| {
        say(&owner, format!("{:?}", next.place), cx)
    })
    .into_any_element()
}

#[gpui::test]
fn a_place_cell_keeps_the_touch_floor_but_its_mark_stays_small(cx: &mut TestAppContext) {
    let (host, cx) = stage(places, cx);
    cx.update(|_, cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    settle(cx);
    let cell = cx
        .debug_bounds("place-Start-Start")
        .expect("the first place cell");
    assert!(
        cell.size.width >= gpui::px(48.0) && cell.size.height >= gpui::px(48.0),
        "the cell a finger lands on is at least a touch target: {:?}",
        cell.size
    );
    // The click still picks the place exactly once.
    cx.simulate_click(cell.center(), gpui::Modifiers::none());
    settle(cx);
    assert_eq!(said(&host, cx), ["(Start, Start)"]);
}

#[gpui::test]
fn a_desktop_place_cell_keeps_its_own_size(cx: &mut TestAppContext) {
    let (_, cx) = stage(places, cx);
    let cell = cx
        .debug_bounds("place-Start-Start")
        .expect("the first place cell");
    assert!(
        cell.size.width < gpui::px(48.0) && cell.size.height < gpui::px(48.0),
        "a mouse keeps the grid as it is drawn: {:?}",
        cell.size
    );
}
