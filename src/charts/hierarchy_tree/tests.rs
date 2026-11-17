use super::*;

fn sample_tree() -> Slice {
    Slice::new("Root", 0.0).children([
        Slice::new("Alpha", 10.0),
        Slice::new("Beta", 0.0).children([
            Slice::new("B1", 6.0),
            Slice::new("B2", 0.0).children([Slice::new("B2a", 4.0)]),
        ]),
    ])
}

#[test]
fn tree_depths_and_cluster_leaves_match_spec() {
    let tree = sample_tree();

    // Orthogonal Tree: Alpha (depth 1) sits at 1/3, B2a (depth 3) sits at 1.0.
    let ortho = compute_layout(
        Some(&tree),
        TreeLayoutMode::Orthogonal,
        true,
        (400.0, 240.0),
        8.0,
        64.0,
        &HashSet::new(),
    );
    let alpha = ortho
        .nodes
        .iter()
        .find(|n| n.name == "Alpha")
        .expect("Alpha");
    let b2 = ortho.nodes.iter().find(|n| n.name == "B2").expect("B2");
    let b2a = ortho.nodes.iter().find(|n| n.name == "B2a").expect("B2a");
    assert!(alpha.x < b2.x && b2.x < b2a.x);

    // Cluster dendrogram: ALL leaves (Alpha, B1, B2a) share the rightmost x coordinate.
    let cluster = compute_layout(
        Some(&tree),
        TreeLayoutMode::Cluster,
        true,
        (400.0, 240.0),
        8.0,
        64.0,
        &HashSet::new(),
    );
    let c_alpha = cluster
        .nodes
        .iter()
        .find(|n| n.name == "Alpha")
        .expect("Alpha");
    let c_b1 = cluster.nodes.iter().find(|n| n.name == "B1").expect("B1");
    let c_b2a = cluster.nodes.iter().find(|n| n.name == "B2a").expect("B2a");
    assert!((c_alpha.x - c_b1.x).abs() < 1e-4);
    assert!((c_b1.x - c_b2a.x).abs() < 1e-4);

    // RadialCluster: ALL leaves lie on the same outermost radius.
    let radial_c = compute_layout(
        Some(&tree),
        TreeLayoutMode::RadialCluster,
        true,
        (300.0, 300.0),
        8.0,
        64.0,
        &HashSet::new(),
    );
    let r_alpha = radial_c
        .nodes
        .iter()
        .find(|n| n.name == "Alpha")
        .expect("Alpha");
    let r_b2a = radial_c
        .nodes
        .iter()
        .find(|n| n.name == "B2a")
        .expect("B2a");
    let dist = |n: &PlacedNode| (n.x - 150.0).hypot(n.y - 150.0);
    assert!((dist(r_alpha) - dist(r_b2a)).abs() < 1e-3);
}

#[test]
fn csv_outputs_depth_path_and_weight() {
    let chart = TreeChart::new("org", sample_tree());
    let csv = chart.csv();
    assert_eq!(
        csv,
        "depth,path,value\n0,Root,20\n1,Root › Alpha,10\n1,Root › Beta,10\n2,Root › Beta › B1,6\n2,Root › Beta › B2,4\n3,Root › Beta › B2 › B2a,4"
    );
}

