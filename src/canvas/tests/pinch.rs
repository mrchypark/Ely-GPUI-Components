use super::*;
use gpui::{InteractiveElement, PinchEvent};

fn inset_plane(_: &Stage, owner: Entity<Stage>) -> AnyElement {
    div()
        .size_full()
        .pl(px(40.0))
        .pt(px(30.0))
        .on_pinch(|_, _, _| panic!("the canvas must consume its pinch"))
        .child(
            InfiniteCanvas::new("inset-plane", Viewport::new(0.0, 0.0, 1.0))
                .on_viewport(move |view, _, cx| hear(&owner, view, cx)),
        )
        .into_any_element()
}

fn pinch(delta: f32, phase: TouchPhase) -> PinchEvent {
    PinchEvent {
        position: point(px(140.0), px(80.0)),
        delta,
        phase,
        modifiers: Modifiers::none(),
    }
}

#[gpui::test]
fn native_pinch_uses_incremental_delta_and_canvas_local_anchor(cx: &mut TestAppContext) {
    let (host, cx) = stage(inset_plane, cx);
    cx.simulate_event(pinch(0.0, TouchPhase::Started));
    cx.simulate_event(pinch(0.5, TouchPhase::Moved));
    cx.simulate_event(pinch(0.5, TouchPhase::Moved));
    cx.simulate_event(pinch(0.0, TouchPhase::Ended));
    settle(cx);
    let views = heard(&host, cx);
    assert_eq!(
        views.len(),
        2,
        "one update per nonzero pinch, without wheel replay"
    );
    let view = views[1];
    assert_eq!(view.zoom, 2.25);
    let held = view.to_canvas((100.0, 50.0));
    assert!((held.0 - 100.0).abs() < 0.001 && (held.1 - 50.0).abs() < 0.001);
    cx.simulate_event(wheel((140.0, 80.0), -240.0, Modifiers::command()));
    settle(cx);
    let views = heard(&host, cx);
    assert_eq!(views.len(), 3);
    assert!((views[2].zoom - 2.25 * std::f32::consts::E).abs() < 0.001);
}

#[gpui::test]
fn native_pinch_rejects_invalid_and_cancelled_updates(cx: &mut TestAppContext) {
    let (host, cx) = stage(inset_plane, cx);
    for delta in [f32::NAN, f32::INFINITY, -1.0, -2.0] {
        cx.simulate_event(pinch(delta, TouchPhase::Moved));
    }
    cx.simulate_event(pinch(1.0, TouchPhase::Cancelled));
    settle(cx);
    assert!(heard(&host, cx).is_empty());
    cx.simulate_event(pinch(100.0, TouchPhase::Moved));
    settle(cx);
    let view = *heard(&host, cx)
        .last()
        .expect("valid pinch after cancellation");
    assert_eq!(view.zoom, Viewport::ZOOMS.1);
    assert_eq!(view.to_canvas((100.0, 50.0)), (100.0, 50.0));
}

#[gpui::test]
fn malformed_pinch_leaves_an_active_desktop_pan_intact(cx: &mut TestAppContext) {
    let (host, cx) = stage(plane, cx);
    let at = |x| point(px(x), px(150.0));
    cx.simulate_mouse_move(at(200.0), None, Modifiers::none());
    cx.simulate_mouse_down(at(200.0), MouseButton::Left, Modifiers::none());
    for x in [190.0, 180.0] {
        cx.simulate_mouse_move(at(x), Some(MouseButton::Left), Modifiers::none());
        settle(cx);
    }
    let before = heard(&host, cx);
    assert_eq!(before.last().expect("pan started").x, 20.0);
    for phase in [TouchPhase::Started, TouchPhase::Moved] {
        for delta in [f32::NAN, f32::INFINITY, -1.0, -2.0] {
            cx.simulate_event(pinch(delta, phase));
        }
        cx.simulate_event(PinchEvent {
            position: point(px(f32::NAN), px(80.0)),
            ..pinch(0.5, phase)
        });
    }
    settle(cx);
    assert_eq!(
        heard(&host, cx),
        before,
        "malformed pinch never changes the view"
    );
    cx.simulate_mouse_move(at(150.0), Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(at(150.0), MouseButton::Left, Modifiers::none());
    settle(cx);
    let after = heard(&host, cx);
    assert!(after.len() > before.len(), "pan still emits updates");
    assert_eq!(*after.last().unwrap(), Viewport::new(50.0, 0.0, 1.0));
}
