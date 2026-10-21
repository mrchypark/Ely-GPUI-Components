use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use jiff::Timestamp;

use crate::{
    chat::{Conversation, ConversationItem},
    theme::Theme,
};

/// A conversation, and what its menu asked for.
struct Chats {
    deleted: Vec<SharedString>,
}

impl Render for Chats {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let conversation = Conversation {
            key: SharedString::from("conv"),
            title: SharedString::from("Lift blends"),
            at: Timestamp::UNIX_EPOCH,
            pinned: false,
        };
        div()
            .w(px(320.))
            .child(ConversationItem::new("conv", conversation).actions(
                |_, _, _| {},
                |_, _, _| {},
                move |key, _, cx| owner.update(cx, |state, _| state.deleted.push(key.clone())),
            ))
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn a_conversation_keeps_its_menu_in_touch_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, _| Chats {
        deleted: Vec::new(),
    });
    settle(cx);
    let trigger = cx
        .debug_bounds("conversation-actions-touch conv")
        .expect("the conversation keeps its menu");
    assert!(
        trigger.size.width >= px(48.) && trigger.size.height >= px(48.),
        "the menu trigger is a touch target: {trigger:?}"
    );
    let at = trigger.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("menu-panel-0").is_some(),
        "the tap opens the conversation actions"
    );
    let row = cx
        .debug_bounds("menu-row-Delete")
        .expect("the menu holds its items");
    assert!(
        row.size.height >= px(48.),
        "the item is a touch target: {row:?}"
    );
    let at = row.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        view.read_with(cx, |state, _| state.deleted.clone()),
        ["conv"],
        "the menu reaches its action"
    );
}

#[gpui::test]
fn a_conversation_hides_its_menu_on_the_desktop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Chats {
        deleted: Vec::new(),
    });
    settle(cx);
    assert!(
        cx.debug_bounds("conversation-actions-touch conv").is_none(),
        "on the desktop the menu waits for a hover"
    );
}
