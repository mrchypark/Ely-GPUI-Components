use ely_gpui_component::theme::Theme;
use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window, px, size};

use super::*;

struct FabDemo;
struct ActionBarDemo;
struct QuickActionsDemo;

impl Render for FabDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        fab(window, cx)
    }
}

impl Render for ActionBarDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        action_bar(window, cx)
    }
}

impl Render for QuickActionsDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        quick_actions(window, cx)
    }
}

#[gpui::test]
fn fab_actions_fit_phone_and_tablet_widths(cx: &mut TestAppContext) {
    cx.update(ely_gpui_component::init_for_tests);
    let (_, cx) = cx.add_window_view(|_, _| FabDemo);

    for (viewport, expected_width) in [(320., 304.), (768., 420.)] {
        cx.simulate_resize(size(px(viewport), px(844.)));
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
        }

        let frame = cx.debug_bounds("gallery-fab-frame").expect("FAB frame");
        let compose = cx.debug_bounds("gallery-fab-compose").expect("compose FAB");
        let add = cx.debug_bounds("gallery-fab-add").expect("add FAB");

        assert_eq!(frame.size.width, px(expected_width));
        assert!(frame.left() >= px(0.) && frame.right() <= px(viewport));
        assert!(compose.left() >= frame.left() && compose.right() <= frame.right());
        assert!(add.left() >= frame.left() && add.right() <= frame.right());
        assert!(compose.right() <= add.left(), "FAB actions overlap");
    }
}

#[gpui::test]
fn action_bars_fit_phone_and_tablet_widths(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| ActionBarDemo);
    let inside = |outer: Bounds<Pixels>, inner: Bounds<Pixels>| {
        inner.left() >= outer.left()
            && inner.top() >= outer.top()
            && inner.right() <= outer.right()
            && inner.bottom() <= outer.bottom()
    };
    let overlaps = |a: Bounds<Pixels>, b: Bounds<Pixels>| {
        a.left() < b.right() && a.right() > b.left() && a.top() < b.bottom() && a.bottom() > b.top()
    };

    for (viewport, expected_width) in [(320., 304.), (768., 560.)] {
        cx.simulate_resize(size(px(viewport), px(844.)));
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
        }

        let frame = cx
            .debug_bounds("gallery-action-bar-frame")
            .expect("ActionBar frame");
        let floating = cx
            .debug_bounds("gallery-action-bar-floating")
            .expect("floating ActionBar");
        let docked = cx
            .debug_bounds("gallery-action-bar-docked")
            .expect("docked ActionBar");
        let floating_actions = [
            cx.debug_bounds("gallery-action-reply").expect("reply"),
            cx.debug_bounds("gallery-action-forward").expect("forward"),
            cx.debug_bounds("gallery-action-archive").expect("archive"),
        ];
        let docked_actions = [
            cx.debug_bounds("gallery-action-cancel").expect("cancel"),
            cx.debug_bounds("gallery-action-save").expect("save"),
        ];

        assert_eq!(frame.size.width, px(expected_width));
        for bounds in [floating, docked]
            .into_iter()
            .chain(floating_actions)
            .chain(docked_actions)
        {
            assert!(
                inside(frame, bounds),
                "{viewport}: {bounds:?} outside {frame:?}"
            );
        }
        for actions in [floating_actions.as_slice(), docked_actions.as_slice()] {
            for (index, left) in actions.iter().enumerate() {
                for right in &actions[index + 1..] {
                    assert!(
                        !overlaps(*left, *right),
                        "actions overlap: {left:?}, {right:?}"
                    );
                }
            }
        }
        for action in floating_actions {
            assert!(inside(floating, action));
        }
        for action in docked_actions {
            assert!(inside(docked, action));
        }
    }
}

#[gpui::test]
fn quick_actions_fit_phone_and_tablet_widths(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| QuickActionsDemo);

    for (viewport, expected_width) in [(320., 304.), (768., 420.)] {
        cx.simulate_resize(size(px(viewport), px(844.)));
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
        }

        let frame = cx
            .debug_bounds("gallery-quick-actions-frame")
            .expect("QuickActions frame");
        let panel = cx
            .debug_bounds("gallery-quick-actions-panel")
            .expect("QuickActions panel");

        assert_eq!(frame.size.width, px(expected_width));
        assert!(frame.left() >= px(0.) && frame.right() <= px(viewport));
        assert!(panel.left() >= frame.left() && panel.right() <= frame.right());
    }
}
