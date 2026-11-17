use super::*;

#[test]
fn surface_scene_sorts_quads_back_to_front_and_handles_degenerate_grids() {
    let rect = Rect {
        x: 10.0,
        y: 10.0,
        w: 200.0,
        h: 160.0,
    };
    let empty = build_surface_scene(1, 3, &[1.0, 2.0, 3.0], -38.0, 28.0, 1.0, rect);
    assert!(empty.quads.is_empty());

    let flat = build_surface_scene(3, 3, &[5.0; 9], -38.0, 28.0, 1.0, rect);
    assert_eq!(flat.quads.len(), 4);
    assert!(flat.quads.iter().all(|q| (q.t - 0.5).abs() < 1e-6));

    let hill = SurfaceChart::from_fn("hill", 4, 4, (-1.0, 1.0), (-1.0, 1.0), |x, y| {
        1.0 - (x * x + y * y)
    });
    let scene = build_surface_scene(
        hill.cols,
        hill.rows,
        &hill.values,
        hill.azimuth_deg,
        hill.elevation_deg,
        1.0,
        rect,
    );
    assert_eq!(scene.quads.len(), 9);
    assert!(
        scene
            .quads
            .windows(2)
            .all(|pair| pair[0].depth >= pair[1].depth)
    );

    let zoomed = build_surface_scene(
        hill.cols,
        hill.rows,
        &hill.values,
        hill.azimuth_deg,
        hill.elevation_deg,
        2.0,
        rect,
    );
    assert!((zoomed.quads[0].center.0 - 110.0).abs() > (scene.quads[0].center.0 - 110.0).abs());
}

