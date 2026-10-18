use ely_gpui_component::theme::{ActiveTheme, ControlSize, Theme};
use gpui::{Bounds, Pixels, TestAppContext, VisualTestContext, px, size};

use crate::MobileGallery;

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

fn fits(
    selector: &'static str,
    area: Bounds<Pixels>,
    cx: &mut VisualTestContext,
) -> Bounds<Pixels> {
    let bounds = cx.debug_bounds(selector).expect(selector);
    assert!(
        bounds.size.width > px(0.) && bounds.size.height > px(0.),
        "{selector}: {bounds:?}"
    );
    assert!(
        bounds.left() >= area.left() && bounds.right() <= area.right(),
        "{selector}: {bounds:?} outside horizontal viewport {area:?}"
    );
    bounds
}

#[gpui::test]
fn phone_flex_grid_and_multiline_input_fit_the_content_width(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    let mut previous_grow = None;
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        view.update(cx, |view, cx| {
            assert!(view.select_story("layout", "Flex", cx))
        });
        draw(cx);
        let content = cx.debug_bounds("gallery-content").unwrap();
        let frame = fits("gallery-flex-frame", content, cx);
        let first = fits("gallery-flex-first", frame, cx);
        let grow = fits("gallery-flex-grow", frame, cx);
        let last = fits("gallery-flex-last", frame, cx);
        assert!(first.right() <= grow.left() && grow.right() <= last.left());
        if let Some(previous) = previous_grow {
            assert!(
                grow.size.width > previous,
                "flex_1 did not grow with the viewport"
            );
        }
        previous_grow = Some(grow.size.width);

        view.update(cx, |view, cx| {
            assert!(view.select_story("layout", "Grid / GridItem", cx))
        });
        draw(cx);
        let content = cx.debug_bounds("gallery-content").unwrap();
        let frame = fits("gallery-grid-frame", content, cx);
        let wide = fits("gallery-grid-wide-top", frame, cx);
        let single = fits("gallery-grid-single-top", frame, cx);
        let lower_single = fits("gallery-grid-single-bottom", frame, cx);
        let lower_wide = fits("gallery-grid-wide-bottom", frame, cx);
        assert!(
            wide.size.width > single.size.width && lower_wide.size.width > lower_single.size.width
        );
        assert!(wide.right() <= single.left() && lower_single.right() <= lower_wide.left());
        assert!(wide.top() == single.top() && lower_single.top() == lower_wide.top());
        assert!(
            wide.bottom() <= lower_single.top(),
            "three-column spans lost row placement"
        );

        view.update(cx, |view, cx| {
            assert!(view.select_story("forms", "TextArea / AutoResizeInput", cx))
        });
        draw(cx);
        let content = cx.debug_bounds("gallery-content").unwrap();
        let input = fits("input-root", content, cx);
        let text = fits("input-text", input, cx);
        assert!(
            input.size.width >= content.size.width - px(1.),
            "TextArea must use the available width at {width}: input={input:?}, content={content:?}"
        );
        let inset = cx.update(|window, cx| {
            cx.theme()
                .control_padding(ControlSize::default())
                .to_pixels(window.rem_size())
                + px(1.)
        });
        let expected_text_width = input.size.width - inset * 2.;
        assert!(
            expected_text_width > px(0.) && text.size.width >= expected_text_width - px(1.),
            "TextArea text allocation collapsed at {width}: text={text:?}, expected width={expected_text_width:?}"
        );
        assert!(input.top() >= content.top() && input.bottom() <= content.bottom());
        assert!(text.top() >= input.top() && text.bottom() <= input.bottom());
    }
}

fn visible(
    selector: &'static str,
    area: Bounds<Pixels>,
    cx: &mut VisualTestContext,
) -> Bounds<Pixels> {
    let bounds = fits(selector, area, cx);
    assert!(
        bounds.top() >= area.top() && bounds.bottom() <= area.bottom(),
        "{selector}: {bounds:?} outside vertical viewport {area:?}"
    );
    bounds
}

