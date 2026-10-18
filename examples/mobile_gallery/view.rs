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

/// A single mounted catalog page; navigation remains outside its scroll viewport.
pub struct MobileGallery {
    focus: FocusHandle,
    page: usize,
    visits: u64,
    catalog: bool,
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
        self.visits += 1;
        cx.set_global(crate::probe::Probes::default());
        self.scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
        true
    }

    pub fn current_slug(&self) -> &'static str {
        pages::ALL[self.page].slug
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
            div()
                .debug_selector(move || format!("gallery-rendered-{slug}"))
                .child((page.render)(window, cx))
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
