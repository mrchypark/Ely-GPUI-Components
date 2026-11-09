use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, TestAppContext,
    VisualTestContext, Window, div, point, prelude::*, px,
};

use super::settle;
use crate::{
    interaction::TouchMenu,
    menus::{Menu, MenuItem},
    theme::Theme,
};

struct Desk {
    downs: usize,
    picked: usize,
}

impl Render for Desk {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (downs, picked) = (cx.entity(), cx.entity());
        div()
            .id("canvas")
            .size_full()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                downs.update(cx, |desk, cx| {
                    desk.downs += 1;
                    cx.notify();
                });
            })
            .child(
                TouchMenu::new(
                    "area",
                    Menu::new()
                        .item(MenuItem::new("Cut").on_click(move |_, cx| {
                            picked.update(cx, |desk, cx| {
                                desk.picked += 1;
                                cx.notify();
                            });
                        }))
                        .item(MenuItem::new("Copy").disabled(true)),
                )
                .child(div().w(px(200.0)).h(px(120.0))),
            )
    }
}

fn touch_desk(cx: &mut TestAppContext) -> (gpui::Entity<Desk>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| Desk {
        downs: 0,
        picked: 0,
    });
    settle(cx);
    (view, cx)
}

fn tap(at: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn a_long_press_opens_the_touch_menu_and_the_panel_holds_the_press(cx: &mut TestAppContext) {
    let (view, cx) = touch_desk(cx);
    let at = point(px(60.0), px(80.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.executor().advance_clock(Duration::from_millis(600));
    settle(cx);
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    let panel = cx.debug_bounds("menu-panel-0").expect("the menu panel");
    let base_downs = view.read_with(cx, |desk, _| desk.downs);
    tap(panel.center(), cx);
    assert_eq!(
        view.read_with(cx, |desk, _| desk.downs),
        base_downs,
        "a tap on the menu panel reached the canvas under it"
    );
}

#[gpui::test]
fn a_short_press_leaves_the_touch_menu_shut(cx: &mut TestAppContext) {
    let (_, cx) = touch_desk(cx);
    let at = point(px(60.0), px(80.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("menu-panel-0").is_none(),
        "a short press opened the menu"
    );
}