#[test]
fn parallel_sets_stacks_bars_and_aligns_ribbon_thickness() {
    let dims = ["Class", "Sex", "Survived"].map(SharedString::from);
    let ribbons = vec![
        (vec!["1st".into(), "Female".into(), "Yes".into()], 30.0),
        (vec!["1st".into(), "Male".into(), "No".into()], 20.0),
        (vec!["3rd".into(), "Male".into(), "No".into()], 50.0),
    ];
    let rect = Rect {
        x: 10.0,
        y: 10.0,
        w: 300.0,
        h: 160.0,
    };
    let layout = compute_parallel_sets(&dims, &ribbons, rect, 12.0);
    assert_eq!(layout.bars.len(), 6);
    assert_eq!(layout.segments.len(), 6);
    assert_eq!(layout.total, 100.0);
    // Segments of the same ribbon keep equal thickness across dimensions.
    let s0 = layout.segments[0];
    let s3 = layout.segments[3];
    assert!((s0.thick - s3.thick).abs() < 1e-4);
    let mid = s0.top(0.5);
    assert!(s0.holds((mid.0, mid.1 + s0.thick * 0.5)));
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn surface_and_parallel_sets_render_hover_and_export(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{
        Context, Modifiers, MouseButton, PinchEvent, Render, ScrollDelta, ScrollEvent,
        ScrollSource, TouchPhase, Window, point, px, size,
    };

    struct SurfaceHarness;
    impl Render for SurfaceHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).child(SurfaceChart::from_fn(
                "surf",
                5,
                5,
                (-1.0, 1.0),
                (-1.0, 1.0),
                |x, y| (x * 2.0).cos() + (y * 2.0).sin(),
            ))
        }
    }

    struct ParSetsHarness;
    impl Render for ParSetsHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(360.0)).child(
                ParallelSetsChart::new("titanic", ["Class", "Sex", "Survived"])
                    .ribbon(["1st", "Female", "Yes"], 40.0)
                    .ribbon(["1st", "Male", "No"], 20.0)
                    .ribbon(["3rd", "Male", "No"], 60.0),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        let surface = SurfaceChart::from_grid(
            "export-surf",
            3,
            3,
            [0.0, 1.0, 0.0, 1.0, 2.0, 1.0, 0.0, 1.0, 0.0],
        )
        .angles(-45.0, 30.0)
        .wireframe(true);
        let s_svg = surface.svg(320.0, 220.0, cx);
        assert!(s_svg.starts_with("<svg") && s_svg.contains("<path"));
        assert!(surface.csv().starts_with("col,row,x,y,z\n0,0,"));

        let parsets = ParallelSetsChart::new("export-ps", ["A", "B"])
            .ribbon(["X", "Y"], 10.0)
            .ribbon(["Z", "Y"], 15.0);
        let p_svg = parsets.svg(320.0, 220.0, cx);
        assert!(p_svg.starts_with("<svg") && p_svg.contains("<path") && p_svg.contains("<rect"));
        assert_eq!(parsets.csv(), "A,B,count\nX,Y,10\nZ,Y,15");
    });

    let (_, vc) = cx.add_window_view(|_, _| SurfaceHarness);
    vc.simulate_resize(size(px(400.0), px(320.0)));
    settle(vc);
    let surf_bounds = vc
        .debug_bounds("surface-plot")
        .expect("surface plot bounds");
    vc.simulate_mouse_move(surf_bounds.center(), None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "hovering surface center opens tooltip"
    );
    assert!(vc.debug_bounds("surface-reset").is_none());

    let start = surf_bounds.center();
    let dragged = point(start.x + px(48.0), start.y + px(24.0));
    vc.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(dragged, Some(MouseButton::Left), Modifiers::none());
    vc.simulate_mouse_up(dragged, MouseButton::Left, Modifiers::none());
    settle(vc);
    let reset_bounds = vc
        .debug_bounds("surface-reset")
        .expect("reset button appears after drag orbit");

    vc.simulate_mouse_down(reset_bounds.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_bounds.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_none());

    vc.simulate_event(ScrollEvent {
        position: surf_bounds.center(),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_some());

    vc.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: surf_bounds.center(),
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: surf_bounds.center(),
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_none());

    vc.simulate_event(ScrollEvent {
        position: surf_bounds.center(),
        delta: ScrollDelta::Pixels(point(px(30.0), px(-20.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_some());

    let reset_bounds = vc
        .debug_bounds("surface-reset")
        .expect("reset button appears after touch orbit");
    vc.simulate_mouse_down(reset_bounds.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_bounds.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_none());

    vc.simulate_event(PinchEvent {
        position: surf_bounds.center(),
        delta: 0.35,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_some());

    let (_, vc) = cx.add_window_view(|_, _| ParSetsHarness);
    vc.simulate_resize(size(px(400.0), px(320.0)));
    settle(vc);
    let ps_bounds = vc
        .debug_bounds("parallel-sets-plot")
        .expect("parallel sets plot bounds");
    vc.simulate_mouse_down(ps_bounds.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(ps_bounds.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "pressing parallel sets center opens tooltip"
    );

    let ribbon_pt = point(
        ps_bounds.left() + ps_bounds.size.width * 0.28,
        ps_bounds.top() + ps_bounds.size.height * 0.72,
    );
    let empty_pt = point(ps_bounds.left() + px(4.0), ps_bounds.top() + px(4.0));
    vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    vc.simulate_mouse_move(empty_pt, None, Modifiers::none());
    settle(vc);
    assert!(
        vc.debug_bounds("chart-tooltip").is_some(),
        "pinned ribbon keeps tooltip visible when pointer moves away"
    );

    vc.simulate_mouse_down(empty_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(empty_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_none());

    vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());
    vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(empty_pt, None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_none());

    let legend_0 = vc
        .debug_bounds("chart-legend-entry-0")
        .expect("first-dim legend entry");
    vc.simulate_mouse_down(legend_0.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(legend_0.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    let legend_1 = vc
        .debug_bounds("chart-legend-entry-1")
        .expect("second first-dim legend entry");
    vc.simulate_mouse_down(legend_1.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(legend_1.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn surface_and_parallel_sets_touch_mode_pinch_clear_and_flip(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{
        Context, Modifiers, MouseButton, PinchEvent, Render, TouchPhase, Window, point, px, size,
    };

    struct SurfaceTouchHarness;
    impl Render for SurfaceTouchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(260.0)).child(SurfaceChart::from_fn(
                "surf-touch",
                5,
                5,
                (-1.0, 1.0),
                (-1.0, 1.0),
                |x, y| (x * 2.0).cos() + (y * 2.0).sin(),
            ))
        }
    }

    struct ParSetsTouchHarness;
    impl Render for ParSetsTouchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(260.0)).child(
                ParallelSetsChart::new("ps-touch", ["Class", "Sex", "Survived"])
                    .ribbon(["1st", "Female", "Yes"], 40.0)
                    .ribbon(["1st", "Male", "No"], 20.0)
                    .ribbon(["3rd", "Male", "No"], 60.0),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });

    let (_, vc) = cx.add_window_view(|_, _| SurfaceTouchHarness);
    vc.simulate_resize(size(px(260.0), px(320.0)));
    settle(vc);
    let surf_bounds = vc
        .debug_bounds("surface-plot")
        .expect("surface plot bounds");
    let center = surf_bounds.center();

    // Pinching after mouse_down clears drag_last so subsequent move hovers rather than orbiting.
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    vc.simulate_event(PinchEvent {
        position: center,
        delta: 0.35,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    vc.simulate_mouse_move(center, Some(MouseButton::Left), Modifiers::none());
    vc.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    settle(vc);
    let reset_btn = vc
        .debug_bounds("surface-reset")
        .expect("surface reset after pinch");
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("surface-reset").is_none());

    let (_, vc) = cx.add_window_view(|_, _| ParSetsTouchHarness);
    vc.simulate_resize(size(px(260.0), px(320.0)));
    settle(vc);
    let ps_bounds = vc
        .debug_bounds("parallel-sets-plot")
        .expect("parallel sets plot bounds");
    let ribbon_pt = point(
        ps_bounds.left() + ps_bounds.size.width * 0.28,
        ps_bounds.top() + ps_bounds.size.height * 0.72,
    );
    vc.simulate_mouse_down(ribbon_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(ribbon_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    let clear_btn = vc
        .debug_bounds("parallel-sets-clear")
        .expect("clear button appears when ribbon is pinned");
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    vc.simulate_mouse_down(clear_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(clear_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("parallel-sets-clear").is_none());
    assert!(vc.debug_bounds("chart-tooltip").is_none());
}
