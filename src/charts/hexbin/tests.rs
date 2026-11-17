use super::*;

#[test]
fn hexbin_snaps_nearby_points_and_handles_empty_or_degenerate_domains() {
    let rect = Rect {
        x: 20.0,
        y: 10.0,
        w: 200.0,
        h: 160.0,
    };
    let empty = compute_layout(&[], rect, 12.0, None);
    assert!(empty.bins.is_empty());
    assert_eq!(empty.max_count, 0);

    let same = compute_layout(&[(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)], rect, 12.0, None);
    assert_eq!(same.bins.len(), 1);
    assert_eq!(same.bins[0].count, 3);
    assert_eq!(same.max_count, 3);

    let cluster = compute_layout(&[(0.0, 0.0), (1.0, 1.0), (100.0, 100.0)], rect, 16.0, None);
    let total: usize = cluster.bins.iter().map(|b| b.count).sum();
    assert_eq!(total, 3);
    assert!(cluster.bins.len() >= 2);

    let base = ((0.0, 10.0), (0.0, 10.0));
    let tiny = zoom_domain(base, base, (5.0, 5.0), 1e-9);
    assert!((tiny.0.1 - tiny.0.0) >= 10.0 * 1e-4 - 1e-9);
    let wide = zoom_domain(base, base, (5.0, 5.0), 100.0);
    assert!((wide.0.1 - wide.0.0) <= 10.0 * 20.0 + 1e-6);
}

