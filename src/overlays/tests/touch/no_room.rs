use super::*;

fn areas() -> [Bounds<Pixels>; 6] {
    [
        Bounds::new(point(px(20.), px(30.)), size(px(0.), px(300.))),
        Bounds::new(point(px(20.), px(30.)), size(px(280.), px(0.))),
        Bounds::new(point(px(20.), px(30.)), size(px(1.), px(300.))),
        Bounds::new(point(px(20.), px(30.)), size(px(280.), px(1.))),
        Bounds::new(point(px(20.), px(30.)), size(px(20.), px(20.))),
        Bounds::new(point(px(500.), px(600.)), size(px(280.), px(300.))),
    ]
}

fn set_bounds(
    view: &gpui::Entity<BoundedOverlay>,
    cx: &mut VisualTestContext,
    area: Bounds<Pixels>,
) {
    view.update(cx, |view, cx| {
        view.bounds = area;
        cx.notify();
    });
    settle(cx);
}

fn restored(view: &gpui::Entity<BoundedOverlay>, cx: &mut VisualTestContext) {
    let before = view.read_with(cx, |view, _| view.before.clone());
    assert!(
        cx.update(|window, _| before.is_focused(window)),
        "focus returned"
    );
}

#[gpui::test]
fn no_room_closes_popover_and_touch_disclosure_before_building_or_taking_focus(
    cx: &mut TestAppContext,
) {
    let (view, cx) = fixture(cx, false);
    let normal = view.read_with(cx, |view, _| view.bounds);
    let builds = view.read_with(cx, |view, _| view.builds.clone());
    for hover in [false, true] {
        view.update(cx, |view, cx| {
            view.hover = hover;
            cx.notify();
        });
        settle(cx);
        for small in areas() {
            open(cx, hover);
            contained(cx, normal);
            let before = builds.get();
            set_bounds(&view, cx, small);
            assert!(cx.debug_bounds("popover-panel").is_none());
            assert_eq!(builds.get(), before, "collapse did not build content");
            restored(&view, cx);
            open(cx, hover);
            assert!(cx.debug_bounds("popover-panel").is_none());
            assert_eq!(
                builds.get(),
                before,
                "no-room activation did not build content"
            );
            restored(&view, cx);
            set_bounds(&view, cx, normal);
            assert!(
                cx.debug_bounds("popover-panel").is_none(),
                "reopening is explicit"
            );
            open(cx, hover);
            contained(cx, normal);
            assert!(builds.get() > before);
            press("escape", cx);
            restored(&view, cx);
        }
    }
}

#[gpui::test]
fn no_room_hides_desktop_hover_and_cancels_pending_open(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    cx.update(|window, cx| {
        window.set_rem_size(px(32.));
        Theme::update(cx, |theme| theme.touch_mode = false);
    });
    settle(cx);
    let normal = view.read_with(cx, |view, _| view.bounds);
    let builds = view.read_with(cx, |view, _| view.builds.clone());
    for small in areas() {
        let primary = cx.debug_bounds("overlay-primary").unwrap().center();
        cx.simulate_mouse_move(primary, None, Modifiers::none());
        settle(cx);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(600));
        settle(cx);
        contained(cx, normal);
        let before = builds.get();
        set_bounds(&view, cx, small);
        assert!(cx.debug_bounds("hover-card-panel").is_none());
        assert_eq!(builds.get(), before);
        restored(&view, cx);
        cx.simulate_mouse_move(point(px(310.), px(450.)), None, Modifiers::none());
        settle(cx);
        cx.simulate_mouse_move(primary, None, Modifiers::none());
        settle(cx);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(600));
        settle(cx);
        assert!(cx.debug_bounds("hover-card-panel").is_none());
        assert_eq!(builds.get(), before);
        set_bounds(&view, cx, normal);
        cx.simulate_mouse_move(point(px(310.), px(450.)), None, Modifiers::none());
        settle(cx);
        cx.simulate_mouse_move(primary, None, Modifiers::none());
        settle(cx);
        // Collapse before the delayed callback runs, then restore without another entry.
        set_bounds(&view, cx, small);
        set_bounds(&view, cx, normal);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(600));
        settle(cx);
        assert!(cx.debug_bounds("hover-card-panel").is_none());
        assert_eq!(
            builds.get(),
            before,
            "cancelled timer did not rebuild content"
        );
        cx.simulate_mouse_move(point(px(310.), px(450.)), None, Modifiers::none());
        settle(cx);
    }
}