fn reveal_strip(selector: &'static str, cx: &mut VisualTestContext) {
    use gpui::{ScrollDelta, ScrollWheelEvent, TouchPhase, point};
    for _ in 0..3 {
        let content = cx.debug_bounds("gallery-content").unwrap();
        let strip = cx.debug_bounds(selector).expect(selector);
        if strip.top() >= content.top() && strip.bottom() <= content.bottom() {
            return;
        }
        for (phase, delta) in [
            (TouchPhase::Started, px(0.)),
            (TouchPhase::Moved, content.center().y - strip.center().y),
            (TouchPhase::Ended, px(0.)),
        ] {
            cx.simulate_event(ScrollWheelEvent {
                position: content.center(),
                delta: ScrollDelta::Pixels(point(px(0.), delta)),
                modifiers: gpui::Modifiers::none(),
                touch_phase: phase,
            });
        }
        draw(cx);
    }
}

#[gpui::test]
fn phone_tabs_real_strips_and_last_enabled_targets_fit_and_select(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        view.update(cx, |view, cx| {
            assert!(view.select_story("navigation", "Tabs", cx))
        });
        draw(cx);
        for (frame, body, strip, first, last, initial, selected, items) in [
            (
                "gallery-tabs-top",
                "gallery-tabs-top-body",
                "tabs-tabs-top-strip",
                "tabs-tabs-top-tab-0",
                "tabs-tabs-top-tab-2",
                "gallery-tabs-selected-overview",
                "gallery-tabs-selected-settings",
                &[
                    "tabs-tabs-top-tab-0",
                    "tabs-tabs-top-tab-1",
                    "tabs-tabs-top-tab-2",
                    "tabs-tabs-top-tab-3",
                ][..],
            ),
            (
                "gallery-tabs-left",
                "gallery-tabs-left-body",
                "tabs-tabs-left-strip",
                "tabs-tabs-left-tab-0",
                "tabs-tabs-left-tab-2",
                "gallery-tabs-selected-general",
                "gallery-tabs-selected-keys",
                &[
                    "tabs-tabs-left-tab-0",
                    "tabs-tabs-left-tab-1",
                    "tabs-tabs-left-tab-2",
                ][..],
            ),
            (
                "gallery-tabs-bottom",
                "gallery-tabs-bottom-body",
                "tabs-tabs-bottom-strip",
                "tabs-tabs-bottom-tab-0",
                "tabs-tabs-bottom-tab-2",
                "gallery-tabs-selected-console",
                "gallery-tabs-selected-problems",
                &[
                    "tabs-tabs-bottom-tab-0",
                    "tabs-tabs-bottom-tab-1",
                    "tabs-tabs-bottom-tab-2",
                ][..],
            ),
        ] {
            reveal_strip(strip, cx);
            let content = cx.debug_bounds("gallery-content").unwrap();
            let frame_bounds = fits(frame, content, cx);
            fits(body, frame_bounds, cx);
            visible(strip, content, cx);
            let first = visible(first, content, cx);
            cx.simulate_click(first.center(), gpui::Modifiers::none());
            draw(cx);
            assert!(
                cx.debug_bounds(initial).is_some(),
                "first tab did not select at {width}"
            );
            assert!(cx.debug_bounds(selected).is_none());
            let last = visible(last, content, cx);
            assert!(last.size.width >= px(48.) && last.size.height >= px(48.));
            cx.simulate_click(last.center(), gpui::Modifiers::none());
            draw(cx);
            assert!(
                cx.debug_bounds(selected).is_some(),
                "last enabled tab did not select at {width}"
            );
            assert!(
                cx.debug_bounds(initial).is_none(),
                "old selected panel remained"
            );
            let content = cx.debug_bounds("gallery-content").unwrap();
            let frame_bounds = fits(frame, content, cx);
            fits(body, frame_bounds, cx);
            let strip_bounds = visible(strip, content, cx);
            for item in items {
                let bounds = visible(item, strip_bounds, cx);
                assert!(bounds.size.width >= px(48.) && bounds.size.height >= px(48.));
            }
            if selected == "gallery-tabs-selected-settings" {
                let disabled = visible("tabs-tabs-top-tab-3", content, cx);
                cx.simulate_click(disabled.center(), gpui::Modifiers::none());
                draw(cx);
                assert!(
                    cx.debug_bounds(selected).is_some(),
                    "disabled Archived selected"
                );
            }
        }
    }
}
