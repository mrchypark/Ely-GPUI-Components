use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, TestAppContext,
    VisualTestContext, Window, div, point, prelude::*, px,
};

use super::settle;
use crate::{
    menus::{ContextMenu, Menu, MenuItem},
    theme::Theme,
};

struct Canvas {
    downs: usize,
    clicks: usize,
    picked: usize,
}

impl Render for Canvas {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (downs, clicks, picked) = (cx.entity(), cx.entity(), cx.entity());
        div()
            .id("canvas")
            .size_full()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                downs.update(cx, |view, cx| {
                    view.downs += 1;
                    cx.notify();
                });
            })
            .on_click(move |_, _, cx| {
                clicks.update(cx, |view, cx| {
                    view.clicks += 1;
                    cx.notify();
                });
            })
            .child(
                ContextMenu::new(
                    "area",
                    Menu::new()
                        .item(MenuItem::new("Cut").on_click(move |_, cx| {
                            picked.update(cx, |view, cx| {
                                view.picked += 1;
                                cx.notify();
                            });
                        }))
                        .item(MenuItem::new("Copy").disabled(true)),
                )
                .child(div().w(px(200.0)).h(px(120.0))),
            )
    }
}

fn tap(at: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn the_context_menu_panel_blocks_taps_to_the_canvas_underneath(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| Canvas {
        downs: 0,
        clicks: 0,
        picked: 0,
    });
    settle(cx);

    // A long press opens the context menu where the finger landed.
    let press_at = point(px(60.0), px(80.0));
    cx.simulate_mouse_move(press_at, None, Modifiers::none());
    cx.simulate_mouse_down(press_at, MouseButton::Left, Modifiers::none());
    cx.executor().advance_clock(Duration::from_millis(600));
    settle(cx);
    cx.simulate_mouse_up(press_at, MouseButton::Left, Modifiers::none());
    settle(cx);
    let panel = cx.debug_bounds("menu-panel-0").expect("the open menu");
    let copy = cx.debug_bounds("menu-row-Copy").expect("the disabled row");
    let cut = cx.debug_bounds("menu-row-Cut").expect("the enabled row");
    let base_downs = view.read_with(cx, |view, _| view.downs);
    let base_clicks = view.read_with(cx, |view, _| view.clicks);

    // A tap on the disabled row and on the panel's padding reaches neither the
    // canvas under the menu nor a row of the menu.
    for at in [
        copy.center(),
        point(panel.left() + px(2.0), panel.top() + px(2.0)),
    ] {
        tap(at, cx);
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.downs),
        base_downs,
        "a tap on the menu panel reached the canvas under it"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.clicks),
        base_clicks,
        "a tap on the menu panel reached the canvas under it"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.picked),
        0,
        "a tap on the disabled row ran it"
    );
    assert!(
        cx.debug_bounds("menu-panel-0").is_some(),
        "the menu closed on a tap that landed on it"
    );

    // The enabled row still runs its action, and still keeps the canvas out.
    tap(cut.center(), cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.picked),
        1,
        "the enabled row did not run"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.downs),
        base_downs,
        "a tap on the menu panel reached the canvas under it"
    );
    assert_eq!(
        view.read_with(cx, |view, _| view.clicks),
        base_clicks,
        "a tap on the menu panel reached the canvas under it"
    );

    // With the menu shut that same point is bare canvas, so the tap lands
    // there; without this the assertions above would prove nothing.
    assert!(
        cx.debug_bounds("menu-panel-0").is_none(),
        "the menu stayed open"
    );
    let bare = point(panel.left() + px(2.0), panel.top() + px(2.0));
    tap(bare, cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.downs),
        base_downs + 1,
        "the control tap missed the canvas, so the test proves nothing"
    );
}
