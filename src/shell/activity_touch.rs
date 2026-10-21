use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, px,
};

use crate::{
    primitives::IconName,
    shell::{ActivityBar, ActivityItem},
    theme::Theme,
};

/// An activity rail with one destination.
struct Rail;

impl Render for Rail {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(80.0)).h(px(200.0)).child(
            ActivityBar::new("rail")
                .item(ActivityItem::new("files", IconName::Folder, "Files"))
                .selected("files"),
        )
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn an_activity_item_is_a_thumb_on_a_phone(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Rail);
    settle(cx);
    let item = cx
        .debug_bounds("activity-files")
        .expect("the activity item");
    assert!(
        item.size.width >= px(48.) && item.size.height >= px(48.),
        "the activity item is a touch target: {item:?}"
    );
}

#[gpui::test]
fn an_activity_item_keeps_the_desktop_size(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Rail);
    settle(cx);
    let item = cx
        .debug_bounds("activity-files")
        .expect("the activity item");
    assert!(
        item.size.width == px(48.),
        "the desktop keeps the rail's own width: {item:?}"
    );
}
