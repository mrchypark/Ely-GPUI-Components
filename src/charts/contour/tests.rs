use super::*;

#[test]
fn marching_squares_and_kde_handle_peaks_flat_and_empty_inputs() {
    let empty = kde_field(&[], (20, 20), None, None);
    assert!(empty.values.is_empty());

    let grid = vec![
        0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let chart = ContourChart::from_grid("g", 4, 4, grid).thresholds([0.5]);
    let field = chart.build_field(None);
    let bands = build_bands(
        &field,
        Linear::new(field.x_domain, (0.0, 90.0)),
        Linear::new(field.y_domain, (90.0, 0.0)),
        4,
        Some(&[0.5]),
    );
    assert_eq!(bands.len(), 1);
    assert!(!bands[0].polygons.is_empty());
    assert!(!bands[0].isolines.is_empty());

    let flat = vec![2.0; 9];
    let flat_chart = ContourChart::from_grid("flat", 3, 3, flat);
    let flat_field = flat_chart.build_field(None);
    let flat_bands = build_bands(
        &flat_field,
        Linear::new(flat_field.x_domain, (0.0, 50.0)),
        Linear::new(flat_field.y_domain, (50.0, 0.0)),
        4,
        None,
    );
    assert!(flat_bands.is_empty());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn contour_chart_renders_kde_and_exports_svg_csv(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{
        Context, Modifiers, PinchEvent, Render, ScrollDelta, ScrollEvent, Window, point, px,
    };
    use std::cell::RefCell;

    struct ContourHarness(Rc<RefCell<Vec<f64>>>);
    impl Render for ContourHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let seen = self.0.clone();
            let pts = vec![
                (20.0, 30.0),
                (22.0, 31.0),
                (24.0, 29.0),
                (70.0, 65.0),
                (72.0, 68.0),
            ];
            div().w(px(360.0)).h(px(260.0)).child(
                ContourChart::from_points("kde-test", pts)
                    .grid_size(16, 16)
                    .levels(4)
                    .format(move |v| {
                        seen.borrow_mut().push(v);
                        compact(v)
                    }),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        let c = ContourChart::from_points("export", [(1.0, 2.0), (2.0, 3.0), (5.0, 6.0)])
            .grid_size(12, 12)
            .levels(3);
        let svg = c.svg(320.0, 220.0, cx);
        assert!(svg.starts_with("<svg") && svg.contains("<path"));
        assert!(c.csv().starts_with("col,row,x,y,value\n"));
    });

    let recorded = Rc::new(RefCell::new(Vec::new()));
    let (_, vc) = cx.add_window_view(|_, _| ContourHarness(recorded.clone()));
    settle(vc);
    let bounds = vc
        .debug_bounds("contour-plot")
        .expect("contour plot bounds");
    let rect_center = point(
        bounds.left() + px(48.0) + (bounds.size.width - px(56.0)) * 0.5,
        bounds.top() + px(8.0) + (bounds.size.height - px(36.0)) * 0.5,
    );
    vc.simulate_mouse_move(rect_center, None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "hover over contour opens tooltip"
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
    assert!(zoomed_span < 70.0, "wheel zoom shrinks visible domain");

    // Left-button drag (> 3px) pans the domain.
    recorded.borrow_mut().clear();
    vc.simulate_mouse_down(rect_center, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(
        rect_center + point(px(90.0), px(-90.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        rect_center + point(px(90.0), px(-90.0)),
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

    // Verify grid source also supports zoom, pan, and reset.
    struct GridHarness;
    impl Render for GridHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let grid = vec![
                0.0, 0.2, 0.2, 0.0, 0.2, 1.0, 1.0, 0.2, 0.2, 1.0, 1.0, 0.2, 0.0, 0.2, 0.2, 0.0,
            ];
            div()
                .w(px(360.0))
                .h(px(260.0))
                .child(ContourChart::from_grid("grid-test", 4, 4, grid).levels(3))
        }
    }
    let (_, gvc) = cx.add_window_view(|_, _| GridHarness);
    settle(gvc);
    let gbounds = gvc
        .debug_bounds("contour-plot")
        .expect("grid contour plot bounds");
    let gcenter = point(
        gbounds.left() + px(48.0) + (gbounds.size.width - px(56.0)) * 0.5,
        gbounds.top() + px(8.0) + (gbounds.size.height - px(36.0)) * 0.5,
    );
    gvc.simulate_event(ScrollEvent {
        position: gcenter,
        delta: ScrollDelta::Pixels(point(px(0.0), px(150.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    settle(gvc);
    assert!(gvc.debug_bounds("show-all").is_some());
    gvc.simulate_mouse_down(gcenter, MouseButton::Left, Modifiers::none());
    gvc.simulate_mouse_move(
        gcenter + point(px(20.0), px(0.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    gvc.simulate_mouse_up(
        gcenter + point(px(20.0), px(0.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(gvc);
    let greset = gvc.debug_bounds("show-all").expect("grid show-all");
    gvc.simulate_click(greset.center(), Modifiers::none());
    settle(gvc);
    assert!(gvc.debug_bounds("show-all").is_none());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn contour_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};
    use std::cell::RefCell;

    struct NarrowContour(Rc<RefCell<Vec<f64>>>);
    impl Render for NarrowContour {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let seen = self.0.clone();
            let pts = vec![(10.0, 20.0), (50.0, 50.0), (90.0, 80.0)];
            div().w(px(240.0)).h(px(240.0)).child(
                ContourChart::from_points("contour-touch", pts)
                    .grid_size(12, 12)
                    .levels(3)
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
    let (_, vc) = cx.add_window_view(|_, _| NarrowContour(recorded.clone()));
    vc.simulate_resize(size(px(240.0), px(240.0)));
    settle(vc);

    let plot = vc.debug_bounds("contour-plot").expect("contour plot");
    let right_tap = point(plot.left() + plot.size.width * 0.78, plot.center().y);
    vc.simulate_mouse_down(right_tap, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(right_tap, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip = vc.debug_bounds("chart-tooltip").expect("tap opens tooltip");
    assert!(
        tip.right() <= plot.right() + px(1.0),
        "right-side tooltip flips inside narrow 240px card"
    );

    // Single-finger down followed by pinch clears drag_last so trailing move does not pan.
    let center = plot.center();
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    settle(vc);
    recorded.borrow_mut().clear();
    vc.simulate_event(PinchEvent {
        position: center,
        delta: 0.5,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    recorded.borrow_mut().clear();
    vc.update(|window, _| window.refresh());
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
        center + point(px(70.0), px(-70.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        center + point(px(70.0), px(-70.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    recorded.borrow_mut().clear();
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
