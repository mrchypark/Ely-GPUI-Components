use ely_gpui_component::{
    buttons::Button,
    layout::{ViewportInsets, usable_bounds},
    theme::ActiveTheme,
};
use gpui::{
    AppContext, Bounds, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Pixels, Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div, point, px,
};

use crate::{pages, probe::probe};

#[derive(serde::Deserialize)]
struct Stories {
    pages: Vec<StoryPage>,
}
#[derive(serde::Deserialize)]
struct StoryPage {
    slug: String,
    stories: Vec<Specimen>,
}
#[derive(serde::Deserialize)]
struct Specimen {
    title: String,
    slug: String,
}

fn stories(page: &str) -> &'static [Specimen] {
    static CATALOG: std::sync::OnceLock<Stories> = std::sync::OnceLock::new();
    CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("../gallery/stories.json"))
                .expect("gallery stories catalog must be valid")
        })
        .pages
        .iter()
        .find(|entry| entry.slug == page)
        .map_or(&[], |entry| entry.stories.as_slice())
}

/// A single mounted catalog page; navigation remains outside its scroll viewport.
pub struct MobileGallery {
    focus: FocusHandle,
    page: usize,
    visits: u64,
    catalog: bool,
    story: Option<usize>,
    story_error: Option<String>,
    scroll: ScrollHandle,
    insets: ViewportInsets,
    keyboard: Pixels,
}

impl MobileGallery {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            focus,
            page: 0,
            visits: 1,
            catalog: true,
            story: None,
            story_error: None,
            scroll: ScrollHandle::new(),
            insets: ViewportInsets::default(),
            keyboard: px(0.),
        }
    }

    /// Feed real platform insets; no simulated keyboard is used in this harness.
    pub fn set_insets(&mut self, insets: ViewportInsets, keyboard: Pixels, cx: &mut Context<Self>) {
        self.insets = insets;
        self.keyboard = keyboard;
        cx.notify();
    }

    /// Stable automation entry, shared with touch navigation. Unknown slugs leave state intact.
    pub fn select(&mut self, slug: &str, cx: &mut Context<Self>) -> bool {
        let Some(page) = pages::find(slug) else {
            return false;
        };
        self.page = page;
        self.catalog = false;
        self.story = None;
        self.story_error = None;
        self.visits += 1;
        cx.set_global(crate::probe::Probes::default());
        self.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
        true
    }

    pub fn current_slug(&self) -> &'static str {
        pages::ALL[self.page].slug
    }
    /// Select a catalog story by slug or exact title without invoking its controls.
    pub fn select_story(&mut self, page: &str, story: &str, cx: &mut Context<Self>) -> bool {
        let Some(index) = stories(page)
            .iter()
            .position(|entry| entry.slug == story || entry.title == story)
        else {
            return false;
        };
        if !self.select(page, cx) {
            return false;
        }
        self.story = Some(index);
        true
    }

    pub fn current_story_slug(&self) -> Option<&'static str> {
        self.story
            .map(|index| stories(self.current_slug())[index].slug.as_str())
    }
    pub fn visits(&self) -> u64 {
        self.visits
    }
}

