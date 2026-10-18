//! Shared gallery crate root for a task-local mobile integration manifest.
//! Register GalleryAssets and prepare its app-private files before rendering.
macro_rules! asset {
    ($name:literal) => {
        concat!("gallery-assets/", $name)
    };
}

pub mod assets;
#[path = "../gallery/menu_model.rs"]
mod menu_model;
#[path = "../gallery/pages/mod.rs"]
pub mod pages;
#[path = "../gallery/probe.rs"]
pub mod probe;
#[path = "../gallery/step.rs"]
pub mod step;
#[path = "../gallery/ui.rs"]
pub mod ui;
mod view {
    include!("view.rs");

    #[cfg(all(test, not(target_family = "wasm")))]
    mod scroll_tests {
        use super::MobileGallery;
        use gpui::{
            Bounds, Modifiers, Pixels, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
            VisualTestContext, point, px, size,
        };

        fn draw(cx: &mut VisualTestContext) {
            cx.run_until_parked();
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
        }

        fn header_bounds(cx: &mut VisualTestContext) -> [Bounds<Pixels>; 4] {
            [
                "gallery-previous",
                "gallery-next",
                "gallery-catalog",
                "gallery-status",
            ]
            .map(|selector| cx.debug_bounds(selector).unwrap())
        }

        // Exercises GPUI wheel dispatch and layout, not UIKit/Android touch synthesis.
        #[gpui::test]
        fn wheel_scroll_moves_catalog_and_specimen_but_not_header(cx: &mut TestAppContext) {
            let _files = crate::assets::prepare_test_files();
            cx.update(|cx| {
                ely_gpui_component::init_for_tests(cx);
                crate::init(cx);
            });
            let (view, cx) = cx.add_window_view(MobileGallery::new);
            cx.simulate_resize(size(px(390.), px(844.)));
            let scroll = view.read_with(cx, |view, _| view.scroll.clone());

            for (page, marker) in [
                (None, "gallery-page-navigation"),
                (Some("navigation"), "gallery-rendered-navigation"),
            ] {
                if let Some(page) = page {
                    view.update(cx, |view, cx| assert!(view.select(page, cx)));
                    assert_eq!(scroll.offset(), point(px(0.), px(0.)));
                }
                draw(cx);
                let viewport = cx.debug_bounds("gallery-content").unwrap();
                assert_eq!(scroll.bounds(), viewport);
                assert!(viewport.size.width > px(0.) && viewport.size.height > px(0.));
                assert!(viewport.left() >= px(0.) && viewport.right() <= px(390.));
                assert!(viewport.top() >= px(0.) && viewport.bottom() <= px(844.));
                assert!(
                    scroll.max_offset().y > px(0.),
                    "{marker} has no scroll range"
                );
                assert_eq!(scroll.offset(), point(px(0.), px(0.)));
                let header = header_bounds(cx);
                assert!(
                    header
                        .iter()
                        .all(|bounds| bounds.bottom() <= viewport.top())
                );
                let before = cx.debug_bounds(marker).unwrap();
                let distance = (scroll.max_offset().y / 2.).min(px(120.));
                let position = viewport.origin + point(px(4.), px(4.));
                cx.simulate_mouse_move(position, None, Modifiers::none());
                for (phase, delta) in [
                    (TouchPhase::Started, -distance / 2.),
                    (TouchPhase::Moved, -distance / 2.),
                    (TouchPhase::Ended, px(0.)),
                ] {
                    cx.simulate_event(ScrollWheelEvent {
                        position,
                        delta: ScrollDelta::Pixels(point(px(0.), delta)),
                        modifiers: Modifiers::none(),
                        touch_phase: phase,
                    });
                }
                draw(cx);
                let offset = scroll.offset();
                assert!(offset.y < px(0.), "{marker} ignored wheel events");
                assert!((offset.y + distance).abs() <= px(0.5));
                assert_eq!(offset.x, px(0.));
                let after = cx.debug_bounds(marker).unwrap();
                assert!(after.top() < before.top(), "{marker} did not move");
                assert!((after.top() - before.top() - offset.y).abs() <= px(0.5));
                assert_eq!(cx.debug_bounds("gallery-content").unwrap(), viewport);
                assert_eq!(header_bounds(cx), header);
                draw(cx);
                assert_eq!(scroll.offset(), offset, "redraw lost the scroll position");
                assert_eq!(cx.debug_bounds(marker).unwrap(), after);
            }

            view.update(cx, |view, cx| assert!(view.select("primitives", cx)));
            assert_eq!(scroll.offset(), point(px(0.), px(0.)));
            draw(cx);
            assert_eq!(scroll.offset(), point(px(0.), px(0.)));
            assert!(cx.debug_bounds("gallery-rendered-primitives").is_some());
        }
    }
}
pub use view::MobileGallery;

/// Ely draws this menu even when the platform has no system application menu.
pub fn app_menus() -> Vec<gpui::Menu> {
    vec![menu_model::view_menu()]
}

/// Call after Ely initialization, before creating the gallery entity.
pub fn init(cx: &mut gpui::App) {
    use ely_gpui_component::theme::{Mode, Theme};
    cx.on_action(|_: &menu_model::LightMode, cx| Theme::set_mode_now(Mode::Light, cx));
    cx.on_action(|_: &menu_model::DarkMode, cx| Theme::set_mode_now(Mode::Dark, cx));
    ely_gpui_component::theme::Theme::update(cx, |theme| theme.touch_mode = true);
    pages::bind_keys(cx);
    pages::install_i18n(cx);
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    #[gpui::test]
    fn navigation_reaches_every_page_and_rejects_unknown_slugs(cx: &mut TestAppContext) {
        let _files = super::assets::prepare_test_files();
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            super::init(cx);
        });
        let (view, cx) = cx.add_window_view(super::MobileGallery::new);
        cx.simulate_resize(gpui::size(gpui::px(390.), gpui::px(844.)));
        static SELECTORS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
        let selectors = SELECTORS.get_or_init(|| {
            super::pages::ALL
                .iter()
                .map(|page| &*Box::leak(format!("gallery-rendered-{}", page.slug).into_boxed_str()))
                .collect()
        });
        assert_eq!(super::pages::ALL.len(), 44, "update the coverage inventory");
        for (index, page) in super::pages::ALL.iter().enumerate() {
            view.update(cx, |view, cx| {
                assert!(view.select(page.slug, cx));
                assert_eq!(view.current_slug(), page.slug);
                assert_eq!(view.visits(), index as u64 + 2);
            });
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
            // debug_bounds takes a static selector; the bounded catalog has 44 entries.
            let selector = selectors[index];
            let bounds = cx
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("{} did not render", page.slug));
            assert!(
                bounds.size.width > gpui::px(0.) && bounds.size.height > gpui::px(0.),
                "{} has empty layout",
                page.slug
            );
            assert!(cx.debug_bounds("gallery-content").is_some());
        }
        view.update(cx, |view, cx| {
            let before = (view.current_slug(), view.visits());
            assert!(!view.select("not-a-gallery-page", cx));
            assert_eq!((view.current_slug(), view.visits()), before);
        });
    }

    #[test]
    fn catalog_addresses_are_unique_and_round_trip() {
        for (index, page) in super::pages::ALL.iter().enumerate() {
            assert!(!page.slug.is_empty());
            assert_eq!(super::pages::find(page.slug), Some(index));
        }
        assert_eq!(super::pages::find("not-a-gallery-page"), None);
    }
}
