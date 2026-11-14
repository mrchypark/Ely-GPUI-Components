use std::f32::consts::{FRAC_PI_2, TAU};

use gpui::{px, size};

use super::*;

#[test]
fn a_child_takes_only_its_share_of_its_parent() {
    let arcs = rings(&[Slice::new("Whole", 100.0).children([Slice::new("Part", 25.0)])]);
    let part = arcs.iter().find(|arc| arc.depth == 1).expect("the part");
    assert!(
        (part.sweep - TAU / 4.0).abs() < 1e-5,
        "a quarter of the ring, got {}",
        part.sweep
    );
}

#[test]
fn rings_nest_children_inside_their_parent() {
    let tree = [
        Slice::new("Design", 0.0).children([Slice::new("Brand", 1.0), Slice::new("Web", 3.0)]),
        Slice::new("Ops", 4.0),
    ];
    let arcs = rings(&tree);
    assert_eq!(arcs.len(), 4);
    assert_eq!(
        (arcs[0].depth, arcs[0].sweep, arcs[0].weight),
        (0, TAU / 2.0, 4.0),
        "Design weighs its children"
    );
    let web = arcs
        .iter()
        .find(|arc| arc.path.last() == Some(&"Web".into()))
        .expect("Web");
    assert_eq!(
        (web.depth, web.start, web.sweep, web.branch),
        (1, -FRAC_PI_2 + TAU / 8.0, TAU * 3.0 / 8.0, 0)
    );
}

#[test]
fn a_point_lands_on_the_arc_under_it() {
    let bounds = gpui::Bounds::new(gpui::point(px(0.0), px(0.0)), size(px(100.0), px(100.0)));
    let arcs = rings(&[Slice::new("a", 1.0)]);
    assert_eq!(
        arc_under(
            &arcs,
            bounds,
            gpui::point(px(50.0), px(20.0)),
            17.0,
            33.0,
            50.0
        )
        .flatten(),
        Some(0),
        "a point in the ring lands on the arc"
    );
    assert_eq!(
        arc_under(
            &arcs,
            bounds,
            gpui::point(px(50.0), px(45.0)),
            17.0,
            33.0,
            50.0
        ),
        None,
        "the hole holds no arc"
    );
    assert_eq!(
        arc_under(
            &arcs,
            bounds,
            gpui::point(px(500.0), px(500.0)),
            17.0,
            33.0,
            50.0
        ),
        None,
        "past the outer edge lands on nothing"
    );
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn branch_click_drills_down_and_hole_up_and_double_click_return(cx: &mut gpui::TestAppContext) {
    use gpui::{Context, Modifiers, Render, point};

    use crate::charts::tests::settle;
    use crate::theme::Theme;

    struct SunDrill;

    impl Render for SunDrill {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Sunburst::new(
                "sun-drill",
                [
                    Slice::new("Design", 0.0).children([
                        Slice::new("Brand", 0.0)
                            .children([Slice::new("Logo", 1.0), Slice::new("Type", 1.0)]),
                        Slice::new("Web", 2.0),
                    ]),
                    Slice::new("Ops", 4.0),
                ],
            )
            .w(px(400.0))
            .h(px(400.0))
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| SunDrill);
    vc.simulate_resize(size(px(400.0), px(400.0)));
    settle(vc);

    assert!(vc.debug_bounds("sunburst-up").is_none());

    let root = vc.debug_bounds("chart-root").expect("sunburst root");
    let center = root.center();
    let design_pt = point(center.x + px(90.0), center.y);
    let leaf_pt = point(center.x - px(90.0), center.y);

    // Clicking a branch slice drills down into its children.
    vc.simulate_mouse_down(design_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(design_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-up").is_some());

    // Clicking a leaf slice keeps the tooltip open without drilling down.
    vc.simulate_mouse_down(leaf_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(leaf_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-tooltip").is_some());
    assert!(vc.debug_bounds("sunburst-up").is_some());

    // Clicking a nested branch drills down a second level, and clicking the center hole pops one level.
    vc.simulate_mouse_down(design_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(design_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-up").is_some());

    // Clicking the Up button pops back to root.
    let up_btn = vc.debug_bounds("sunburst-up").expect("up button").center();
    vc.simulate_mouse_down(up_btn, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(up_btn, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-up").is_none());

    // Double-clicking anywhere resets back to root.
    vc.simulate_mouse_down(design_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(design_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-up").is_some());
    vc.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: leaf_pt,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    vc.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: leaf_pt,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(vc);
    assert!(vc.debug_bounds("sunburst-up").is_none());
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn sunburst_narrow_touch_card_drilldown_and_up_button(cx: &mut gpui::TestAppContext) {
    use gpui::{Context, Modifiers, Render, point, px, size};

    use crate::charts::tests::settle;
    use crate::theme::Theme;

    struct NarrowSun;
    impl Render for NarrowSun {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Sunburst::new(
                "sun-narrow",
                [
                    Slice::new("Very Long Branch Name", 0.0)
                        .children([Slice::new("Sub A", 2.0), Slice::new("Sub B", 2.0)]),
                    Slice::new("Ops", 4.0),
                ],
            )
            .w(px(240.0))
            .h(px(240.0))
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| NarrowSun);
    vc.simulate_resize(size(px(240.0), px(240.0)));
    settle(vc);

    let root = vc.debug_bounds("chart-root").expect("sunburst root");
    let center = root.center();
    let branch_pt = point(center.x + px(60.0), center.y);
    vc.simulate_mouse_down(branch_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(branch_pt, MouseButton::Left, Modifiers::none());
    settle(vc);

    let crumb = vc
        .debug_bounds("sunburst-breadcrumb")
        .expect("breadcrumb bar");
    let up = vc.debug_bounds("sunburst-up").expect("Up button");
    assert!(up.size.width >= px(48.0) && up.size.height >= px(48.0));
    assert!(crumb.left() >= root.left() && crumb.right() <= root.right() + px(1.0));
    assert!(up.right() <= root.right() + px(1.0));

    let leaf_pt = point(center.x - px(60.0), center.y);
    vc.simulate_mouse_down(leaf_pt, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(leaf_pt, MouseButton::Left, Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-tooltip").is_some());

    vc.simulate_click(up.center(), Modifiers::none());
    settle(vc);
    assert!(vc.debug_bounds("sunburst-up").is_none());
}
