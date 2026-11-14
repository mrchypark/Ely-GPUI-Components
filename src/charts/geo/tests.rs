use super::*;

#[test]
fn rotation_and_projections_clip_and_clamp_accurately() {
    let v = rotate_vec(127.0, 37.5, (-127.0, -37.5));
    assert!((v[0] - 1.0).abs() < 1e-6);
    assert!(v[1].abs() < 1e-6);
    assert!(v[2].abs() < 1e-6);

    let front = rotate_vec(10.0, 0.0, (0.0, 0.0));
    let back = rotate_vec(170.0, 0.0, (0.0, 0.0));
    assert!(project_vec(front, GeoProjection::Orthographic).is_some());
    assert!(project_vec(back, GeoProjection::Orthographic).is_none());

    let pole = rotate_vec(0.0, 90.0, (0.0, 0.0));
    let (mx, my) = project_vec(pole, GeoProjection::Mercator).expect("clamped mercator");
    assert!(mx.abs() < 1e-6);
    assert!((my - mercator_max_y()).abs() < 1e-6);

    let center = project_vec(
        rotate_vec(0.0, 0.0, (0.0, 0.0)),
        GeoProjection::Stereographic,
    );
    assert_eq!(center, Some((0.0, 0.0)));
    let edge = project_vec(
        rotate_vec(90.0, 0.0, (0.0, 0.0)),
        GeoProjection::Stereographic,
    )
    .expect("90 deg visible in stereographic");
    assert!(edge.0.hypot(edge.1) < 1.0);
}

#[test]
fn horizon_clipping_and_point_in_polygon_handle_polygons() {
    let poly = [(-20.0, -20.0), (20.0, -20.0), (20.0, 20.0), (-20.0, 20.0)];
    let screen = vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
    assert!(point_in_polygon((50.0, 50.0), &screen));
    assert!(!point_in_polygon((150.0, 50.0), &screen));

    let vecs: Vec<[f64; 3]> = [(-30.0, -10.0), (130.0, -10.0), (130.0, 10.0), (-30.0, 10.0)]
        .into_iter()
        .map(|(lon, lat)| rotate_vec(lon, lat, (0.0, 0.0)))
        .collect();
    let clipped = clip_ring_horizon(&subdivide_ring(&vecs), 0.0);
    assert!(clipped.len() >= 3);
    assert!(clipped.iter().all(|v| v[0] >= -1e-9));
    let _ = GeoRegion::new("Box", 10.0, poly);
}

