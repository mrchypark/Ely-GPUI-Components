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

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::init(cx);
    });
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
