use crate::{MobileGallery, probe::Probes};
use ely_gpui_component::theme::Theme;
use gpui::{
    Bounds, Pixels, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase, VisualTestContext,
    point, px, size,
};

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
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
fn fits(name: &str, bounds: Bounds<Pixels>, area: Bounds<Pixels>) {
    assert!(
        bounds.size.width > px(0.) && bounds.size.height > px(0.),
        "{name}: {bounds:?}"
    );
    assert!(
        bounds.left() >= area.left() && bounds.right() <= area.right(),
        "{name}: {bounds:?} outside {area:?}"
    );
}
fn measured(
    name: &'static str,
    area: Bounds<Pixels>,
    cx: &mut VisualTestContext,
) -> Bounds<Pixels> {
    let bounds = cx.debug_bounds(name).expect(name);
    fits(name, bounds, area);
    bounds
}

#[gpui::test]
fn phone_document_editor_callers_resize_without_changing_the_fixture(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        for (story, probe) in [
            (
                "RichTextEditor / FixedFormatToolbar / FloatingFormatToolbar / LinkEditor / MentionMenu / EmojiAutocomplete / WordCount / ReadingTime",
                "doc-rich",
            ),
            ("MarkdownEditor", "doc-markdown"),
        ] {
            view.update(cx, |view, cx| {
                assert!(view.select_story("documents", story, cx))
            });
            draw(cx);
            let content = cx.debug_bounds("gallery-content").unwrap();
            let editor = cx.update(|_, cx| Probes::get(probe, cx).expect(probe));
            fits(probe, editor, content);
            assert!(
                editor.size.width > content.size.width - px(4.),
                "editor did not use available width"
            );
        }
    }
}

#[gpui::test]
fn phone_presence_header_avatars_and_document_fit_together(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        view.update(cx, |view, cx| {
            assert!(view.select_story(
                "collab",
                "PresenceAvatars / LiveIndicator / FollowMode / RemoteCursor / RemoteSelection",
                cx
            ))
        });
        draw(cx);
        let content = cx.debug_bounds("gallery-content").unwrap();
        let frame = measured("gallery-presence-frame", content, cx);
        let header = measured("gallery-presence-header", frame, cx);
        let title = measured("gallery-presence-title", header, cx);
        let live = measured("gallery-presence-live", header, cx);
        let peers = measured("gallery-presence-peers", header, cx);
        assert!(
            title.bottom() <= live.top() && title.bottom() <= peers.top(),
            "title did not wrap above controls"
        );
        let target = measured("presence-primary", peers, cx);
        assert!(target.size.width >= px(48.) && target.size.height >= px(48.));
        let document = measured("gallery-presence-document", frame, cx);
        let input = measured("input-root", document, cx);
        measured("input-text", input, cx);
    }
}

#[gpui::test]
fn phone_chat_virtual_rows_day_system_and_markdown_fit_the_conversation(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        view.update(cx, |view, cx| assert!(view.select_story("chat",
            "ChatContainer / MessageList / MessageBubble / MessageAvatar / MessageHeader / MessageFooter / DateSeparator / ScrollToBottomButton", cx)));
        draw(cx);
        let mut seen = [false; 3];
        for delta in [
            0., -2400., 160., 160., 160., 160., 160., 160., 160., 160., 160., 160., 160., 160.,
        ] {
            let content = cx.debug_bounds("gallery-content").unwrap();
            let frame = measured("gallery-chat-frame", content, cx);
            assert!(
                frame.size.width >= content.size.width - px(2.),
                "conversation probe shrank: {frame:?} in {content:?}"
            );
            for (ix, selector) in [
                "gallery-chat-day-3",
                "gallery-chat-row-4",
                "gallery-chat-markdown-6",
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(bounds) = cx.debug_bounds(selector) {
                    if bounds.bottom() > frame.top() && bounds.top() < frame.bottom() {
                        fits(selector, bounds, frame);
                        seen[ix] = true;
                    }
                }
            }
            if seen.iter().all(|seen| *seen) {
                break;
            }
            let left = frame.left().max(content.left());
            let top = frame.top().max(content.top());
            let right = frame.right().min(content.right());
            let bottom = frame.bottom().min(content.bottom());
            assert!(
                right > left && bottom > top,
                "chat has no visible wheel target"
            );
            let position = point((left + right) / 2., (top + bottom) / 2.);
            for (phase, amount) in [
                (TouchPhase::Started, 0.),
                (TouchPhase::Moved, delta),
                (TouchPhase::Ended, 0.),
            ] {
                cx.simulate_event(ScrollWheelEvent {
                    position,
                    delta: ScrollDelta::Pixels(point(px(0.), px(amount))),
                    modifiers: gpui::Modifiers::none(),
                    touch_phase: phase,
                });
            }
            draw(cx);
        }
        assert!(
            seen.iter().all(|seen| *seen),
            "not all day/system/formula-bearing markdown rows visited at {width}: {seen:?}"
        );
    }
}
#[gpui::test]
fn phone_generative_enhance_button_fits_the_actual_prompt_field(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        for reveal in 0..2 {
            view.update(cx, |view, cx| assert!(view.select_story("generative",
            "ImageGenerationPanel / PromptEnhancer / NegativePromptInput / StylePresetPicker / AspectRatioPicker / SeedInput", cx)));
            draw(cx);
            let content = cx.debug_bounds("gallery-content").unwrap();
            // Reselection clears Probes; unchanged Measure bounds do not emit again.
            let prompt = cx.debug_bounds("gallery-gen-prompt");
            let target = cx.debug_bounds("prompt-enhancer-gen-prompt-enhance");
            let probe = cx.update(|_, cx| Probes::get("gen-enhancer", cx));
            eprintln!(
                "Generative width={width} reveal={reveal}: viewport={content:?} prompt={prompt:?} enhance={target:?} cached_probe={probe:?}"
            );
            let prompt = prompt.expect("actual prompt must exist in the current frame");
            fits("gen-enhancer", prompt, content);
            assert!(
                prompt.size.width >= content.size.width - px(2.),
                "prompt probe shrank: {prompt:?} in {content:?}"
            );
            let button = measured("prompt-enhancer-gen-prompt-enhance", prompt, cx);
            assert!(
                button.top() >= content.top() && button.bottom() <= content.bottom(),
                "Enhance is vertically unreachable at {width}: {button:?} in {content:?}"
            );
            assert!(button.size.width >= px(48.) && button.size.height >= px(48.));
        }
    }
}
