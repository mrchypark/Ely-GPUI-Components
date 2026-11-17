use super::*;

#[test]
fn horizon_splits_bands_and_crossings_cleanly() {
    let row = Rect {
        x: 0.0,
        y: 10.0,
        w: 100.0,
        h: 40.0,
    };
    let patches = build_band_patches(&[0.0, 100.0], &[-10.0, 10.0], row, 2, 5.0, true);
    assert!(
        patches.iter().any(|p| p.positive && p.band == 0)
            && patches.iter().any(|p| !p.positive && p.band == 0),
        "crossing zero produces both positive and negative band patches"
    );
    for patch in &patches {
        for &(x, y) in &patch.points {
            assert!((0.0..=100.0).contains(&x), "x stays in row: {x}");
            assert!(
                y >= row.y - 1e-3 && y <= row.y + row.h + 1e-3,
                "y stays in row: {y}"
            );
        }
    }

    let offset_patches = build_band_patches(&[0.0, 100.0], &[-5.0, -5.0], row, 2, 5.0, false);
    assert_eq!(offset_patches.len(), 1);
    assert!(!offset_patches[0].positive);
    assert!(
        offset_patches[0]
            .points
            .iter()
            .any(|&(_, y)| (y - row.y).abs() < 1e-3),
        "offset mode anchors negative band to row top"
    );
}

