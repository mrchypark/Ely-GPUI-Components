#[path = "spacer_tests.rs"]
mod spacer_tests;
#[path = "view_keyboard_tests.rs"]
mod view_keyboard_tests;

use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, size};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        assert!(cx.try_global::<crate::ui::Story>().is_none());
    });
}

fn draw_reveal(cx: &mut VisualTestContext) {
    draw(cx);
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    draw(cx);
    cx.run_until_parked();
}

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::init(cx);
    });
}

#[cfg(any(target_os = "ios", target_os = "android"))]
#[test]
fn system_chrome_contrasts_with_the_gallery_background() {
    assert_eq!(
        system_chrome(false).status_bar_style,
        StatusBarContentStyle::Dark
    );
    assert_eq!(
        system_chrome(true).status_bar_style,
        StatusBarContentStyle::Light
    );
    assert_eq!(
        bar_content_style(false),
        StatusBarContentStyle::Dark,
        "light three-button navigation needs dark buttons"
    );
    assert_eq!(
        bar_content_style(true),
        StatusBarContentStyle::Light,
        "dark three-button navigation needs light buttons"
    );
}

#[gpui::test]
fn focused_palette_keyboard_does_not_scroll_gallery(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    view.update(cx, |view, cx| {
        assert!(view.select_story("navigation", "QuickOpen", cx));
    });
    for _ in 0..3 {
        draw(cx);
    }
    let opener = cx
        .debug_bounds("gallery-quick-open")
        .expect("palette opener");
    cx.simulate_click(opener.center(), Modifiers::none());
    for _ in 0..3 {
        draw(cx);
    }
    assert!(cx.debug_bounds("palette-card").is_some());
    assert!(
        cx.update(|window, cx| { ely_gpui_component::primitives::focused_in_takeover(window, cx) })
    );
    assert!(
        cx.update(|window, _| window.focused_text_input_geometry())
            .is_some()
    );
    let before = view.read_with(cx, |view, _| view.scroll.offset());
    let stale_seen = std::rc::Rc::new(std::cell::Cell::new(false));
    cx.update(|window, cx| {
        let focused = window.focused(cx).unwrap();
        let other = cx.focus_handle();
        let stale_seen = stale_seen.clone();
        window.after_next_paint(move |window, cx| {
            window.focus(&other, cx);
            window.focus(&focused, cx);
            stale_seen.set(window.focused_text_input_geometry().is_none());
        });
    });
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(250.), cx);
    });
    for _ in 0..5 {
        draw_reveal(cx);
    }
    assert!(cx.debug_bounds("palette-card").is_some());
    assert!(
        cx.update(|window, cx| { ely_gpui_component::primitives::focused_in_takeover(window, cx) })
    );
    assert!(
        stale_seen.get(),
        "focus changes must invalidate painted geometry"
    );
    assert_eq!(view.read_with(cx, |view, _| view.scroll.offset()), before);

    // Leave a spacer exposed while focused, then reclaim part once the floating keyboard clears.
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(0.), cx);
        view.set_floating_keyboard(
            Some(Bounds::new(point(px(500.), px(0.)), size(px(10.), px(10.)))),
            cx,
        );
        view.top_reach = px(1000.);
        view.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    });
    for _ in 0..3 {
        draw_reveal(cx);
    }
    view.update(cx, |view, cx| view.set_floating_keyboard(None, cx));
    for _ in 0..3 {
        draw_reveal(cx);
    }
    let remaining = view.read_with(cx, |view, _| view.top_reach);
    assert!(remaining > px(40.));
    view.update(cx, |view, cx| {
        view.scroll.set_offset(point(px(0.), px(-40.)));
        cx.notify();
    });
    for _ in 0..5 {
        draw_reveal(cx);
    }
    let (after_reach, after_offset) =
        view.read_with(cx, |view, _| (view.top_reach, view.scroll.offset().y));
    assert!(after_reach < remaining);
    assert!(((after_reach + after_offset) - (remaining - px(40.))).abs() <= px(1.));
    assert!(
        cx.update(|window, cx| { ely_gpui_component::primitives::focused_in_takeover(window, cx) })
    );
}

