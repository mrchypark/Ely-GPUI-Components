use super::*;

#[test]
fn difference_chart_splits_crossing_segments() {
    let scale = Linear::new((0.0, 10.0), (100.0, 0.0));
    let regions = split_difference_regions(&[0.0, 100.0], &[8.0, 2.0], &[2.0, 8.0], &scale);
    assert_eq!(regions.len(), 2);
    assert!(regions[0].0, "first triangle has primary above baseline");
    assert!(!regions[1].0, "second triangle has primary below baseline");
    assert_eq!(regions[0].1[1].0, 50.0, "crossing at midpoint x=50");
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn difference_chart_supports_pin_brush_scroll_zoom_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

    struct DiffHarness;
    impl Render for DiffHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.0)).h(px(280.0)).child(
                DifferenceChart::new("diff-int", ["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
                    .primary(Series::new("Actual", [10.0, 22.0, 14.0, 28.0, 19.0, 32.0]))
                    .baseline(Series::new("Plan", [12.0, 16.0, 18.0, 20.0, 24.0, 26.0])),
            )
        }
    }

    cx.update(Theme::init);
    let (_, vc) = cx.add_window_view(|_, _| DiffHarness);
    vc.simulate_resize(gpui::size(px(400.0), px(280.0)));
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("difference root");
    let click_at = root.center();
    vc.simulate_mouse_down(click_at, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(click_at, MouseButton::Left, Modifiers::none());
    settle(vc);

    // Moving outside keeps pinned crosshair and tooltip visible.
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "clicking without dragging pins tooltip"
    );

    // Clicking same spot unpins it.
    vc.simulate_mouse_down(click_at, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(click_at, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_none(),
        "clicking again unpins tooltip"
    );

    // Click to pin, then drag horizontally (> 4px) to brush-zoom.
    vc.simulate_mouse_down(click_at, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(click_at, MouseButton::Left, Modifiers::none());
    settle(vc);
    let from = point(root.left() + root.size.width * 0.25, click_at.y);
    let to = point(root.left() + root.size.width * 0.75, click_at.y);
    vc.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
    vc.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-show-all").is_some(),
        "horizontal drag brush zooms span"
    );

    // Show all resets both span and pinned state.
    let show_all = vc.debug_bounds("chart-show-all").expect("show-all button");
    vc.simulate_mouse_down(show_all.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(show_all.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-show-all").is_none());
    assert!(
        vc.debug_bounds("chart-tooltip").is_none(),
        "Show all resets pinned state"
    );

    // Wheel scroll zooms span; double-click resets span and pinned state.
    vc.simulate_event(ScrollEvent {
        position: root.center(),
        delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    settle(vc);
    assert!(
        vc.debug_bounds("chart-show-all").is_some(),
        "wheel scroll zooms span"
    );

    vc.simulate_event(MouseDownEvent {
        position: root.center(),
        button: MouseButton::Left,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_mouse_up(root.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-show-all").is_none(),
        "double-click resets span"
    );
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn difference_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};

    struct NarrowDiff;
    impl Render for NarrowDiff {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(240.0)).h(px(240.0)).child(
                DifferenceChart::new(
                    "diff-touch",
                    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],
                )
                .primary(Series::new(
                    "Actual",
                    [10.0, 22.0, 14.0, 28.0, 19.0, 32.0, 25.0, 34.0],
                ))
                .baseline(Series::new(
                    "Plan",
                    [12.0, 16.0, 18.0, 20.0, 24.0, 26.0, 22.0, 29.0],
                )),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| NarrowDiff);
    vc.simulate_resize(gpui::size(px(240.0), px(240.0)));
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("difference root");
    // Tap on the right side (> 0.55 * width) pins and flips tooltip inside 240px card.
    let right_tap = point(root.left() + root.size.width * 0.78, root.center().y);
    vc.simulate_mouse_down(right_tap, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(right_tap, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip = vc.debug_bounds("chart-tooltip").expect("tap pins tooltip");
    assert!(
        tip.right() <= root.right() + px(1.0),
        "right-side tooltip flips inside narrow 240px card"
    );
    let reset_btn = vc
        .debug_bounds("chart-show-all")
        .expect("pinned selection shows reset button");
    assert!(reset_btn.size.width >= px(48.0) && reset_btn.size.height >= px(48.0));
    vc.simulate_click(reset_btn.center(), Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-show-all").is_none());
    assert!(vc.debug_bounds("chart-tooltip").is_none());

    // Single-finger down starts brush, then pinch clears brush so trailing move does not brush-zoom.
    let from = point(root.left() + root.size.width * 0.3, root.center().y);
    let to = point(root.left() + root.size.width * 0.9, root.center().y);
    vc.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    vc.simulate_event(PinchEvent {
        position: root.center(),
        delta: 0.6,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc);
    vc.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
    vc.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-show-all").is_some());

    // Horizontal touch swipe pans when zoomed, and Show all resets.
    vc.simulate_event(ScrollEvent {
        position: root.center(),
        delta: ScrollDelta::Pixels(point(px(30.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);
    let show_all = vc
        .debug_bounds("chart-show-all")
        .expect("show-all after pan");
    vc.simulate_click(show_all.center(), Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-show-all").is_none());

    // Horizontal touch drag brushes and zooms without requiring an earlier mouse-down.
    vc.simulate_event(gpui::TouchDragEvent {
        phase: TouchPhase::Started,
        start_position: from,
        position: to,
    });
    vc.simulate_event(gpui::TouchDragEvent {
        phase: TouchPhase::Ended,
        start_position: from,
        position: to,
    });
    settle(vc);
    assert!(vc.debug_bounds("chart-show-all").is_some());
}
