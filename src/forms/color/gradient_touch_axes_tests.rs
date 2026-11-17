use super::*;

#[gpui::test]
fn gradient_add_bar_uses_the_full_touch_height(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let bar = cx.debug_bounds("gradient-touch-bar").expect("gradient bar");
    assert_eq!(bar.size.height, px(48.), "gradient bar: {bar:?}");
    for (x, y) in [(0.3, bar.top() + px(4.)), (0.7, bar.bottom() - px(4.))] {
        cx.simulate_click(point(bar.left() + bar.size.width * x, y), Modifiers::none());
        cx.run_until_parked();
    }
    assert_eq!(view.read_with(cx, |view, _| view.stops.len()), 4);
}

#[gpui::test]
fn gradient_touch_axes_share_scroll_and_cancel_restores(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let wheels = Rc::new(Cell::new(0));
    let scroll_handle = gpui::ScrollHandle::new();
    let (probe_wheels, probe_scroll) = (wheels.clone(), scroll_handle.clone());
    let (view, cx) = cx.add_window_view(move |_, _| GradientScrollProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
        wheels: probe_wheels.clone(),
        scroll: probe_scroll.clone(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let start = point(rail.left(), rail.center().y);
    let initial = scroll_handle.offset();

    scroll(cx, TouchPhase::Started, start, Pixels::ZERO, px(-30.));
    scroll(cx, TouchPhase::Ended, start, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops[0].at), 0.);
    assert!(wheels.get() > 0, "the vertical swipe reached its ancestor");
    assert!(scroll_handle.offset().y < initial.y);
    let wheels_after_vertical = wheels.get();

    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("scrolled gradient rail");
    let start = point(rail.left(), rail.center().y);
    let horizontal = point(start.x + rail.size.width * 0.2, start.y);
    scroll(
        cx,
        TouchPhase::Started,
        horizontal,
        rail.size.width * 0.2,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Moved,
        horizontal,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Ended,
        horizontal,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    let moved = view.read_with(cx, |view, _| view.stops.clone());
    assert!(
        (moved[0].at - 0.2).abs() < 0.01,
        "horizontal drag: {moved:?}"
    );
    assert_eq!(
        wheels.get(),
        wheels_after_vertical,
        "horizontal stays local"
    );

    scroll(
        cx,
        TouchPhase::Moved,
        start,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Ended, start, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert_eq!(
        wheels.get(),
        wheels_after_vertical,
        "horizontal momentum stays local"
    );
    assert_eq!(view.read_with(cx, |view, _| view.stops.clone()), moved);

    let start = point(rail.left() + rail.size.width * 0.2, rail.center().y);
    let cancelled = point(start.x + rail.size.width * 0.1, start.y);
    scroll(
        cx,
        TouchPhase::Started,
        cancelled,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Cancelled,
        cancelled,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops.clone()), moved);
    assert_eq!(wheels.get(), wheels_after_vertical, "cancel stays local");

    let wheels_before_vertical = wheels.get();
    scroll(cx, TouchPhase::Started, start, Pixels::ZERO, px(-30.));
    scroll(cx, TouchPhase::Ended, start, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert!(wheels.get() > wheels_before_vertical);
    assert_eq!(view.read_with(cx, |view, _| view.stops.clone()), moved);
}

#[gpui::test]
fn caught_fling_zero_start_activates_the_new_horizontal_drag(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let wheels = Rc::new(Cell::new(0));
    let scroll_handle = gpui::ScrollHandle::new();
    let (probe_wheels, probe_scroll) = (wheels.clone(), scroll_handle.clone());
    let (view, cx) = cx.add_window_view(move |_, _| GradientScrollProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
        wheels: probe_wheels.clone(),
        scroll: probe_scroll.clone(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let first = point(rail.left(), rail.center().y);
    let first_end = point(first.x + rail.size.width * 0.2, first.y);
    scroll(
        cx,
        TouchPhase::Started,
        first_end,
        rail.size.width * 0.2,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Ended, first_end, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert!(
        (view.read_with(cx, |view, _| view.stops[0].at) - 0.2).abs() < 0.01,
        "the preceding fling must move the first stop"
    );
    let wheels_before = wheels.get();

    let caught = point(rail.left() + rail.size.width * 0.2, rail.center().y);
    scroll(cx, TouchPhase::Ended, caught, Pixels::ZERO, Pixels::ZERO);
    scroll(cx, TouchPhase::Started, caught, Pixels::ZERO, Pixels::ZERO);
    let caught_first = point(caught.x + rail.size.width * 0.1, caught.y);
    scroll(
        cx,
        TouchPhase::Started,
        caught_first,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    let caught_second = point(caught.x + rail.size.width * 0.2, caught.y);
    scroll(
        cx,
        TouchPhase::Moved,
        caught_second,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Ended,
        caught_second,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert!(
        (stops[0].at - 0.4).abs() < 0.01,
        "caught fling drag: {stops:?}"
    );
    assert_eq!(wheels.get(), wheels_before, "horizontal catch stays local");
}

#[gpui::test]
fn pending_vertical_move_does_not_focus_or_select_a_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let wheels = Rc::new(Cell::new(0));
    let (view, cx) = cx.add_window_view(|_, _| GradientScrollProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 0.5,
                color: rgb(0x00ff00).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
        wheels: wheels.clone(),
        scroll: gpui::ScrollHandle::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let middle = point(rail.left() + rail.size.width * 0.5, rail.center().y);
    scroll(cx, TouchPhase::Started, middle, Pixels::ZERO, Pixels::ZERO);
    scroll(cx, TouchPhase::Moved, middle, Pixels::ZERO, px(-30.));
    scroll(cx, TouchPhase::Ended, middle, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert_eq!(wheels.get(), 1, "the vertical pan reaches its ancestor");
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops[1].at), 0.5);
}

#[gpui::test]
fn pending_cancel_does_not_focus_or_select_a_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 0.5,
                color: rgb(0x00ff00).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let middle = point(rail.left() + rail.size.width * 0.5, rail.center().y);
    scroll(cx, TouchPhase::Started, middle, Pixels::ZERO, Pixels::ZERO);
    scroll(
        cx,
        TouchPhase::Cancelled,
        middle,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops[1].at), 0.5);
}

#[gpui::test]
fn tapping_a_stop_does_not_click_its_ancestor(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| GradientAncestorProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 0.5,
                color: rgb(0x00ff00).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
        taps: 0,
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    cx.simulate_click(
        point(rail.left() + rail.size.width * 0.5, rail.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.taps), 0);
    assert_eq!(view.read_with(cx, |view, _| view.stops.len()), 3);
}

#[gpui::test]
fn touch_remove_button_deletes_the_selected_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let stop = |at, color| GradientStop {
        at,
        color: rgb(color).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![
            stop(0.0, 0xff0000),
            stop(0.5, 0x00ff00),
            stop(1.0, 0x0000ff),
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    cx.simulate_click(
        point(rail.left() + rail.size.width * 0.5, rail.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    let remove = cx
        .debug_bounds("gradient-touch-remove")
        .expect("touch remove button");
    assert_eq!(remove.size, gpui::size(px(48.), px(48.)));
    cx.simulate_click(remove.center(), Modifiers::none());
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert_eq!(places(&stops), [0.0, 1.0]);
    assert_eq!(place(&stops, 0xff0000), 0.0);
    assert_eq!(place(&stops, 0x0000ff), 1.0);
    assert!(
        cx.debug_bounds("gradient-touch-remove").is_none(),
        "the two-stop minimum has no removable stop"
    );
}
