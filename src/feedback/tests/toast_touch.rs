use std::time::Duration;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::{desk, push, settle, toasts, wait};
use crate::{feedback::Toast, theme::Theme};

/// Where a finger lands on the toast stack.
fn stack(cx: &mut VisualTestContext) -> gpui::Point<gpui::Pixels> {
    let bounds = cx.debug_bounds("toast-stack").expect("the toast stack");
    bounds.origin + point(px(16.0), bounds.size.height / 2.0)
}

/// A phone has no hover, so a tap holds the toast, and it goes on its way after.
#[gpui::test]
fn a_tap_holds_a_toast_and_then_lets_it_go(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    settle(cx);
    let id = push(&view, Toast::new("Saved"), cx);
    // The toast rises on the wall clock; let it finish before touching it.
    std::thread::sleep(Duration::from_millis(400));
    settle(cx);

    let at = stack(cx);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    wait(Duration::from_secs(6), cx);
    assert_eq!(
        toasts(&view, cx),
        [(id, false)],
        "the tap held it past its own time"
    );
    wait(Duration::from_secs(6), cx);
    assert!(
        toasts(&view, cx).is_empty(),
        "the hold ended, and the toast folded away"
    );
}

/// A second tap does not strand a toast: the first tap's timer still ends the hold.
#[gpui::test]
fn a_second_tap_still_lets_the_toast_go(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    settle(cx);
    push(&view, Toast::new("Saved"), cx);
    // The toast rises on the wall clock; let it finish before touching it.
    std::thread::sleep(Duration::from_millis(400));
    settle(cx);

    let at = stack(cx);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    wait(Duration::from_secs(2), cx);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    wait(Duration::from_secs(7), cx);
    assert!(
        !toasts(&view, cx).is_empty(),
        "the tap still holds the toast open"
    );
    // The first tap's timer ends the hold five seconds in, and the toast's own
    // clock takes it from there. A second tap must not push that out.
    wait(Duration::from_secs(2), cx);
    assert!(
        toasts(&view, cx).is_empty(),
        "the first timer ended the hold, and the toast went on its way"
    );
}