#[test]
fn geo_state_clamps_rotation_and_zoom_and_scales_scene() {
    let mut state = GeoState::default();
    assert!(!state.is_modified());

    state.rotate_by(100.0, -500.0, 0.0);
    assert!(state.is_modified());
    assert!(state.yaw_offset > 0.0);
    assert_eq!(state.pitch_offset, 89.0);

    assert!(state.zoom_by(20.0));
    assert_eq!(state.zoom, 12.0);
    assert!(!state.zoom_by(2.0));
    assert!(state.zoom_by(0.01));
    assert_eq!(state.zoom, 0.5);

    let chart = GeoChart::new("scale-check")
        .projection(GeoProjection::Orthographic)
        .point(GeoPoint::new("Center", 0.0, 0.0, 10.0));
    let base_scene = build_scene(
        &chart,
        (300.0, 200.0),
        10.0,
        GeoView {
            rotate: (0.0, 0.0),
            zoom: 1.0,
            pan: (0.0, 0.0),
        },
    );
    let zoomed_scene = build_scene(
        &chart,
        (300.0, 200.0),
        10.0,
        GeoView {
            rotate: (0.0, 0.0),
            zoom: 2.0,
            pan: (0.0, 0.0),
        },
    );
    assert!(
        (zoomed_scene.globe.expect("globe").2 - base_scene.globe.expect("globe").2 * 2.0).abs()
            < 1e-4
    );

    state.reset();
    assert!(!state.is_modified());
    assert_eq!(state.zoom, 1.0);
    assert_eq!(state.yaw_offset, 0.0);
    assert_eq!(state.pitch_offset, 0.0);
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn geo_chart_renders_exports_and_opens_tooltip_on_hover(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, Window, px};

    struct GeoHarness;
    impl Render for GeoHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).h(px(260.0)).child(
                GeoChart::new("geo-test")
                    .projection(GeoProjection::Orthographic)
                    .rotate(-10.0, -10.0)
                    .graticule(30.0)
                    .region(
                        GeoRegion::new(
                            "Atlantis",
                            42.0,
                            [(-20.0, -20.0), (40.0, -20.0), (40.0, 40.0), (-20.0, 40.0)],
                        )
                        .ring([
                            (5.0, 5.0),
                            (15.0, 5.0),
                            (15.0, 15.0),
                            (5.0, 15.0),
                        ]),
                    )
                    .point(GeoPoint::new("Capital", 10.0, 10.0, 100.0)),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        let chart = GeoChart::new("export")
            .projection(GeoProjection::Mercator)
            .region(GeoRegion::new(
                "Alpha, Zone",
                15.0,
                [(-10.0, -10.0), (10.0, -10.0), (10.0, 10.0), (-10.0, 10.0)],
            ))
            .point(GeoPoint::new("Hub", 0.0, 0.0, 50.0));
        let svg = chart.svg(360.0, 240.0, cx);
        assert!(svg.starts_with("<svg") && svg.contains("<path") && svg.contains("<circle"));
        let csv = chart.csv();
        assert!(csv.starts_with("kind,name,lon,lat,value\n"));
        assert!(csv.contains("\"Alpha, Zone\""));
        assert!(csv.contains("point,Hub,0,0,50"));
    });

    let (_, vc) = cx.add_window_view(|_, _| GeoHarness);
    settle(vc);
    assert!(vc.debug_bounds("chart-root").is_some());
    let plot = vc.debug_bounds("geo-plot").expect("geo plot bounds");
    vc.simulate_mouse_move(plot.center(), None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn geo_chart_supports_drag_rotation_zoom_and_reset(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

    struct GeoManipHarness;
    impl Render for GeoManipHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).h(px(260.0)).child(
                GeoChart::new("geo-manip")
                    .projection(GeoProjection::Orthographic)
                    .point(GeoPoint::new("Origin", 0.0, 0.0, 50.0)),
            )
        }
    }

    cx.update(Theme::init);
    let (_, vc) = cx.add_window_view(|_, _| GeoManipHarness);
    settle(vc);

    let plot = vc.debug_bounds("geo-plot").expect("geo plot bounds");
    let center = plot.center();
    assert!(vc.debug_bounds("geo-reset").is_none());

    // Hover at center opens tooltip on the origin point.
    vc.simulate_mouse_move(center, None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    // Left-button drag rotates the globe away from the origin point and reveals Reset.
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(
        center + point(px(140.0), px(-30.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        center + point(px(140.0), px(-30.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(vc);
    assert!(vc.debug_bounds("geo-reset").is_some());
    assert!(vc.debug_bounds("chart-tooltip").is_none());

    // Double-click resets rotation and hides Reset; origin is back at center.
    for click_count in [1, 2] {
        vc.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: center,
            modifiers: Modifiers::none(),
            click_count,
            first_mouse: false,
        });
        vc.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: center,
            modifiers: Modifiers::none(),
            click_count,
        });
        settle(vc);
    }
    assert!(vc.debug_bounds("geo-reset").is_none());
    vc.simulate_mouse_move(center, None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    // Pointer wheel scroll zooms in and shows Reset.
    vc.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    settle(vc);
    let reset_btn = vc
        .debug_bounds("geo-reset")
        .expect("reset after scroll zoom");

    // Clicking Reset restores initial state.
    vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("geo-reset").is_none());

    // Pinch zoom and touch two-axis scroll also update state and double-click resets.
    vc.simulate_event(PinchEvent {
        position: center,
        delta: 0.4,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc);
    assert!(vc.debug_bounds("geo-reset").is_some());

    vc.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: center,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: center,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(vc);
    assert!(vc.debug_bounds("geo-reset").is_none());

    vc.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(40.0), px(-20.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);
    assert!(vc.debug_bounds("geo-reset").is_some());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn geo_chart_touch_mode_uses_24px_hit_radius_clears_drag_on_pinch_and_flips_tooltip(
    cx: &mut gpui::TestAppContext,
) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, Window, point, px, size};

    struct GeoTouchHarness;
    impl Render for GeoTouchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(260.0)).h(px(220.0)).child(
                GeoChart::new("geo-touch")
                    .projection(GeoProjection::Equirectangular)
                    .point(GeoPoint::new("Center", 0.0, 0.0, 40.0))
                    .point(GeoPoint::new("East", 60.0, 0.0, 80.0)),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| GeoTouchHarness);
    vc.simulate_resize(size(px(260.0), px(220.0)));
    settle(vc);

    let plot = vc.debug_bounds("geo-plot").expect("geo plot bounds");
    let center = plot.center();

    // Tapping 18px left of the center point hits it via the >=24px touch radius.
    let near_center = center - point(px(18.0), px(0.0));
    vc.simulate_mouse_down(near_center, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(near_center, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    // Tapping the east point (> 0.55 * width) flips its tooltip to the left.
    let east = point(
        plot.left() + plot.size.width * (0.5 + 60.0 / 360.0),
        center.y,
    );
    vc.simulate_mouse_down(east, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(east, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip = vc.debug_bounds("chart-tooltip").expect("east tooltip");
    assert!(tip.center().x < east.x);

    // Pinching after mouse down clears drag_last so subsequent move does not rotate.
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    vc.simulate_event(PinchEvent {
        position: center,
        delta: 0.35,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    vc.simulate_mouse_move(
        center + point(px(80.0), px(0.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        center + point(px(80.0), px(0.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(vc);

    let reset_btn = vc.debug_bounds("geo-reset").expect("geo reset after pinch");
    // Center point stayed unrotated because pinch cleared drag_last.
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("geo-reset").is_none());
}
