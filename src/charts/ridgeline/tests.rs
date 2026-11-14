use super::*;

#[test]
fn ridgeline_computes_stacked_curves_within_frame() {
    let groups = vec![
        ("A".into(), vec![1.0, 2.0, 2.5, 3.0, 4.0]),
        ("B".into(), vec![3.0, 4.0, 4.5, 5.0, 6.0]),
    ];
    let layout = compute_ridges(
        &groups,
        Rect {
            x: 20.0,
            y: 10.0,
            w: 200.0,
            h: 160.0,
        },
        24,
        1.5,
        None,
    );
    assert_eq!(layout.ridges.len(), 2);
    assert!(layout.ridges[1].baseline > layout.ridges[0].baseline);
    assert_eq!(layout.ridges[0].points.len(), 24);
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn ridgeline_supports_legend_pinch_pan_and_touch_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{
        Context, Modifiers, PinchEvent, Render, ScrollDelta, ScrollEvent, ScrollSource, TouchPhase,
        Window, point, px,
    };

    struct Harness;
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(340.0)).h(px(260.0)).child(
                RidgelineChart::new("ridge-ui")
                    .group("Alpha", [10.0, 20.0, 25.0, 30.0])
                    .group("Beta", [15.0, 25.0, 35.0, 45.0]),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| Harness);
    settle(vc);
    let root = vc.debug_bounds("chart-root").expect("ridge root");
    let mid = root.center();
    vc.simulate_event(PinchEvent {
        position: mid,
        delta: 0.4,
        modifiers: Modifiers::none(),
        phase: TouchPhase::Moved,
    });
    settle(vc);
    vc.simulate_event(ScrollEvent {
        position: mid,
        delta: ScrollDelta::Pixels(point(px(24.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);
}