#[test]
fn hex_vertices_form_regular_circumradius_hexagon() {
    let verts = hex_vertices((100.0, 50.0), 10.0);
    for (x, y) in verts {
        let dist = ((x - 100.0).powi(2) + (y - 50.0).powi(2)).sqrt();
        assert!((dist - 10.0).abs() < 1e-4);
    }
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn hexbin_chart_renders_exports_and_opens_tooltip_on_hover(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{
        Context, Modifiers, PinchEvent, Render, ScrollDelta, ScrollEvent, Window, point, px,
    };
    use std::cell::RefCell;

    struct HexHarness(Vec<(f64, f64)>, Rc<RefCell<Vec<f64>>>);
    impl Render for HexHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let seen = self.1.clone();
            div().w(px(360.0)).h(px(260.0)).child(
                HexbinChart::new("hex-test")
                    .points(self.0.iter().copied())
                    .radius(14.0)
                    .axes("Duration", "Cost")
                    .format(move |v| {
                        seen.borrow_mut().push(v);
                        compact(v)
                    }),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        let chart = HexbinChart::new("export")
            .points([(10.0, 20.0), (11.0, 21.0), (50.0, 80.0)])
            .radius(12.0);
        let svg = chart.svg(360.0, 240.0, cx);
        assert!(svg.starts_with("<svg") && svg.contains("<path"));
        let csv = chart.csv(360.0, 240.0, cx);
        assert!(csv.starts_with("x,y,count\n"));
    });

    let (_, vc) = cx.add_window_view(|_, _| HexHarness(vec![], Rc::new(RefCell::new(Vec::new()))));
    settle(vc);
    assert!(vc.debug_bounds("hexbin-plot").is_some());

    let recorded = Rc::new(RefCell::new(Vec::new()));
    let (_, vc) = cx.add_window_view(|_, _| {
        HexHarness(
            vec![(0.0, 0.0), (50.0, 50.0), (50.5, 50.5), (100.0, 100.0)],
            recorded.clone(),
        )
    });
    settle(vc);
    let plot = vc.debug_bounds("hexbin-plot").expect("hexbin plot bounds");
    let rect_center = point(
        plot.left() + px(48.0) + (plot.size.width - px(56.0)) * 0.5,
        plot.top() + px(8.0) + (plot.size.height - px(36.0)) * 0.5,
    );
    vc.simulate_mouse_move(rect_center, None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "hover over bin opens tooltip"
    );
    assert!(vc.debug_bounds("show-all").is_none());

    // Pointer wheel scroll zooms in around cursor.
    recorded.borrow_mut().clear();
    vc.simulate_event(ScrollEvent {
        position: rect_center,
        delta: ScrollDelta::Pixels(point(px(0.0), px(200.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    settle(vc);
    assert!(vc.debug_bounds("show-all").is_some());
    let zoomed_ticks = recorded.borrow().clone();
    assert!(!zoomed_ticks.is_empty());
    let zoomed_span = zoomed_ticks
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        - zoomed_ticks.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(zoomed_span < 100.0, "wheel zoom shrinks visible domain");

    // Left-button drag (> 3px) pans the domain.
    recorded.borrow_mut().clear();
    let start = rect_center;
    vc.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(
        start + point(px(90.0), px(-90.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        start + point(px(90.0), px(-90.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(vc);
    let panned_ticks = recorded.borrow().clone();
    assert!(!panned_ticks.is_empty());
    let panned_min = panned_ticks.iter().copied().fold(f64::INFINITY, f64::min);
    let zoomed_min = zoomed_ticks.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(panned_min < zoomed_min, "dragging shifts domain ticks");

    // Double-click resets domain = None.
    vc.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: rect_center,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_event(gpui::MouseUpEvent {
        button: MouseButton::Left,
        position: rect_center,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(vc);
    assert!(
        vc.debug_bounds("show-all").is_none(),
        "double-click resets zoom"
    );

    // Pinch zooms in and Show all button resets.
    vc.simulate_event(PinchEvent {
        position: rect_center,
        delta: 0.6,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc);
    let show_all = vc
        .debug_bounds("show-all")
        .expect("pinch shows reset button");
    vc.simulate_click(show_all.center(), Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("show-all").is_none(),
        "Show all button resets zoom"
    );

    // Touch scroll pans and tap without drag preserves tooltip.
    vc.simulate_event(ScrollEvent {
        position: rect_center,
        delta: ScrollDelta::Pixels(point(px(30.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);
    assert!(vc.debug_bounds("show-all").is_some());
    let reset = vc
        .debug_bounds("show-all")
        .expect("show-all after touch pan");
    vc.simulate_click(reset.center(), Modifiers::none());
    settle(vc);
    vc.simulate_mouse_down(rect_center, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(rect_center, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "tap without drag preserves tooltip"
    );
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn hexbin_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};
    use std::cell::RefCell;

    let pts = vec![(0.0, 0.0), (80.0, 50.0), (100.0, 100.0)];
    struct NarrowHex(Vec<(f64, f64)>, Rc<RefCell<Vec<f64>>>);
    impl Render for NarrowHex {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let seen = self.1.clone();
            div().w(px(240.0)).h(px(240.0)).child(
                HexbinChart::new("hex-touch")
                    .points(self.0.iter().copied())
                    .radius(6.0)
                    .format(move |v| {
                        seen.borrow_mut().push(v);
                        compact(v)
                    }),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let recorded = Rc::new(RefCell::new(Vec::new()));
    let (_, vc) = cx.add_window_view(|_, _| NarrowHex(pts.clone(), recorded.clone()));
    vc.simulate_resize(size(px(240.0), px(240.0)));
    settle(vc);

    let plot = vc.debug_bounds("hexbin-plot").expect("hexbin plot");
    let rect = frame(
        (f32::from(plot.size.width), f32::from(plot.size.height)),
        48.0,
        24.0,
        8.0,
    );
    let layout = compute_layout(&pts, rect, 6.0, None);
    let right_bin = layout
        .bins
        .iter()
        .find(|b| b.center.0 > rect.x + rect.w * 0.6 && b.center.0 < rect.x + rect.w * 0.95)
        .expect("mid-right bin");
    let near_tap = point(
        plot.left() + px(right_bin.center.0 - 18.0),
        plot.top() + px(right_bin.center.1),
    );
    vc.simulate_mouse_down(near_tap, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(near_tap, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip = vc
        .debug_bounds("chart-tooltip")
        .expect("touch_mode 24px target hits small hex bin");
    assert!(
        tip.right() <= plot.right() + px(1.0),
        "right-side tooltip flips inside narrow 240px card"
    );

    // Single-finger down followed by pinch clears drag_last so trailing move does not pan.
    let center = plot.center();
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    settle(vc);
    recorded.borrow_mut().clear();
    vc.simulate_event(PinchEvent {
        position: center,
        delta: 0.5,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc);
    let range = |vals: &[f64]| {
        (
            vals.iter().copied().fold(f64::INFINITY, f64::min),
            vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let after_pinch_range = range(&recorded.borrow());
    recorded.borrow_mut().clear();
    vc.simulate_mouse_move(
        center + point(px(80.0), px(-80.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        center + point(px(80.0), px(-80.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(vc);
    vc.update(|window, _| window.refresh());
    settle(vc);
    assert_eq!(range(&recorded.borrow()), after_pinch_range);

    // Touch scroll pans and Show all button (>= 48px) resets.
    vc.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(24.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);
    let show_all = vc
        .debug_bounds("show-all")
        .expect("show-all after pinch/pan");
    assert!(show_all.size.width >= px(48.0) && show_all.size.height >= px(48.0));
    vc.simulate_click(show_all.center(), Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("show-all").is_none());
}