#[test]
fn radial_layout_maps_rings_and_envelope_within_outer_radius() {
    let series = [Series::new("Temp", [10.0, 20.0, 15.0, 12.0])];
    let refs: Vec<&Series> = series.iter().collect();
    let band = (vec![5.0, 15.0, 10.0, 8.0], vec![14.0, 25.0, 19.0, 16.0]);
    let layout = compute_radial(4, &refs, Some(&band), (120.0, 120.0), 80.0, 0.0);
    assert_eq!(layout.series_pts[0].len(), 4);
    let (low_pts, high_pts) = layout.band.expect("range band");
    for (lp, hp) in low_pts.iter().zip(&high_pts) {
        let dl = (lp.0 - 120.0).hypot(lp.1 - 120.0);
        let dh = (hp.0 - 120.0).hypot(hp.1 - 120.0);
        assert!(dl <= dh + 1e-3 && dh <= 80.0 + 1e-3);
    }
    assert_eq!(radial_spoke_at(4, (100.0, 100.0), (100.0, 40.0), 0.0), 0);
    assert_eq!(radial_spoke_at(4, (100.0, 100.0), (160.0, 100.0), 0.0), 1);
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn horizon_and_radial_render_export_and_open_tooltips(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, Window, px};

    struct HorizonHarness;
    impl Render for HorizonHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).h(px(280.0)).child(
                HorizonChart::new("hz", ["T1", "T2", "T3", "T4"])
                    .series(Series::new("Cluster A", [12.0, -8.0, 25.0, -18.0]))
                    .series(Series::new("Cluster B", [4.0, 9.0, -6.0, 14.0]))
                    .bands(4)
                    .baseline(0.0)
                    .mode_mirror(true),
            )
        }
    }

    struct RadialHarness;
    impl Render for RadialHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).h(px(280.0)).child(
                RadialLineChart::new("rad", ["Jan", "Apr", "Jul", "Oct"])
                    .series(Series::new("Mean", [4.0, 14.0, 24.0, 13.0]))
                    .range_band([0.0, 8.0, 18.0, 7.0], [8.0, 20.0, 30.0, 19.0])
                    .filled(true),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        let horizon = HorizonChart::new("hz-exp", ["A", "B", "C"])
            .series(Series::new("Load", [1.0, -2.0, 3.5]))
            .bands(3);
        assert_eq!(horizon.csv(), ",A,B,C\nLoad,1,-2,3.5");
        let h_svg = horizon.svg(320.0, 200.0, cx);
        assert!(h_svg.starts_with("<svg") && h_svg.contains("<path"));

        let radial = RadialLineChart::new("rad-exp", ["N", "E", "S", "W"])
            .series(Series::new("Temp", [10.0, 20.0, 15.0, 5.0]))
            .range_band([5.0, 12.0, 9.0, 2.0], [14.0, 25.0, 21.0, 9.0])
            .filled(true);
        assert!(radial.csv().contains("Temp,10,20,15,5"));
        assert!(radial.csv().contains("High,14,25,21,9"));
        let r_svg = radial.svg(280.0, 280.0, cx);
        assert!(r_svg.starts_with("<svg") && r_svg.contains("<circle") && r_svg.contains("<path"));
    });

    let (_, vc_h) = cx.add_window_view(|_, _| HorizonHarness);
    vc_h.simulate_resize(gpui::size(px(360.0), px(280.0)));
    settle(vc_h);
    let h_root = vc_h.debug_bounds("chart-root").expect("horizon root");
    let row_label = vc_h
        .debug_bounds("horizon-row-label")
        .expect("horizon row label");
    assert!(
        row_label.left() >= h_root.left(),
        "label clipped on left: {row_label:?} vs {h_root:?}"
    );
    vc_h.simulate_mouse_move(h_root.center(), None, Modifiers::none());
    settle(vc_h);
    assert!(vc_h.debug_bounds("chart-tooltip").is_some());

    let (_, vc_r) = cx.add_window_view(|_, _| RadialHarness);
    vc_r.simulate_resize(gpui::size(px(360.0), px(280.0)));
    settle(vc_r);
    let r_root = vc_r.debug_bounds("chart-root").expect("radial root");
    vc_r.simulate_mouse_move(r_root.center(), None, Modifiers::none());
    settle(vc_r);
    assert!(vc_r.debug_bounds("chart-tooltip").is_some());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn horizon_chart_supports_legend_brush_scroll_zoom_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

    struct HorizonInteractiveHarness;
    impl Render for HorizonInteractiveHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.0)).h(px(280.0)).child(
                HorizonChart::new("hz-int", ["T1", "T2", "T3", "T4", "T5", "T6"])
                    .series(Series::new("CPU", [12.0, -8.0, 25.0, -18.0, 10.0, 4.0]))
                    .series(Series::new("Mem", [4.0, 9.0, -6.0, 14.0, -2.0, 8.0])),
            )
        }
    }

    cx.update(Theme::init);
    let (_, vc) = cx.add_window_view(|_, _| HorizonInteractiveHarness);
    vc.simulate_resize(gpui::size(px(400.0), px(280.0)));
    settle(vc);

    let legend_entry = vc
        .debug_bounds("chart-legend-entry-0")
        .expect("multi-series horizon legend renders");
    vc.simulate_mouse_down(legend_entry.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(legend_entry.center(), MouseButton::Left, Modifiers::none());
    settle(vc);

    let plot = vc
        .debug_bounds("horizon-plot")
        .expect("horizon plot bounds");
    assert!(vc.debug_bounds("chart-show-all").is_none());

    let y = plot.center().y;
    let from = point(plot.left() + plot.size.width * 0.25, y);
    let to = point(plot.left() + plot.size.width * 0.75, y);
    vc.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
    vc.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-show-all").is_some(),
        "horizontal drag brush zooms span and shows reset button"
    );

    let show_all = vc.debug_bounds("chart-show-all").expect("show-all button");
    vc.simulate_mouse_down(show_all.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(show_all.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-show-all").is_none(),
        "Show all button resets zoomed span"
    );

    vc.simulate_event(ScrollEvent {
        position: plot.center(),
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
        position: plot.center(),
        button: MouseButton::Left,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_mouse_up(plot.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-show-all").is_none(),
        "double-click resets zoomed span"
    );
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn radial_line_chart_supports_legend_pin_rotation_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, Window, point, px};

    struct RadialInteractiveHarness;
    impl Render for RadialInteractiveHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).h(px(300.0)).child(
                RadialLineChart::new("rad-int", ["Jan", "Apr", "Jul", "Oct"])
                    .series(Series::new("Mean", [4.0, 14.0, 24.0, 13.0]))
                    .series(Series::new("Peak", [8.0, 18.0, 28.0, 17.0])),
            )
        }
    }

    cx.update(Theme::init);
    let (_, vc) = cx.add_window_view(|_, _| RadialInteractiveHarness);
    vc.simulate_resize(gpui::size(px(360.0), px(300.0)));
    settle(vc);

    let legend_entry = vc
        .debug_bounds("chart-legend-entry-1")
        .expect("multi-series radial legend renders");
    vc.simulate_mouse_down(legend_entry.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(legend_entry.center(), MouseButton::Left, Modifiers::none());
    settle(vc);

    let plot = vc.debug_bounds("radial-plot").expect("radial plot bounds");
    let top_spoke = point(plot.center().x, plot.center().y - px(45.0));
    vc.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(top_spoke, MouseButton::Left, Modifiers::none());
    settle(vc);

    // Moving pointer outside keeps pinned tooltip visible.
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    let tip_before = vc
        .debug_bounds("chart-tooltip")
        .expect("pinned spoke keeps tooltip visible");

    // Drag from 12 o'clock to 3 o'clock to rotate dial clockwise ~90 deg.
    let right_spoke = point(plot.center().x + px(45.0), plot.center().y);
    vc.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(right_spoke, Some(MouseButton::Left), Modifiers::none());
    vc.simulate_mouse_up(right_spoke, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip_after = vc
        .debug_bounds("chart-tooltip")
        .expect("rotated pinned spoke keeps tooltip visible");
    assert!(
        tip_after.top() > tip_before.top() + px(20.0) && tip_after.origin != tip_before.origin,
        "rotating clockwise shifts pinned spoke tooltip down around the dial"
    );

    // Double-click resets rotation and pinned state.
    vc.simulate_event(MouseDownEvent {
        position: plot.center(),
        button: MouseButton::Left,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_mouse_up(plot.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_none(),
        "double-click clears pinned state"
    );
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn horizon_and_radial_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};

    struct NarrowHorizon;
    impl Render for NarrowHorizon {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(240.0)).h(px(240.0)).child(
                HorizonChart::new("hz-touch", ["T1", "T2", "T3", "T4", "T5", "T6"])
                    .series(Series::new("CPU", [12.0, -8.0, 25.0, -18.0, 10.0, 4.0]))
                    .series(Series::new("Mem", [4.0, 9.0, -6.0, 14.0, -2.0, 8.0])),
            )
        }
    }

    struct NarrowRadial;
    impl Render for NarrowRadial {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(240.0)).h(px(240.0)).child(
                RadialLineChart::new("rad-touch", ["Jan", "Apr", "Jul", "Oct"])
                    .series(Series::new("Mean", [4.0, 14.0, 24.0, 13.0]))
                    .series(Series::new("Peak", [8.0, 18.0, 28.0, 17.0])),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });

    // HorizonChart touch_mode: tap pin, right-side tooltip flip, pinch clears brush, touch swipe pan, reset.
    let (_, vc_h) = cx.add_window_view(|_, _| NarrowHorizon);
    vc_h.simulate_resize(gpui::size(px(240.0), px(240.0)));
    settle(vc_h);
    let h_plot = vc_h.debug_bounds("horizon-plot").expect("horizon plot");
    let right_tap = point(h_plot.left() + h_plot.size.width * 0.78, h_plot.center().y);
    vc_h.simulate_mouse_down(right_tap, MouseButton::Left, Modifiers::none());
    vc_h.simulate_mouse_up(right_tap, MouseButton::Left, Modifiers::none());
    settle(vc_h);
    let tip = vc_h
        .debug_bounds("chart-tooltip")
        .expect("tap pins tooltip");
    assert!(
        tip.right() <= h_plot.right() + px(1.0),
        "right-side tooltip flips inside narrow 240px card"
    );
    let h_reset = vc_h
        .debug_bounds("chart-show-all")
        .expect("pinned shows reset button");
    assert!(h_reset.size.width >= px(48.0) && h_reset.size.height >= px(48.0));
    vc_h.simulate_click(h_reset.center(), Modifiers::none());
    settle(vc_h);
    assert!(vc_h.debug_bounds("chart-show-all").is_none());

    // Start brush, pinch clears brush so trailing move does not brush-zoom.
    let h_from = point(h_plot.left() + h_plot.size.width * 0.3, h_plot.center().y);
    let h_to = point(h_plot.left() + h_plot.size.width * 0.9, h_plot.center().y);
    vc_h.simulate_mouse_down(h_from, MouseButton::Left, Modifiers::none());
    vc_h.simulate_event(PinchEvent {
        position: h_plot.center(),
        delta: 0.6,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc_h);
    vc_h.simulate_mouse_move(h_to, Some(MouseButton::Left), Modifiers::none());
    vc_h.simulate_mouse_up(h_to, MouseButton::Left, Modifiers::none());
    settle(vc_h);
    vc_h.simulate_event(ScrollEvent {
        position: h_plot.center(),
        delta: ScrollDelta::Pixels(point(px(28.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc_h);
    let h_show_all = vc_h
        .debug_bounds("chart-show-all")
        .expect("show-all after pinch/pan");
    vc_h.simulate_click(h_show_all.center(), Modifiers::none());
    settle(vc_h);
    assert!(vc_h.debug_bounds("chart-show-all").is_none());

    vc_h.simulate_event(gpui::TouchDragEvent {
        phase: TouchPhase::Started,
        start_position: h_from,
        position: h_to,
    });
    vc_h.simulate_event(gpui::TouchDragEvent {
        phase: TouchPhase::Ended,
        start_position: h_from,
        position: h_to,
    });
    settle(vc_h);
    assert!(vc_h.debug_bounds("chart-show-all").is_some());

    // RadialLineChart touch_mode: tap pin, pinch clears drag, touch swipe rotates, Reset button resets.
    let (_, vc_r) = cx.add_window_view(|_, _| NarrowRadial);
    vc_r.simulate_resize(gpui::size(px(240.0), px(240.0)));
    settle(vc_r);
    let r_plot = vc_r.debug_bounds("radial-plot").expect("radial plot");
    let top_spoke = point(r_plot.center().x, r_plot.center().y - px(32.0));
    let right_spoke = point(r_plot.center().x + px(32.0), r_plot.center().y);
    vc_r.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
    vc_r.simulate_mouse_up(top_spoke, MouseButton::Left, Modifiers::none());
    settle(vc_r);
    assert!(vc_r.debug_bounds("chart-tooltip").is_some());
    let r_reset = vc_r
        .debug_bounds("radial-reset")
        .expect("pinned spoke shows Reset button");
    assert!(r_reset.size.width >= px(48.0) && r_reset.size.height >= px(48.0));
    vc_r.simulate_click(r_reset.center(), Modifiers::none());
    settle(vc_r);
    assert!(vc_r.debug_bounds("radial-reset").is_none());

    // Press down, pinch clears drag so move does not rotate, then touch scroll rotates and Reset clears.
    vc_r.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
    vc_r.simulate_event(PinchEvent {
        position: r_plot.center(),
        delta: 0.4,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc_r);
    vc_r.simulate_mouse_move(right_spoke, Some(MouseButton::Left), Modifiers::none());
    vc_r.simulate_mouse_up(right_spoke, MouseButton::Left, Modifiers::none());
    settle(vc_r);
    assert!(
        vc_r.debug_bounds("radial-reset").is_some(),
        "pinch zoom shows radial-reset"
    );
    vc_r.simulate_event(ScrollEvent {
        position: r_plot.center(),
        delta: ScrollDelta::Pixels(point(px(36.0), px(0.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc_r);
    let r_reset = vc_r
        .debug_bounds("radial-reset")
        .expect("reset button visible");
    vc_r.simulate_click(r_reset.center(), Modifiers::none());
    settle(vc_r);
    assert!(vc_r.debug_bounds("radial-reset").is_none());
}
