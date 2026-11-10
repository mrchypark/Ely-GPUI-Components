use gpui::{
    AppContext as _, Context, Entity, InteractiveElement, IntoElement, Modifiers, ParentElement,
    Render, ScrollHandle, SharedString, Styled, TestAppContext, Window, div, point, px,
};

use super::setup;
use crate::{
    forms::Choice,
    navigation::{
        Breadcrumb, Crumb, NavigationMenu, Sections, Steps, TabOverflowMenu, TableOfContents,
    },
    theme::{Density, Theme},
};

struct TouchNavigation {
    sections: Entity<Sections>,
    picked: Option<(usize, SharedString)>,
}

impl Render for TouchNavigation {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let crumb = cx.entity();
        let steps = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                NavigationMenu::new("target-menu")
                    .entry("File", [Choice::new("new", "New")])
                    .on_select(|_, _, _| {}),
            )
            .child(
                div().flex().debug_selector(|| "touch-crumb".into()).child(
                    Breadcrumb::new(
                        "target-crumb",
                        [Crumb::new("home", "Home"), Crumb::new("docs", "Docs")],
                    )
                    .on_select(move |ix, value, _, cx| {
                        let value = value.clone();
                        crumb.update(cx, |view, _| view.picked = Some((ix, value)));
                    }),
                ),
            )
            .child(
                div()
                    .flex()
                    .debug_selector(|| "touch-overflow".into())
                    .child(TabOverflowMenu::new(
                        "target-overflow",
                        [Choice::new("a", "A"), Choice::new("b", "B")],
                    )),
            )
            .child(div().flex().debug_selector(|| "touch-toc".into()).child(
                TableOfContents::new("target-toc", &self.sections).entry(
                    "intro",
                    "Introduction",
                    0,
                ),
            ))
            .child(
                Steps::new(
                    "target-steps",
                    [
                        Choice::new("account", "Account"),
                        Choice::new("plan", "Plan"),
                    ],
                    1,
                )
                .on_select(move |ix, _, cx| {
                    steps.update(cx, |view, _| {
                        view.picked = Some((ix, SharedString::from("step")))
                    });
                }),
            )
    }
}

#[gpui::test]
fn touch_navigation_controls_keep_thumb_sized_hit_areas(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| {
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.density = Density::Compact;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.set_rem_size(px(8.));
        TouchNavigation {
            sections: cx.new(|_| Sections::new(ScrollHandle::new())),
            picked: None,
        }
    });
    cx.run_until_parked();

    let menu = cx.debug_bounds("navigation-menu-title-0").unwrap();
    let crumb = cx.debug_bounds("touch-crumb").unwrap();
    let overflow = cx.debug_bounds("touch-overflow").unwrap();
    let toc = cx.debug_bounds("touch-toc").unwrap();
    let steps = cx.debug_bounds("steps-root").unwrap();
    for (name, bounds) in [
        ("menu trigger", menu),
        ("breadcrumb", crumb),
        ("overflow trigger", overflow),
        ("contents row", toc),
        ("completed step", steps),
    ] {
        assert!(
            bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
            "{name} is {bounds:?}"
        );
    }

    cx.simulate_click(
        point(crumb.left() + px(8.), crumb.bottom() - px(2.)),
        Modifiers::none(),
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.picked.clone()),
        Some((0, SharedString::from("home")))
    );

    cx.simulate_click(
        point(steps.left() + px(20.), steps.bottom() - px(2.)),
        Modifiers::none(),
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.picked.clone()),
        Some((0, SharedString::from("step")))
    );

    cx.simulate_click(
        point(menu.left() + px(8.), menu.bottom() - px(2.)),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert!(cx.debug_bounds("navigation-menu-panel").is_some());
}
