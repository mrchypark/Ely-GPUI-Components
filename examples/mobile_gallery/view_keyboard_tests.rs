use super::*;

#[gpui::test]
fn focused_input_remains_visible_on_a_short_landscape_keyboard(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(615.), px(700.)));
    cx.update(|window, _| window.activate_window());
    view.update(cx, |view, cx| {
        assert!(view.select_story("forms", "Input / TextField / ClearableInput", cx));
    });
    for _ in 0..3 {
        draw(cx);
    }
    let input = cx.debug_bounds("gallery-forms-first-input").unwrap();
    cx.simulate_click(input.center(), Modifiers::none());
    cx.simulate_resize(size(px(615.), px(346.)));
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(214.), cx)
    });
    for _ in 0..6 {
        draw_reveal(cx);
    }
    let field = cx.debug_bounds("gallery-forms-first-input").unwrap();
    let viewport = cx.debug_bounds("gallery-content").unwrap();
    assert!(
        field.top() >= viewport.top() - px(1.) && field.bottom() <= viewport.bottom() + px(1.),
        "field {field:?} outside {viewport:?}"
    );
}

#[gpui::test]
fn focused_input_is_revealed_after_a_keyboard_active_display_resize(cx: &mut TestAppContext) {
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
    let input = cx.debug_bounds("gallery-forms-first-input").unwrap();
    cx.simulate_click(input.center(), Modifiers::none());
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(560.), cx)
    });
    for _ in 0..5 {
        draw_reveal(cx);
    }

    cx.simulate_resize(size(px(1200.), px(844.)));
    for _ in 0..6 {
        draw_reveal(cx);
    }

    let field = cx.debug_bounds("gallery-forms-first-input").unwrap();
    let viewport = cx.debug_bounds("gallery-content").unwrap();
    assert!(
        field.top() >= viewport.top() - px(1.) && field.bottom() <= viewport.bottom() + px(1.),
        "field {field:?} outside resized viewport {viewport:?}"
    );
}

#[gpui::test]
fn forms_first_input_is_revealed_when_keyboard_shrinks_the_viewport(cx: &mut TestAppContext) {
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
    let input = cx
        .debug_bounds("gallery-forms-first-input")
        .expect("first input");
    cx.simulate_click(input.center(), Modifiers::none());
    let focus = cx
        .update(|window, cx| window.focused(cx))
        .expect("input focused");
    assert!(
        !cx.update(|window, cx| {
            ely_gpui_component::primitives::focused_in_takeover(window, cx)
        })
    );
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(560.), cx);
    });
    for _ in 0..5 {
        draw_reveal(cx);
    }
    let gallery_focus = view.read_with(cx, |view, _| view.focus.clone());
    assert!(cx.update(|window, cx| gallery_focus.contains_focused(window, cx)));
    assert!(
        cx.update(|window, _| window.focused_text_input_geometry())
            .is_some(),
        "focused input must have painted geometry"
    );
    let field = cx
        .debug_bounds("gallery-forms-first-input")
        .expect("focused input");
    let viewport = cx.debug_bounds("gallery-content").expect("scroll viewport");
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        Some(focus.clone())
    );
    let geometry = cx.update(|window, _| window.focused_text_input_geometry());
    let (offset, max, bounds) = view.read_with(cx, |view, _| {
        (
            view.scroll.offset(),
            view.scroll.max_offset(),
            view.scroll.bounds(),
        )
    });
    assert!(
        offset.y < px(0.),
        "geometry={geometry:?} scroll={bounds:?} offset={offset:?} max={max:?}"
    );
    assert!(
        field.top() >= viewport.top() - px(1.) && field.bottom() <= viewport.bottom() + px(1.),
        "field {field:?} outside {viewport:?}"
    );
    assert!(field.bottom() <= px(284.) + px(1.));
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(0.), cx)
    });
    for _ in 0..3 {
        draw(cx);
    }
    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(focus));
    cx.simulate_input("Ada");
    assert!(cx.debug_bounds("gallery-content").unwrap().size.height > viewport.size.height);
    let field = cx
        .debug_bounds("gallery-forms-first-input")
        .expect("input after dismissal");
    assert!(field.size.height > px(0.));
}

