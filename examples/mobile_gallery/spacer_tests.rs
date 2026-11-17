use super::*;

#[gpui::test]
fn floating_keyboard_above_first_input_adds_and_removes_top_reach(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    view.update(cx, |view, cx| {
        assert!(view.select_story("forms", "Input / TextField / ClearableInput", cx));
    });
    for _ in 0..3 {
        draw(cx);
    }
    let first = cx.debug_bounds("gallery-forms-first-input").unwrap();
    let viewport = cx.debug_bounds("gallery-content").unwrap();
    cx.simulate_click(first.center(), Modifiers::none());
    let focus = cx.update(|window, cx| window.focused(cx));
    let keyboard = Bounds::new(
        point(viewport.left(), viewport.top()),
        size(
            viewport.size.width,
            first.bottom() - viewport.top() + px(20.),
        ),
    );
    view.update(cx, |view, cx| {
        view.set_floating_keyboard(Some(keyboard), cx)
    });
    for _ in 0..8 {
        draw_reveal(cx);
    }
    let moved = cx.debug_bounds("gallery-forms-first-input").unwrap();
    assert!(
        moved.top() >= keyboard.bottom(),
        "{moved:?} overlaps {keyboard:?}"
    );
    let reach = view.read_with(cx, |view, _| view.top_reach);
    assert!(reach > px(0.));
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
    view.update(cx, |view, cx| {
        let offset = view.scroll.offset();
        view.scroll
            .set_offset(point(offset.x, offset.y - reach.min(px(30.))));
        cx.notify();
    });
    draw(cx);
    let before_hide = cx.debug_bounds("gallery-forms-first-input").unwrap();
    view.update(cx, |view, cx| view.set_floating_keyboard(None, cx));
    for _ in 0..4 {
        draw_reveal(cx);
    }
    let restored = cx.debug_bounds("gallery-forms-first-input").unwrap();
    let adjustment = before_hide.top() - restored.top();
    assert!(
        adjustment >= px(0.) && adjustment <= reach + px(1.),
        "adjustment {adjustment:?} exceeds reach {reach:?}"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.scroll.max_offset().y),
        px(0.)
    );
    assert_eq!(view.read_with(cx, |view, _| view.top_reach), px(0.));
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
}

#[gpui::test]
fn moving_floating_keyboard_away_reclaims_reach_without_moving_content(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    view.update(cx, |view, cx| {
        assert!(view.select_story("forms", "Input / TextField / ClearableInput", cx));
    });
    for _ in 0..3 {
        draw(cx);
    }
    let first = cx.debug_bounds("gallery-forms-first-input").unwrap();
    let viewport = cx.debug_bounds("gallery-content").unwrap();
    cx.simulate_click(first.center(), Modifiers::none());
    let focus = cx.update(|window, cx| window.focused(cx));
    let keyboard = Bounds::new(
        point(viewport.left(), viewport.top()),
        size(
            viewport.size.width,
            first.bottom() - viewport.top() + px(20.),
        ),
    );
    view.update(cx, |view, cx| {
        view.set_floating_keyboard(Some(keyboard), cx)
    });
    for _ in 0..8 {
        draw_reveal(cx);
    }
    let reach = view.read_with(cx, |view, _| view.top_reach);
    assert!(reach > px(0.));
    view.update(cx, |view, cx| {
        let offset = view.scroll.offset();
        view.scroll
            .set_offset(point(offset.x, offset.y - reach.min(px(30.))));
        cx.notify();
    });
    draw(cx);
    let before = cx.debug_bounds("gallery-forms-first-input").unwrap();
    let away = Bounds::new(
        point(viewport.right() - px(20.), viewport.top()),
        size(px(10.), keyboard.size.height),
    );
    view.update(cx, |view, cx| view.set_floating_keyboard(Some(away), cx));
    for _ in 0..5 {
        draw_reveal(cx);
    }
    let after = cx.debug_bounds("gallery-forms-first-input").unwrap();
    assert!(view.read_with(cx, |view, _| view.scroll.max_offset().y) > px(0.));
    assert!((after.top() - before.top()).abs() <= px(1.));
    let remaining = view.read_with(cx, |view, _| view.top_reach);
    assert!(remaining > px(0.) && remaining < reach);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
    cx.simulate_event(gpui::ScrollEvent {
        position: viewport.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), -remaining - px(5.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
        source: Default::default(),
    });
    for _ in 0..5 {
        draw_reveal(cx);
    }
    assert_eq!(view.read_with(cx, |view, _| view.top_reach), px(0.));
}
