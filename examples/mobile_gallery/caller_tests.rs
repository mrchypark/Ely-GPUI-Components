use ely_gpui_component::{layout::ViewportInsets, theme::Theme};
use gpui::{Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext, point, px, size};

use crate::MobileGallery;

#[gpui::test]
fn gallery_all_six_wrapper_callers_forward_live_usable_bounds(cx: &mut TestAppContext) {
    use gpui::{ScrollDelta, ScrollWheelEvent, TouchPhase};
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(390.), px(844.)));
    let safe = ViewportInsets {
        left: px(16.),
        right: px(16.),
        top: px(20.),
        bottom: px(12.),
    };
    let area = Bounds::new(point(px(16.), px(20.)), size(px(358.), px(812.)));
    let smaller = Bounds::new(point(px(16.), px(20.)), size(px(358.), px(424.)));
    for (page, story, opener) in [
        ("navigation", "QuickOpen", "gallery-quick-open"),
        ("navigation", "QuickSwitcher", "gallery-quick-switcher"),
        (
            "editor",
            "CodeActionMenu / RenameInput / PeekView / ReferencesPanel / SymbolOutline / GoToSymbol / CallHierarchy / TypeHierarchy",
            "gallery-go-to-symbol",
        ),
        (
            "editor",
            "WelcomePage / RecentProjects / ProjectSwitcher / ThemePreview / TaskRunner / RunConfiguration",
            "gallery-project-switcher",
        ),
        (
            "git",
            "BranchSelector / BranchList / TagList / StashList",
            "gallery-branch-selector",
        ),
        (
            "terminal",
            "CommandBlock / CommandHistory",
            "gallery-command-history",
        ),
    ] {
        view.update(cx, |view, cx| {
            view.set_insets(safe, px(0.), cx);
            assert!(view.select_story(page, story, cx), "{story}");
        });
        draw(cx);
        for _ in 0..4 {
            let content = cx.debug_bounds("gallery-content").unwrap();
            let target = cx.debug_bounds(opener).expect(opener);
            if target.left() >= content.left()
                && target.right() <= content.right()
                && target.top() >= content.top()
                && target.bottom() <= content.bottom()
            {
                break;
            }
            for (phase, delta) in [
                (TouchPhase::Started, point(px(0.), px(0.))),
                (TouchPhase::Moved, content.center() - target.center()),
                (TouchPhase::Ended, point(px(0.), px(0.))),
            ] {
                cx.simulate_event(ScrollWheelEvent {
                    position: content.center(),
                    delta: ScrollDelta::Pixels(delta),
                    modifiers: Modifiers::none(),
                    touch_phase: phase,
                });
            }
            draw(cx);
        }
        let content = cx.debug_bounds("gallery-content").unwrap();
        tap(opener, content, cx);
        contained("palette-card", area, cx);
        view.update(cx, |view, cx| view.set_insets(safe, px(400.), cx));
        draw(cx);
        contained("palette-card", smaller, cx);
        let close = contained("palette-close", smaller, cx);
        assert!(close.size.width >= px(48.) && close.size.height >= px(48.));
        tap("palette-close", smaller, cx);
        assert!(
            cx.debug_bounds("palette-card").is_none(),
            "{story} did not close"
        );
    }
}

fn draw(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
}
fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
}
fn contained(
    selector: &'static str,
    area: Bounds<Pixels>,
    cx: &mut VisualTestContext,
) -> Bounds<Pixels> {
    let bounds = cx.debug_bounds(selector).expect(selector);
    assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
    assert!(
        bounds.left() >= area.left()
            && bounds.right() <= area.right()
            && bounds.top() >= area.top()
            && bounds.bottom() <= area.bottom(),
        "{selector}: {bounds:?} outside {area:?}"
    );
    bounds
}
fn tap(selector: &'static str, area: Bounds<Pixels>, cx: &mut VisualTestContext) {
    let target = contained(selector, area, cx);
    cx.simulate_click(target.center(), Modifiers::none());
    draw(cx);
}

#[gpui::test]
fn mobile_launcher_uses_bounded_inline_search_and_real_result_callback(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    cx.simulate_resize(size(px(390.), px(844.)));
    let safe = ViewportInsets {
        left: px(16.),
        right: px(16.),
        top: px(20.),
        bottom: px(12.),
    };
    let area = Bounds::new(point(px(16.), px(20.)), size(px(358.), px(812.)));
    view.update(cx, |view, cx| {
        assert!(view.select_story(
            "navigation",
            "SearchPalette / SpotlightSearch / QuickLauncher",
            cx
        ));
        view.set_insets(safe, px(0.), cx);
    });
    draw(cx);
    let count = cx.update(|_, cx| cx.windows().len());
    tap("gallery-launcher-open", area, cx);
    contained("palette-card", area, cx);
    assert_eq!(cx.update(|_, cx| cx.windows().len()), count);
    assert!(cx.update(|_, cx| crate::probe::Opened::get("launcher", cx).is_none()));
    tap("palette-row-0", area, cx);
    assert!(cx.debug_bounds("palette-card").is_none());
    assert!(cx.debug_bounds("gallery-search-result-palette").is_some());
    tap("gallery-launcher-open", area, cx);
    view.update(cx, |view, cx| view.set_insets(safe, px(400.), cx));
    draw(cx);
    let smaller = Bounds::new(point(px(16.), px(20.)), size(px(358.), px(424.)));
    contained("palette-card", smaller, cx);
    tap("palette-close", smaller, cx);
    assert!(cx.debug_bounds("palette-card").is_none());
    assert_eq!(cx.update(|_, cx| cx.windows().len()), count);
}

#[gpui::test]
fn mobile_file_grid_select_then_open_stays_inside_phone_content(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for (width, tile, result) in [
        (390., "file-grid-tile-0", "gallery-file-opened-Design"),
        (320., "file-grid-tile-1", "gallery-file-opened-Recordings"),
    ] {
        cx.simulate_resize(size(px(width), px(844.)));
        view.update(cx, |view, cx| {
            assert!(view.select_story("files", "FileGrid", cx));
        });
        draw(cx);
        let content = cx.debug_bounds("gallery-content").unwrap();
        let open = contained("file-grid-open", content, cx);
        assert!(open.size.width >= px(48.) && open.size.height >= px(48.));
        assert!(cx.debug_bounds(result).is_none());
        tap(tile, content, cx);
        tap("file-grid-open", content, cx);
        assert!(cx.debug_bounds(result).is_some());
    }
}