#[test]
fn collapse_hides_descendants_and_zoom_at_anchors_pointer() {
    let tree = sample_tree();
    let mut collapsed = HashSet::new();
    collapsed.insert("Root\u{1f}Beta".to_string());
    let layout = compute_layout(
        Some(&tree),
        TreeLayoutMode::Orthogonal,
        true,
        (400.0, 240.0),
        8.0,
        64.0,
        &collapsed,
    );
    assert_eq!(layout.nodes.len(), 3);
    let beta = layout
        .nodes
        .iter()
        .find(|n| n.name == "Beta")
        .expect("Beta");
    assert!(beta.is_collapsed);
    assert!(beta.children.is_empty());
    assert_eq!(beta.total_children, 2);
    assert_eq!(beta.weight, 10.0);

    let mut state = TreeState::default();
    state.zoom_at(2.0, (120.0, 80.0));
    assert!((state.zoom - 2.0).abs() < 1e-6);
    assert!((state.pan.0 - (-120.0)).abs() < 1e-6);
    assert!((state.pan.1 - (-80.0)).abs() < 1e-6);
    let anchored_pt = transform_point((120.0, 80.0), state.zoom, state.pan);
    assert!((anchored_pt.0 - 120.0).abs() < 1e-4);
    assert!((anchored_pt.1 - 80.0).abs() < 1e-4);
    state.zoom_at(10.0, (120.0, 80.0));
    assert!((state.zoom - 6.0).abs() < 1e-6);
    state.reset();
    assert!(!state.is_modified());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn tree_chart_renders_exports_and_opens_tooltip_on_hover(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, Window, px};

    struct TreeHarness(TreeLayoutMode);
    impl Render for TreeHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.0)).h(px(280.0)).child(
                TreeChart::new("tree-test", sample_tree())
                    .mode(self.0)
                    .horizontal(true),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        for mode in [
            TreeLayoutMode::Orthogonal,
            TreeLayoutMode::Cluster,
            TreeLayoutMode::RadialTree,
            TreeLayoutMode::RadialCluster,
        ] {
            let chart = TreeChart::new("export", sample_tree()).mode(mode);
            let svg = chart.svg(360.0, 260.0, cx);
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
            assert_eq!(svg.matches("<path").count(), 5);
            assert_eq!(svg.matches("<circle").count(), 6);
        }
    });

    let (_, vc) = cx.add_window_view(|_, _| TreeHarness(TreeLayoutMode::RadialTree));
    settle(vc);
    let root = vc.debug_bounds("chart-root").expect("tree root bounds");
    // In RadialTree, the root node sits directly at the center of chart-root.
    vc.simulate_mouse_move(root.center(), None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn tree_chart_collapses_expands_pans_zooms_and_resets(cx: &mut gpui::TestAppContext) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

    struct TreeHarness;
    impl Render for TreeHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.0)).h(px(280.0)).child(
                TreeChart::new("tree-interact", sample_tree())
                    .mode(TreeLayoutMode::Orthogonal)
                    .horizontal(true)
                    .h(px(280.0)),
            )
        }
    }

    cx.update(Theme::init);
    let (_, vc) = cx.add_window_view(|_, _| TreeHarness);
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("tree root bounds");
    assert!(vc.debug_bounds("tree-reset").is_none());

    let (inset, label_room) = vc.update(|window, cx| {
        let theme = cx.theme();
        let sizes = theme.chart();
        let rem = window.rem_size();
        (
            f32::from(sizes.inset.to_pixels(rem)),
            f32::from(sizes.label.to_pixels(rem)),
        )
    });
    let layout = compute_layout(
        Some(&sample_tree()),
        TreeLayoutMode::Orthogonal,
        true,
        (f32::from(root.size.width), f32::from(root.size.height)),
        inset,
        label_room,
        &HashSet::new(),
    );
    let root_node = layout
        .nodes
        .iter()
        .find(|n| n.name == "Root")
        .expect("Root");
    let beta = layout
        .nodes
        .iter()
        .find(|n| n.name == "Beta")
        .expect("Beta");
    let b2a = layout.nodes.iter().find(|n| n.name == "B2a").expect("B2a");

    let at_node = |n: &PlacedNode| root.origin + point(px(n.x), px(n.y));

    // Clicking internal node Beta collapses its descendants.
    let beta_pt = at_node(beta);
    vc.simulate_mouse_down(beta_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(beta_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_some());

    // Hidden descendant B2a is no longer hit-testable when Beta is collapsed.
    vc.simulate_mouse_move(at_node(b2a), None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_none());

    // Clicking Beta again expands its descendants using its collapsed layout coordinates.
    let mut collapsed = HashSet::new();
    collapsed.insert(beta.path_key.clone());
    let collapsed_layout = compute_layout(
        Some(&sample_tree()),
        TreeLayoutMode::Orthogonal,
        true,
        (f32::from(root.size.width), f32::from(root.size.height)),
        inset,
        label_room,
        &collapsed,
    );
    let collapsed_beta = collapsed_layout
        .nodes
        .iter()
        .find(|n| n.name == "Beta")
        .expect("collapsed Beta");
    let collapsed_beta_pt = at_node(collapsed_beta);
    vc.simulate_mouse_down(collapsed_beta_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(collapsed_beta_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_none());

    // B2a is visible and interactive again.
    vc.simulate_mouse_move(at_node(b2a), None, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    // Wheel scroll zooms around pointer and shows Reset button.
    let zoom_center = root.center();
    vc.simulate_event(ScrollEvent {
        position: zoom_center,
        delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_some());

    // Dragging > 4px pans without toggling Beta even if drag starts on a node.
    let scale = (-120.0_f32 * 0.002).exp().clamp(0.1, 10.0);
    let cx_local = f32::from(zoom_center.x - root.origin.x);
    let cy_local = f32::from(zoom_center.y - root.origin.y);
    let zoom_pan = (cx_local * (1.0 - scale), cy_local * (1.0 - scale));
    let beta_zoomed = transform_point((beta.x, beta.y), scale, zoom_pan);
    let beta_zoomed_pt = root.origin + point(px(beta_zoomed.0), px(beta_zoomed.1));
    vc.simulate_mouse_down(beta_zoomed_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_move(
        beta_zoomed_pt + point(px(16.0), px(10.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        beta_zoomed_pt + point(px(16.0), px(10.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(vc);

    // Touch scroll also pans.
    vc.simulate_event(ScrollEvent {
        position: zoom_center,
        delta: ScrollDelta::Pixels(point(px(8.0), px(-6.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    settle(vc);

    // Verify Root moved to its zoomed + panned screen position and B2a stayed expanded.
    let expected_pan = (zoom_pan.0 + 16.0 + 8.0, zoom_pan.1 + 10.0 - 6.0);
    let root_moved = transform_point((root_node.x, root_node.y), scale, expected_pan);
    vc.simulate_mouse_move(
        root.origin + point(px(root_moved.0), px(root_moved.1)),
        None,
        Modifiers::none(),
    );
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());
    let b2a_moved = transform_point((b2a.x, b2a.y), scale, expected_pan);
    vc.simulate_mouse_move(
        root.origin + point(px(b2a_moved.0), px(b2a_moved.1)),
        None,
        Modifiers::none(),
    );
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    // Clicking Reset restores pan, zoom, and expanded state.
    let reset_btn = vc.debug_bounds("tree-reset").expect("reset button bounds");
    vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_none());

    // Pinch zooms, and double-click resets.
    vc.simulate_event(PinchEvent {
        position: zoom_center,
        delta: 0.35,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_some());

    vc.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: zoom_center,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: zoom_center,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_none());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn tree_chart_touch_mode_uses_24px_hit_radius_clears_drag_on_pinch_and_flips_tooltip(
    cx: &mut gpui::TestAppContext,
) {
    use crate::charts::tests::settle;
    use crate::theme::Theme;
    use gpui::{Context, Modifiers, Render, Window, point, px, size};

    struct TreeTouchHarness;
    impl Render for TreeTouchHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(260.0)).h(px(260.0)).child(
                TreeChart::new("tree-touch", sample_tree())
                    .mode(TreeLayoutMode::Orthogonal)
                    .horizontal(true)
                    .h(px(260.0)),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| TreeTouchHarness);
    vc.simulate_resize(size(px(260.0), px(260.0)));
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("tree root bounds");
    let (inset, label_room) = vc.update(|window, cx| {
        let sizes = cx.theme().chart();
        let rem = window.rem_size();
        (
            f32::from(sizes.inset.to_pixels(rem)),
            f32::from(sizes.label.to_pixels(rem)),
        )
    });
    let layout = compute_layout(
        Some(&sample_tree()),
        TreeLayoutMode::Orthogonal,
        true,
        (f32::from(root.size.width), f32::from(root.size.height)),
        inset,
        label_room,
        &HashSet::new(),
    );
    let beta = layout
        .nodes
        .iter()
        .find(|n| n.name == "Beta")
        .expect("Beta");
    let b2a = layout.nodes.iter().find(|n| n.name == "B2a").expect("B2a");

    // Tapping 20px away from Beta hits it in touch_mode (>= 24px) and collapses its branch.
    let near_beta = root.origin + point(px(beta.x), px(beta.y + 20.0));
    vc.simulate_mouse_down(near_beta, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(near_beta, MouseButton::Left, Modifiers::none());
    settle(vc);
    let reset_btn = vc
        .debug_bounds("tree-reset")
        .expect("tree reset after collapse");
    vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_none());

    // Pinching after mouse_down clears drag_start/drag_last so move does not pan or toggle Beta.
    let beta_pt = root.origin + point(px(beta.x), px(beta.y));
    vc.simulate_mouse_down(beta_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_event(PinchEvent {
        position: root.center(),
        delta: 0.35,
        phase: TouchPhase::Moved,
        modifiers: Modifiers::none(),
    });
    vc.simulate_mouse_move(
        beta_pt + point(px(40.0), px(20.0)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    vc.simulate_mouse_up(
        beta_pt + point(px(40.0), px(20.0)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(vc);
    let reset_btn = vc
        .debug_bounds("tree-reset")
        .expect("tree reset after pinch");

    // B2a (> 0.55 * width) stayed expanded and its zoomed tooltip flips to the left.
    let scale = (0.35_f32).exp().clamp(0.1, 10.0);
    let c_local = (
        f32::from(root.size.width) * 0.5,
        f32::from(root.size.height) * 0.5,
    );
    let zoom_pan = (c_local.0 * (1.0 - scale), c_local.1 * (1.0 - scale));
    let b2a_zoomed = transform_point((b2a.x, b2a.y), scale, zoom_pan);
    let b2a_pt = root.origin + point(px(b2a_zoomed.0), px(b2a_zoomed.1));
    vc.simulate_mouse_down(b2a_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(b2a_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip = vc.debug_bounds("chart-tooltip").expect("B2a tooltip");
    assert!(tip.center().x < b2a_pt.x);

    vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("tree-reset").is_none());
}
