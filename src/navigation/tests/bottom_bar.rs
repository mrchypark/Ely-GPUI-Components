use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, SharedString, Styled, TestAppContext,
    Window, div, px,
};

use super::super::{BottomNavBar, BottomNavItem};
use crate::{primitives::IconName, theme::Theme};

struct Host {
    selected: SharedString,
    inset: gpui::Pixels,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let items = [
            BottomNavItem::new("home", IconName::Search, "Home"),
            BottomNavItem::new("inbox", IconName::Inbox, "Inbox").badge(4),
            BottomNavItem::new("settings", IconName::Settings, "Settings"),
        ];
        div().w(px(390.)).h(px(200.)).flex().items_end().child(
            BottomNavBar::new("bottom-nav", items, self.selected.clone())
                .bottom_inset(self.inset)
                .on_select(move |id, _, cx| {
                    let id = id.clone();
                    entity.update(cx, |host, cx| {
                        host.selected = id;
                        cx.notify();
                    });
                }),
        )
    }
}

#[gpui::test]
fn bottom_nav_bar_selects_tabs_and_applies_bottom_inset(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (host, cx) = cx.add_window_view(|_, _| Host {
        selected: "home".into(),
        inset: px(20.),
    });
    cx.run_until_parked();

    let bar = cx.debug_bounds("bottom-nav-bar").expect("bottom nav bar");
    let inbox = cx.debug_bounds("bottom-nav-inbox").expect("inbox tab");
    assert!(
        bar.bottom() - inbox.bottom() >= px(20.),
        "bottom inset pads below tab items: bar={bar:?}, inbox={inbox:?}"
    );
    assert!(inbox.size.height >= px(44.));

    cx.simulate_click(inbox.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        host.read_with(cx, |host, _| host.selected.clone()),
        SharedString::from("inbox")
    );
}
