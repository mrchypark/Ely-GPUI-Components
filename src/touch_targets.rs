//! The touch floor: every tappable element keeps a 48 CSS pixel hit area.

use crate::{
    buttons::Button,
    forms::{Input, TextInput},
    lists::ListItem,
    navigation::{NavGroup, NavItem},
    overlays::Dialog,
    primitives::IconName,
    shell::{TabBar, WindowTab},
    theme::{ControlSize, Theme},
};
use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, px,
};

/// Everything the sprint's touch targets cover, drawn once.
struct Targets {
    field: Entity<TextInput>,
}

impl Render for Targets {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let field = self.field.clone();
        div()
            .w(px(320.0))
            .child(ListItem::new("item", "Inbox"))
            .child(NavItem::new("nav", IconName::House, "Home"))
            .child(NavGroup::new("group", "Places", true).child(NavItem::new(
                "nav-in-group",
                IconName::Star,
                "Starred",
            )))
            .child(
                TabBar::new("tabs")
                    .tab(WindowTab::new("doc", "Document"))
                    .selected("doc"),
            )
            .child(Input::new(&field).size(ControlSize::Sm))
            .child(Dialog::new("dialog", "Delete", |_, _| {}).action(|_| {
                Button::new("dialog-ok", "Delete").debug_selector(|| "dialog-action-button".into())
            }))
    }
}

fn setup(cx: &mut TestAppContext) -> &mut VisualTestContext {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, cx| Targets {
        field: cx.new(|cx| TextInput::new(window, cx)),
    });
    cx.run_until_parked();
    cx
}

/// The tappable elements the sprint named, and the selector each one answers to.
const TARGETS: [&str; 6] = [
    "list-item-Inbox",
    "nav-item",
    "nav-group-title",
    "window-tab-doc",
    "input-root",
    "dialog-action-button",
];

#[gpui::test]
fn every_named_touch_target_keeps_a_48_pixel_hit_area(cx: &mut TestAppContext) {
    let cx = setup(cx);
    for selector in TARGETS {
        let bounds = cx.debug_bounds(selector).expect(selector);
        assert!(
            bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
            "{selector} is {bounds:?}"
        );
    }
}

#[gpui::test]
fn a_desktop_density_keeps_the_small_targets_small(cx: &mut TestAppContext) {
    let cx = setup(cx);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    cx.run_until_parked();
    for selector in TARGETS {
        let bounds = cx.debug_bounds(selector).expect(selector);
        assert!(
            bounds.size.height < px(48.),
            "{selector} kept a touch floor it never asked for: {bounds:?}"
        );
    }
}