#[gpui::test]
fn stale_reveal_retries_stop_until_insets_change(cx: &mut TestAppContext) {
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    view.update(cx, |view, cx| {
        view.set_insets(ViewportInsets::default(), px(200.), cx);
        assert!(view.retry_keyboard_reveal(cx));
        assert!(!view.retry_keyboard_reveal(cx));
        assert!(!view.retry_keyboard_reveal(cx));
        view.set_insets(ViewportInsets::default(), px(200.), cx);
        assert!(!view.retry_keyboard_reveal(cx));
        view.set_insets(ViewportInsets::default(), px(201.), cx);
        assert!(view.retry_keyboard_reveal(cx));
        assert!(!view.retry_keyboard_reveal(cx));
    });
}

#[test]
fn floating_keyboard_falls_back_to_caret_for_a_tall_input() {
    let rect = |x, y, width, height| Bounds::new(point(px(x), px(y)), size(px(width), px(height)));
    let frame = rect(0., 0., 400., 600.);
    let keyboard = Some(rect(0., 200., 400., 200.));
    let element = Some(rect(20., 100., 300., 350.));
    let caret = Some(rect(40., 150., 0., 20.));
    let (target, visible) =
        keyboard_target(element, caret, frame, keyboard, px(-300.), px(600.)).unwrap();
    assert_eq!(target, rect(40., 134., 1., 52.));
    assert!(target.top() >= visible.top() && target.bottom() <= visible.bottom());
    assert!(keyboard_target(element, None, frame, keyboard, px(-300.), px(600.)).is_none());
}

#[test]
fn floating_keyboard_prefers_the_whole_input_when_it_fits() {
    let rect = |x, y, width, height| Bounds::new(point(px(x), px(y)), size(px(width), px(height)));
    let frame = rect(0., 0., 400., 600.);
    let keyboard = Some(rect(200., 200., 200., 200.));
    let element = Some(rect(20., 250., 120., 30.));
    let caret = Some(rect(30., 255., 0., 20.));
    let (target, visible) =
        keyboard_target(element, caret, frame, keyboard, px(-300.), px(600.)).unwrap();
    assert_eq!(target, rect(20., 234., 120., 62.));
    assert_eq!(visible, frame, "the input is beside the keyboard");
}

#[test]
fn floating_keyboard_uses_reachable_side_when_nearest_side_cannot_scroll() {
    let rect = |x, y, width, height| Bounds::new(point(px(x), px(y)), size(px(width), px(height)));
    let frame = rect(0., 0., 400., 800.);
    let keyboard = Some(rect(0., 300., 400., 200.));
    let element = Some(rect(20., 350., 200., 50.));
    let (field, visible) =
        keyboard_target(element, None, frame, keyboard, px(-200.), px(210.)).unwrap();
    assert_eq!(visible.top(), px(500.));
    assert_eq!(
        ely_gpui_component::layout::reveal_offset(field, visible).y,
        px(166.)
    );
}

#[test]
fn top_reach_is_only_the_missing_downward_scroll() {
    let rect = |x, y, width, height| Bounds::new(point(px(x), px(y)), size(px(width), px(height)));
    let field = rect(20., 84., 200., 82.);
    let below = rect(0., 300., 400., 500.);
    let keyboard = Some(rect(0., 0., 400., 300.));
    assert_eq!(needed_top_reach(field, below, keyboard, px(0.)), px(216.));
    assert_eq!(
        needed_top_reach(field, below, keyboard, px(-100.)),
        px(116.)
    );
    assert_eq!(needed_top_reach(field, below, None, px(0.)), px(0.));
}

#[test]
fn platform_ime_edge_reveals_without_a_docked_keyboard_rectangle() {
    let viewport = Bounds::new(point(px(0.), px(0.)), size(px(390.), px(844.)));
    let keyboard = keyboard_occlusion(px(0.), px(300.), false);
    let visible = usable_bounds(viewport, ViewportInsets::default(), keyboard);
    let field = Bounds::new(point(px(20.), px(560.)), size(px(300.), px(48.)));

    assert_eq!(keyboard, px(300.));
    assert!(visible.bottom() < field.bottom());
    assert!(
        ely_gpui_component::layout::reveal_offset(field, visible).y < px(0.),
        "the IME edge must drive the same reveal path as a reported keyboard rectangle"
    );
    assert_eq!(keyboard_occlusion(px(0.), px(300.), true), px(0.));
}