#[gpui::test]
fn floating_keyboard_reveals_the_newly_focused_field(cx: &mut TestAppContext) {
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
    cx.simulate_click(first.center(), Modifiers::none());
    let second = cx.debug_bounds("gallery-forms-small-input").unwrap();
    let keyboard = Bounds::new(
        point(px(0.), second.top() - px(5.)),
        size(px(390.), px(180.)),
    );
    view.update(cx, |view, cx| {
        view.set_floating_keyboard(Some(keyboard), cx)
    });
    for _ in 0..3 {
        draw_reveal(cx);
    }
    cx.update(|window, cx| window.focus_next(cx));
    let focus = cx
        .update(|window, cx| window.focused(cx))
        .expect("second input");
    for _ in 0..6 {
        draw_reveal(cx);
    }
    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(focus));
    let field = cx.debug_bounds("gallery-forms-small-input").unwrap();
    let overlap = field.intersect(&keyboard);
    let geometry = cx.update(|window, _| window.focused_text_input_geometry());
    let focused_element = geometry
        .and_then(|(element, _)| element)
        .expect("focused input bounds");
    assert!(
        focused_element.top() >= field.top() && focused_element.bottom() <= field.bottom(),
        "focused input {focused_element:?} is not the second field {field:?}"
    );
    let (offset, max, bounds) = view.read_with(cx, |view, _| {
        (
            view.scroll.offset(),
            view.scroll.max_offset(),
            view.scroll.bounds(),
        )
    });
    assert!(
        overlap.size.width <= px(0.) || overlap.size.height <= px(0.),
        "field {field:?} overlaps keyboard {keyboard:?}; geometry {geometry:?}; scroll {bounds:?} offset {offset:?} max {max:?}"
    );
}

#[gpui::test]
fn queued_keyboard_reveal_does_not_scroll_after_focus_is_cleared(cx: &mut TestAppContext) {
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
    cx.simulate_click(first.center(), Modifiers::none());
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(560.), cx);
    });
    draw(cx);
    let before = view.read_with(cx, |view, _| view.scroll.offset());
    cx.update(|window, cx| {
        window.blur(cx);
        window.simulate_next_frame(cx);
    });
    draw(cx);
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.scroll.offset()), before);
}

#[gpui::test]
fn queued_keyboard_reveal_does_not_scroll_after_keyboard_hides(cx: &mut TestAppContext) {
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
    cx.simulate_click(first.center(), Modifiers::none());
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(560.), cx);
    });
    draw(cx);
    let before = view.read_with(cx, |view, _| view.scroll.offset());
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(0.), cx);
    });
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    draw(cx);
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.scroll.offset()), before);
}

#[gpui::test]
fn choice_cards_fit_phone_and_tablet_viewports(cx: &mut TestAppContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    view.update(cx, |view, cx| {
        assert!(view.select_story("forms", "CheckboxCard / RadioCard", cx))
    });
    for width in [320., 768.] {
        cx.simulate_resize(size(px(width), px(1024.)));
        draw(cx);
        let content = cx.debug_bounds("gallery-content").unwrap();
        let personal = cx.debug_bounds("gallery-card-personal").unwrap();
        let team = cx.debug_bounds("gallery-card-team").unwrap();
        let backup = cx.debug_bounds("gallery-card-backup").unwrap();
        for card in [personal, team, backup] {
            assert!(
                card.left() >= content.left() && card.right() <= content.right(),
                "{width}: {card:?} outside {content:?}"
            );
        }
        if width == 320. {
            assert!(team.top() >= personal.bottom());
        } else {
            assert_eq!(team.top(), personal.top());
        }
    }
}
