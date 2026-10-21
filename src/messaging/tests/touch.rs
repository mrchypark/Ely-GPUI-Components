use std::{path::Path, time::Duration};

use gpui::{
    Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px, size,
};
use jiff::Timestamp;

use super::settle;
use crate::{
    data_display::Avatar,
    menus::{Menu, MenuItem},
    messaging::{ChatMessage, Gif, GifPicker, Gifs, HuddleIndicator, MessageThread},
    theme::Theme,
};

/// The messaging controls a finger has to reach, and what they did.
struct Controls {
    joined: usize,
    opened: usize,
    picked: Vec<String>,
    menu_ran: usize,
}

impl Render for Controls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (joined, opened, picked, menu_ran) =
            (cx.entity(), cx.entity(), cx.entity(), cx.entity());
        div()
            .flex()
            .flex_col()
            .child(
                HuddleIndicator::new("huddle", [Avatar::new("ana", "Ana Lima")]).on_join(
                    move |_, cx| {
                        joined.update(cx, |host, cx| {
                            host.joined += 1;
                            cx.notify();
                        })
                    },
                ),
            )
            .child(
                MessageThread::new("thread", 3, Timestamp::now()).on_open(move |_, cx| {
                    opened.update(cx, |host, cx| {
                        host.opened += 1;
                        cx.notify();
                    })
                }),
            )
            .child(
                GifPicker::new(
                    "gifs",
                    Gifs::Found(vec![Gif::new("a", "a", Path::new("/nowhere.gif"), 1.5)]),
                )
                .on_pick(move |key, _, cx| {
                    picked.update(cx, |host, cx| host.picked.push(key.to_string()))
                }),
            )
            .child(
                ChatMessage::new("m1", "Ana Lima", Timestamp::now()).context_menu(
                    Menu::new().item(MenuItem::new("Copy").on_click(move |_, cx| {
                        menu_ran.update(cx, |host, cx| {
                            host.menu_ran += 1;
                            cx.notify();
                        })
                    })),
                ),
            )
    }
}

fn touch(cx: &mut TestAppContext) -> (Entity<Controls>, &mut VisualTestContext) {
    super::setup(cx);
    cx.update(|cx| Theme::update(cx, |theme| theme.touch_mode = true));
    let (host, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(16.));
        Controls {
            joined: 0,
            opened: 0,
            picked: Vec::new(),
            menu_ran: 0,
        }
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

fn tap(at: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext) {
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(
        at.x >= px(0.) && at.y >= px(0.) && at.x <= viewport.width && at.y <= viewport.height,
        "a tap at {at:?} lands outside the window {viewport:?}"
    );
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn the_huddle_chip_keeps_a_finger_sized_target(cx: &mut TestAppContext) {
    let (_, cx) = touch(cx);
    let chip = cx.debug_bounds("huddle").expect("the chip draws");
    assert!(
        chip.size.width >= px(48.) && chip.size.height >= px(48.),
        "the huddle chip is too small to tap: {chip:?}"
    );
}

#[gpui::test]
fn the_thread_opener_keeps_a_finger_sized_target(cx: &mut TestAppContext) {
    let (_, cx) = touch(cx);
    let row = cx.debug_bounds("message-thread").expect("the row draws");
    assert!(
        row.size.width >= px(48.) && row.size.height >= px(48.),
        "the thread opener is too small to tap: {row:?}"
    );
}

#[gpui::test]
fn the_gif_tile_keeps_a_finger_sized_target(cx: &mut TestAppContext) {
    let (_, cx) = touch(cx);
    let tile = cx.debug_bounds("gif a").expect("the tile draws");
    assert!(
        tile.size.width >= px(48.) && tile.size.height >= px(48.),
        "the gif tile is too small to tap: {tile:?}"
    );
}

#[gpui::test]
fn a_tap_on_each_control_reaches_it(cx: &mut TestAppContext) {
    let (host, cx) = touch(cx);
    tap(cx.debug_bounds("huddle").unwrap().center(), cx);
    tap(cx.debug_bounds("message-thread").unwrap().center(), cx);
    tap(cx.debug_bounds("gif a").unwrap().center(), cx);
    let (joined, opened, picked) = host.read_with(cx, |host, _| {
        (host.joined, host.opened, host.picked.clone())
    });
    assert_eq!(joined, 1, "the tap missed the huddle chip");
    assert_eq!(opened, 1, "the tap missed the thread opener");
    assert_eq!(picked, ["a"], "the tap missed the gif tile");
}

#[gpui::test]
fn a_long_press_on_a_message_opens_its_menu(cx: &mut TestAppContext) {
    let (host, cx) = touch(cx);
    let message = cx.debug_bounds("message m1").expect("the message draws");
    let at = message.center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.executor().advance_clock(Duration::from_millis(600));
    settle(cx);
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("menu-panel-0").is_some(),
        "a long press did not open the message menu"
    );
    let copy = cx
        .debug_bounds("menu-row-Copy")
        .expect("the menu row draws");
    tap(copy.center(), cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.menu_ran),
        1,
        "the menu row did not run"
    );
}

#[gpui::test]
fn a_short_press_on_a_message_leaves_its_menu_shut(cx: &mut TestAppContext) {
    let (_, cx) = touch(cx);
    let message = cx.debug_bounds("message m1").expect("the message draws");
    tap(message.center(), cx);
    assert!(
        cx.debug_bounds("menu-panel-0").is_none(),
        "a short press opened the message menu"
    );
}