#[test]
fn story_catalog_addresses_are_unique_and_cover_every_page() {
    let catalog: Stories = serde_json::from_str(include_str!("../gallery/stories.json")).unwrap();
    assert_eq!(catalog.pages.len(), pages::ALL.len());
    let mut addresses = std::collections::HashSet::new();
    for page in pages::ALL {
        assert_eq!(
            catalog
                .pages
                .iter()
                .filter(|entry| entry.slug == page.slug)
                .count(),
            1
        );
        assert!(!stories(page.slug).is_empty(), "{}", page.slug);
        for story in stories(page.slug) {
            assert_eq!(story.slug, crate::ui::slug(&story.title));
            assert!(addresses.insert((page.slug, story.slug.as_str())));
        }
    }
    assert_eq!(addresses.len(), 626, "update the specimen inventory");
}

#[gpui::test]
fn every_catalog_story_draws_with_identity_and_reveals_from_the_top(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for page in pages::ALL {
        for story in stories(page.slug) {
            cx.simulate_resize(size(px(320.), px(640.)));
            view.update(cx, |view, cx| {
                view.scroll.set_offset(point(px(-100.), px(-200.)));
                assert!(view.select_story(page.slug, &story.slug, cx));
                assert_eq!(view.current_slug(), page.slug);
                assert_eq!(view.current_story_slug(), Some(story.slug.as_str()));
                assert_eq!(view.scroll.offset(), point(px(0.), px(0.)));
            });
            // Resize the mounted story without selecting it again.
            for (width, height) in [(320., 640.), (640., 280.), (768., 1024.), (1024., 768.)] {
                cx.simulate_resize(size(px(width), px(height)));
                draw(cx);
                view.update(cx, |view, _| {
                    assert_eq!(view.current_slug(), page.slug);
                    assert_eq!(view.current_story_slug(), Some(story.slug.as_str()));
                    assert!(
                        view.story_error.is_none(),
                        "{}/{}: {:?}",
                        page.slug,
                        story.slug,
                        view.story_error
                    );
                });
                let content = cx.debug_bounds("gallery-content").unwrap();
                let body = cx.debug_bounds("gallery-story-body").unwrap();
                assert!(body.size.width > px(0.) && body.size.height > px(0.));
                assert!(
                    content.contains(&(body.origin + point(px(1.), px(1.)))),
                    "{}/{} not revealed at {width}×{height}: {body:?} in {content:?}",
                    page.slug,
                    story.slug
                );
                assert!(cx.debug_bounds("gallery-story-error").is_none());
            }
        }
    }
}

#[gpui::test]
fn long_story_title_wraps_on_phone_and_scrolls_on_short_viewport(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    view.update(cx, |view, cx| {
        assert!(view.select_story(
            "forms",
            "MonthPicker / YearPicker / QuarterPicker / WeekPicker",
            cx,
        ));
    });

    cx.simulate_resize(size(px(1200.), px(844.)));
    draw(cx);
    let single_line = cx.debug_bounds("gallery-story-label").unwrap();

    cx.simulate_resize(size(px(320.), px(844.)));
    draw(cx);
    let wrapped = cx.debug_bounds("gallery-story-label").unwrap();
    let content = cx.debug_bounds("gallery-content").unwrap();
    assert!(
        wrapped.size.height > single_line.size.height,
        "{wrapped:?} vs {single_line:?}"
    );
    assert!(wrapped.left() >= px(0.) && wrapped.right() <= px(320.));
    assert!(content.top() >= wrapped.bottom() && content.size.height > px(0.));

    cx.simulate_resize(size(px(320.), px(280.)));
    draw(cx);
    let before = cx.debug_bounds("gallery-story-label").unwrap();
    let viewport = cx.debug_bounds("gallery-content").unwrap();
    assert!(viewport.size.height > px(0.));
    assert!(before.top() >= viewport.top(), "{before:?} vs {viewport:?}");
    view.update(cx, |view, cx| {
        view.scroll.set_offset(point(px(0.), px(-60.)));
        cx.notify();
    });
    draw(cx);
    let after = cx.debug_bounds("gallery-story-label").unwrap();
    assert!(after.top() < before.top(), "{before:?} -> {after:?}");
}

