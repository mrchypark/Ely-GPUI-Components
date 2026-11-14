use super::*;

#[cfg(feature = "test-support")]
mod interactive_tests {
    use std::rc::Rc;

    use gpui::{
        Context, Modifiers, MouseButton, Render, TestAppContext, div, point, prelude::*, px,
    };

    use crate::charts::pointer::answer;
    use crate::charts::{
        geometry::{Geometry, Rect},
        paint::measure,
        parts::ChartTooltip,
        plot::{Pick, State},
        tests::settle,
    };
    use crate::theme::Theme;

    struct HoverProbe {
        state: gpui::Entity<State>,
        span: (usize, usize),
        total: usize,
    }

    impl HoverProbe {
        fn new(cx: &mut Context<Self>, span: (usize, usize), total: usize) -> Self {
            Self {
                state: cx.new(|_| {
                    let mut state = State::default();
                    state.span = Some(span);
                    state
                }),
                span,
                total,
            }
        }
    }

    impl Render for HoverProbe {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            cx: &mut Context<Self>,
        ) -> impl gpui::IntoElement {
            let hovered = self.state.read(cx).hover.is_some();
            let rect = Rect {
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 200.0,
            };
            answer(
                div()
                    .id("hover-probe")
                    .w(px(300.0))
                    .h(px(200.0))
                    .debug_selector(|| "chart-root".into()),
                &self.state,
                (Pick::Point, false, true),
                (rect, self.span, 0.0, self.total),
                Rc::new(Geometry::default()),
            )
            .child(measure(self.state.clone(), |state| &mut state.bounds))
            .children(hovered.then(|| {
                div()
                    .debug_selector(|| "hover-tooltip".into())
                    .child(ChartTooltip::new("picked"))
            }))
        }
    }

    macro_rules! probe {
        ($cx:expr, $touch:expr) => {{
            $cx.update(|cx| {
                Theme::init(cx);
                Theme::update(cx, |theme| theme.touch_mode = $touch);
            });
            let (_, cx) = $cx.add_window_view(|_, cx| HoverProbe::new(cx, (0, 10), 11));
            cx.simulate_resize(gpui::size(px(400.0), px(240.0)));
            settle(cx);
            cx
        }};
    }

    #[gpui::test]
    fn a_press_selects_hover_without_a_mouse_move(cx: &mut TestAppContext) {
        let cx = probe!(cx, true);
        let at = point(px(100.0), px(100.0));
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        settle(cx);

        assert!(
            cx.debug_bounds("hover-tooltip").is_some(),
            "a press alone opens the hover tooltip"
        );
    }

    #[gpui::test]
    fn mouse_move_still_selects_hover(cx: &mut TestAppContext) {
        let cx = probe!(cx, false);
        cx.simulate_mouse_move(point(px(100.0), px(100.0)), None, Modifiers::none());
        settle(cx);

        assert!(cx.debug_bounds("hover-tooltip").is_some());
    }

    #[gpui::test]
    fn moving_outside_still_clears_hover(cx: &mut TestAppContext) {
        let cx = probe!(cx, false);
        cx.simulate_mouse_move(point(px(100.0), px(100.0)), None, Modifiers::none());
        settle(cx);
        assert!(cx.debug_bounds("hover-tooltip").is_some());

        cx.simulate_mouse_move(point(px(350.0), px(220.0)), None, Modifiers::none());
        settle(cx);
        assert!(cx.debug_bounds("hover-tooltip").is_none());
    }

    #[gpui::test]
    fn touch_drag_zooms_horizontally_but_leaves_vertical_scroll(cx: &mut TestAppContext) {
        use gpui::{TouchDragEvent, TouchPhase};
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, cx| HoverProbe::new(cx, (0, 10), 11));
        cx.simulate_resize(gpui::size(px(400.0), px(240.0)));
        settle(cx);
        let send = |phase, start_position, position, cx: &mut gpui::VisualTestContext| {
            cx.simulate_event(TouchDragEvent {
                phase,
                start_position,
                position,
            });
            settle(cx);
        };
        let start = point(px(60.0), px(80.0));
        let end = point(px(240.0), px(90.0));
        send(TouchPhase::Started, start, end, cx);
        send(TouchPhase::Ended, start, end, cx);
        assert_eq!(
            view.read_with(cx, |view, cx| view.state.read(cx).span),
            Some((2, 8))
        );
        view.update(cx, |view, cx| {
            view.state.update(cx, |state, _| state.span = None)
        });
        let vertical = point(px(65.0), px(160.0));
        send(TouchPhase::Started, start, vertical, cx);
        send(TouchPhase::Ended, start, vertical, cx);
        assert_eq!(
            view.read_with(cx, |view, cx| view.state.read(cx).span),
            None
        );
    }

    #[gpui::test]
    fn pinch_zooms_category_range(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, cx| HoverProbe::new(cx, (0, 10), 11));
        cx.simulate_resize(gpui::size(px(400.0), px(240.0)));
        settle(cx);
        cx.simulate_event(gpui::PinchEvent {
            position: point(px(150.0), px(100.0)),
            delta: 0.6,
            phase: gpui::TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(cx);
        let span = view
            .read_with(cx, |view, cx| view.state.read(cx).span)
            .unwrap();
        assert!(span.1 - span.0 < 10);
    }

    #[gpui::test]
    fn touch_pinch_expands_a_zoomed_category_range_to_the_full_count(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, cx| HoverProbe::new(cx, (5, 9), 20));
        cx.simulate_resize(gpui::size(px(400.0), px(240.0)));
        settle(cx);
        cx.simulate_event(gpui::PinchEvent {
            position: point(px(150.0), px(100.0)),
            delta: 0.6_f32.ln(),
            phase: gpui::TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(cx);

        assert_eq!(
            view.read_with(cx, |view, cx| view.state.read(cx).span),
            Some((3, 10)),
            "expansion uses all 20 categories and keeps the range centered"
        );
    }
}

#[test]
fn the_pointer_picks_bands_points_and_dots() {
    use crate::charts::series::Symbol;
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 50.0,
    };
    assert_eq!(
        pick_at(Pick::Band, (30.0, 10.0), frame, false, (0, 3), &[], 0.0),
        Some(1)
    );
    assert_eq!(
        pick_at(Pick::Point, (40.0, 10.0), frame, false, (2, 4), &[], 0.0),
        Some(3),
        "the nearest of three, past the zoomed start"
    );
    assert_eq!(
        pick_at(Pick::Band, (30.0, 60.0), frame, false, (0, 3), &[], 0.0),
        None,
        "below the frame"
    );
    assert_eq!(
        pick_at(Pick::Band, (10.0, 30.0), frame, true, (0, 4), &[], 0.0),
        Some(3),
        "sideways bands run down"
    );
    let dots = [Dot {
        ink: Ink::Series(0),
        center: (10.0, 10.0),
        radius: 2.0,
        symbol: Symbol::Circle,
        key: 7,
    }];
    assert_eq!(
        pick_at(Pick::Dot, (14.0, 10.0), frame, false, (0, 0), &dots, 6.0),
        Some(7)
    );
    assert_eq!(
        pick_at(Pick::Dot, (30.0, 10.0), frame, false, (0, 0), &dots, 6.0),
        None,
        "out of reach"
    );
}
