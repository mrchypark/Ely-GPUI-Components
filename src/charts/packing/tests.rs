use super::*;

#[test]
fn circle_packing_places_siblings_without_overlap_inside_outer_bound() {
    let slices = vec![
        Slice::new("A", 0.0).children([Slice::new("A1", 10.0), Slice::new("A2", 25.0)]),
        Slice::new("B", 18.0),
        Slice::new("C", 9.0),
    ];
    let packed = pack_hierarchy(&slices, (100.0, 100.0), 90.0);
    assert_eq!(packed.len(), 5);
    let top: Vec<_> = packed.iter().filter(|c| c.depth == 0).collect();
    for i in 0..top.len() {
        assert!((top[i].cx - 100.0).hypot(top[i].cy - 100.0) + top[i].r <= 90.5);
        for j in i + 1..top.len() {
            let d = (top[i].cx - top[j].cx).hypot(top[i].cy - top[j].cy);
            assert!(d + 0.5 >= top[i].r + top[j].r);
        }
    }
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn branch_click_drills_down_and_outside_up_and_double_click_return(cx: &mut gpui::TestAppContext) {
    use gpui::{Context, Modifiers, Render, point, px, size};

    use crate::charts::tests::settle;
    use crate::theme::Theme;

    fn sample() -> Vec<Slice> {
        vec![
            Slice::new("Design", 0.0).children([
                Slice::new("Brand", 0.0)
                    .children([Slice::new("Logo", 10.0), Slice::new("Type", 10.0)]),
                Slice::new("Web", 20.0),
            ]),
            Slice::new("Ops", 40.0),
        ]
    }

    fn hit_point(slices: &[Slice], name: &str, origin: gpui::Point<Pixels>) -> gpui::Point<Pixels> {
        let circles = pack_hierarchy(slices, (200.0, 200.0), 184.0);
        let target_ix = circles
            .iter()
            .position(|c| c.path.last().map(SharedString::as_ref) == Some(name))
            .expect("target circle");
        let c = &circles[target_ix];
        for frac in [0.0, 0.78, 0.96] {
            for step in 0..16 {
                let angle = TAU * step as f32 / 16.0;
                let pt = (
                    c.cx + c.r * frac * angle.cos(),
                    c.cy + c.r * frac * angle.sin(),
                );
                if circle_under(&circles, pt) == Some(target_ix) {
                    return origin + point(px(pt.0), px(pt.1));
                }
            }
        }
        panic!("no uncovered hit point for {name}");
    }

    struct PackDrill;

    impl Render for PackDrill {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            CirclePacking::new("pack-drill", sample())
                .w(px(400.0))
                .h(px(400.0))
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| PackDrill);
    vc.simulate_resize(size(px(400.0), px(400.0)));
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("packing root");
    let data = sample();
    let ops_pt = hit_point(&data, "Ops", root.origin);
    let design_pt = hit_point(&data, "Design", root.origin);
    let outside_pt = root.origin + point(px(4.0), px(4.0));

    // Clicking a leaf circle shows the tooltip and does not drill down.
    vc.simulate_mouse_down(ops_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(ops_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());
    assert!(vc.debug_bounds("packing-up").is_none());

    // Clicking a branch circle drills down into its children.
    vc.simulate_mouse_down(design_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(design_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("packing-up").is_some());

    // Drilling one level deeper into Brand, then clicking outside all circles pops up one level.
    let brand_pt = hit_point(&data[0].children, "Brand", root.origin);
    vc.simulate_mouse_down(brand_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(brand_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    vc.simulate_mouse_down(outside_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(outside_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("packing-up").is_some());

    // Clicking the Up button pops back to root.
    let up_btn = vc.debug_bounds("packing-up").expect("up button").center();
    vc.simulate_mouse_down(up_btn, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(up_btn, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("packing-up").is_none());

    // Double-clicking resets to root.
    vc.simulate_mouse_down(design_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(design_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("packing-up").is_some());
    vc.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: root.center(),
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: root.center(),
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(vc);
    assert!(vc.debug_bounds("packing-up").is_none());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn packing_narrow_touch_card_drilldown_up_and_tooltip_flip(cx: &mut gpui::TestAppContext) {
    use gpui::{Context, Modifiers, Render, point, px, size};

    use crate::charts::tests::settle;
    use crate::theme::Theme;

    let slices = vec![
        Slice::new("Very Long Branch Name", 0.0)
            .children([Slice::new("Leaf A", 15.0), Slice::new("Leaf B", 25.0)]),
        Slice::new("Small", 1.0),
    ];
    struct NarrowPack(Vec<Slice>);
    impl Render for NarrowPack {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            CirclePacking::new("pack-narrow", self.0.clone())
                .w(px(240.0))
                .h(px(260.0))
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| NarrowPack(slices.clone()));
    vc.simulate_resize(size(px(240.0), px(260.0)));
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("packing root");
    let circles = pack_hierarchy(&slices, (120.0, 130.0), 120.0 * 0.92);
    let branch = circles.iter().find(|c| c.is_branch).expect("branch circle");
    let branch_ix = circles
        .iter()
        .position(|c| c.is_branch)
        .expect("branch index");
    let branch_pt = (0..16)
        .find_map(|step| {
            let a = TAU * step as f32 / 16.0;
            let pt = (
                branch.cx + branch.r * 0.94 * a.cos(),
                branch.cy + branch.r * 0.94 * a.sin(),
            );
            (circle_under(&circles, pt) == Some(branch_ix))
                .then(|| root.origin + point(px(pt.0), px(pt.1)))
        })
        .expect("uncovered branch rim");

    // Tapping just outside the small leaf circle (within 24px touch radius) opens tooltip.
    let small = circles
        .iter()
        .find(|c| c.path.last().map(SharedString::as_ref) == Some("Small"))
        .expect("small leaf");
    let small_dx = small.r + 4.0;
    let small_dir = if small.cx > 120.0 { 1.0 } else { -1.0 };
    let small_near = root.origin + point(px(small.cx + small_dir * small_dx), px(small.cy));
    assert!(circle_under(&circles, (small.cx + small_dir * small_dx, small.cy)).is_none());
    vc.simulate_mouse_down(small_near, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(small_near, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("chart-tooltip").is_some());

    vc.simulate_mouse_down(branch_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(branch_pt, MouseButton::Left, Modifiers::none());
    settle(vc);

    let crumb = vc
        .debug_bounds("packing-breadcrumb")
        .expect("breadcrumb bar");
    let up = vc.debug_bounds("packing-up").expect("Up button");
    assert!(up.size.width >= px(48.0) && up.size.height >= px(48.0));
    assert!(crumb.left() >= root.left() && crumb.right() <= root.right() + px(1.0));
    assert!(up.right() <= root.right() + px(1.0));

    // Tap a child on the right half so tooltip opens and flips inside narrow 240px card.
    let children = pack_hierarchy(&slices[0].children, (120.0, 130.0), 120.0 * 0.92);
    let right_child = children
        .iter()
        .max_by(|a, b| a.cx.total_cmp(&b.cx))
        .expect("rightmost child");
    let leaf_pt = root.origin + point(px(right_child.cx), px(right_child.cy));
    vc.simulate_mouse_down(leaf_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(leaf_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    let tip = vc.debug_bounds("chart-tooltip").expect("leaf tooltip");
    assert!(tip.right() <= root.right() + px(1.0));

    vc.simulate_click(up.center(), Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("packing-up").is_none());
}