#[gpui::test]
fn longest_story_title_keeps_body_visible_in_short_landscape(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    let story = stories("editor")
        .iter()
        .max_by_key(|story| story.title.len())
        .unwrap();
    view.update(cx, |view, cx| {
        assert!(view.select_story("editor", &story.slug, cx))
    });
    cx.simulate_resize(size(px(640.), px(280.)));
    draw(cx);
    let label = cx.debug_bounds("gallery-story-label").unwrap();
    let content = cx.debug_bounds("gallery-content").unwrap();
    let body = cx.debug_bounds("gallery-story-body").unwrap();
    assert!(label.size.height <= px(48.), "{label:?}");
    assert!(
        content.contains(&(body.origin + point(px(1.), px(1.)))),
        "{body:?} outside {content:?}"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.current_story_slug()),
        Some(story.slug.as_str())
    );
    let text_before = cx.debug_bounds("gallery-story-label-text").unwrap();
    assert!(
        text_before.size.height > label.size.height,
        "title should overflow: {text_before:?}, {label:?}"
    );
    cx.simulate_mouse_move(label.center(), None, Modifiers::none());
    cx.simulate_event(gpui::ScrollEvent {
        position: label.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-300.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
        source: gpui::ScrollSource::Pointer,
    });
    draw(cx);
    let text_after = cx.debug_bounds("gallery-story-label-text").unwrap();
    assert!(
        text_after.top() < text_before.top(),
        "title must scroll: {text_before:?} -> {text_after:?}"
    );
    assert!(
        text_after.bottom() <= label.bottom() + px(1.),
        "title end must be reachable: {text_after:?} vs {label:?}"
    );
}

#[gpui::test]
fn story_buttons_preserve_page_catalog_and_reject_unknown_addresses(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(390.), px(844.)));
    view.update(cx, |view, cx| {
        assert!(view.select("primitives", cx));
    });
    let tap = |selector: &'static str, cx: &mut VisualTestContext| {
        draw(cx);
        let position = cx.debug_bounds(selector).unwrap().center();
        cx.simulate_click(position, Modifiers::none());
        draw(cx);
    };
    tap("gallery-story-next", cx);
    view.update(cx, |view, _| {
        assert_eq!(view.story, Some(0));
    });
    tap("gallery-story-previous", cx);
    view.update(cx, |view, _| {
        assert_eq!(view.story, Some(0));
    });
    tap("gallery-story-next", cx);
    view.update(cx, |view, _| {
        assert_eq!(view.story, Some(1));
    });
    tap("gallery-story-previous", cx);
    tap("gallery-catalog", cx);
    view.update(cx, |view, _| {
        assert!(view.catalog);
        assert_eq!(view.story, Some(0));
    });
    tap("gallery-catalog", cx);
    view.update(cx, |view, cx| {
        assert!(!view.catalog);
        let before = (view.page, view.story, view.visits);
        assert!(!view.select_story("primitives", "not-a-story", cx));
        assert!(!view.select_story("not-a-page", "text", cx));
        assert_eq!((view.page, view.story, view.visits), before);
        let last = stories("primitives").last().unwrap();
        assert!(view.select_story("primitives", &last.slug, cx));
    });
    tap("gallery-story-next", cx);
    view.update(cx, |view, _| {
        assert_eq!(view.story, Some(stories("primitives").len() - 1));
    });
    tap("gallery-story-all", cx);
    view.update(cx, |view, _| {
        assert!(view.story.is_none());
    });
    tap("gallery-next", cx);
    view.update(cx, |view, _| {
        assert_eq!(view.current_slug(), "typography");
        assert!(view.story.is_none());
    });
}