impl Render for MobileGallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let available = usable_bounds(
            Bounds::new(point(px(0.), px(0.)), window.viewport_size()),
            self.insets,
            self.keyboard,
        );
        let colors = cx.theme().colors.clone();
        let page = &pages::ALL[self.page];
        let specimens = stories(page.slug);
        let mut navigation = div().flex().flex_wrap().gap_2();
        for (id, label, target) in [
            ("gallery-previous", "Previous", self.page.checked_sub(1)),
            (
                "gallery-next",
                "Next",
                (self.page + 1 < pages::ALL.len()).then_some(self.page + 1),
            ),
        ] {
            let entity = cx.entity();
            navigation = navigation.child(probe(
                id,
                div()
                    .min_h(px(48.))
                    .debug_selector(move || id.into())
                    .child(Button::new(id, label).disabled(target.is_none()).on_click(
                        move |_, _, cx| {
                            if let Some(index) = target {
                                entity.update(cx, |view, cx| {
                                    view.select(pages::ALL[index].slug, cx);
                                });
                            }
                        },
                    )),
            ));
        }
        let entity = cx.entity();
        navigation = navigation.child(probe(
            "gallery-catalog",
            div()
                .min_h(px(48.))
                .debug_selector(|| "gallery-catalog".into())
                .child(
                    Button::new("gallery-catalog", "Catalog").on_click(move |_, _, cx| {
                        entity.update(cx, |view, cx| {
                            view.catalog = !view.catalog;
                            cx.notify();
                        })
                    }),
                ),
        ));
        let mut story_navigation = div().flex().flex_wrap().gap_2();
        for (id, label, target) in [
            (
                "gallery-story-previous",
                "‹ Story",
                self.story.and_then(|index| index.checked_sub(1)),
            ),
            (
                "gallery-story-next",
                "Story ›",
                match self.story {
                    Some(index) => (index + 1 < specimens.len()).then_some(index + 1),
                    None => (!specimens.is_empty()).then_some(0),
                },
            ),
        ] {
            let entity = cx.entity();
            story_navigation = story_navigation.child(probe(
                id,
                div()
                    .min_h(px(48.))
                    .debug_selector(move || id.into())
                    .child(Button::new(id, label).disabled(target.is_none()).on_click(
                        move |_, _, cx| {
                            if let Some(index) = target {
                                entity.update(cx, |view, cx| {
                                    view.select_story(page.slug, &specimens[index].slug, cx);
                                });
                            }
                        },
                    )),
            ));
        }
        let entity = cx.entity();
        story_navigation = story_navigation.child(probe(
            "gallery-story-all",
            div()
                .min_h(px(48.))
                .debug_selector(|| "gallery-story-all".into())
                .child(
                    Button::new("gallery-story-all", "All").on_click(move |_, _, cx| {
                        entity.update(cx, |view, cx| {
                            view.select(page.slug, cx);
                        });
                    }),
                ),
        ));
        let body = if self.catalog {
            let mut list = div().flex().flex_col().gap_2();
            for page in pages::ALL {
                let slug = page.slug;
                let entity = cx.entity();
                list = list.child(
                    div()
                        .min_h(px(48.))
                        .debug_selector(move || format!("gallery-page-{slug}"))
                        .child(Button::new(slug, page.title).on_click(move |_, _, cx| {
                            entity.update(cx, |view, cx| {
                                view.select(slug, cx);
                            })
                        })),
                );
            }
            list.into_any_element()
        } else {
            let slug = page.slug;
            // Story is only a render-time filter. Selection and geometry belong to this view.
            if let Some(index) = self.story {
                cx.set_global(crate::ui::Story::new(specimens[index].title.clone().into()));
            }
            let rendered = (page.render)(window, cx, Some(available));
            self.story_error = self
                .story
                .and_then(|_| cx.remove_global::<crate::ui::Story>().missing(slug));
            let rendered = match &self.story_error {
                Some(error) => div()
                    .debug_selector(|| "gallery-story-error".into())
                    .child(error.clone())
                    .into_any_element(),
                None => rendered,
            };
            div()
                .debug_selector(move || format!("gallery-rendered-{slug}"))
                .child(
                    div()
                        .debug_selector(|| "gallery-story-body".into())
                        .child(rendered),
                )
                .into_any_element()
        };
        div()
            .size_full()
            .track_focus(&self.focus)
            .bg(colors.bg)
            .text_color(colors.fg)
            .child(
                div()
                    .absolute()
                    .left(available.origin.x)
                    .top(available.origin.y)
                    .w(available.size.width)
                    .h(available.size.height)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .overflow_hidden()
                    .child(navigation)
                    .children((!self.catalog).then_some(story_navigation))
                    .children((!self.catalog).then(|| {
                        div()
                            .debug_selector(|| "gallery-story-label".into())
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(match self.story {
                                Some(index) => format!(
                                    "{}/{} · {}",
                                    index + 1,
                                    specimens.len(),
                                    specimens[index].title
                                ),
                                None => format!(
                                    "All {} stories · Story › opens the first",
                                    specimens.len()
                                ),
                            })
                    }))
                    .child(
                        div()
                            .debug_selector(|| "gallery-status".into())
                            .child(format!(
                                "{} · {}/{} · visits {}",
                                page.slug,
                                self.page + 1,
                                pages::ALL.len(),
                                self.visits
                            )),
                    )
                    .child(
                        div()
                            .id("gallery-content")
                            .debug_selector(|| "gallery-content".into())
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_y_scroll()
                            .overflow_x_scroll()
                            .track_scroll(&self.scroll)
                            .child(body),
                    ),
            )
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod story_tests {
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
        let catalog: Stories =
            serde_json::from_str(include_str!("../gallery/stories.json")).unwrap();
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
        assert_eq!(addresses.len(), 624, "update the specimen inventory");
    }

    #[gpui::test]
    fn every_catalog_story_draws_with_identity_and_reveals_from_the_top(cx: &mut TestAppContext) {
        let _files = crate::assets::prepare_test_files();
        init(cx);
        let (view, cx) = cx.add_window_view(MobileGallery::new);
        cx.simulate_resize(size(px(390.), px(844.)));
        for page in pages::ALL {
            for story in stories(page.slug) {
                view.update(cx, |view, cx| {
                    view.scroll.set_offset(point(px(-100.), px(-200.)));
                    assert!(view.select_story(page.slug, &story.slug, cx));
                    assert_eq!(view.current_slug(), page.slug);
                    assert_eq!(view.current_story_slug(), Some(story.slug.as_str()));
                    assert_eq!(view.scroll.offset(), point(px(0.), px(0.)));
                });
                // A second frame also checks that Story's match bookkeeping never accumulates.
                for _ in 0..2 {
                    draw(cx);
                    view.update(cx, |view, _| {
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
                        "{}/{} not revealed: {body:?} in {content:?}",
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
}
