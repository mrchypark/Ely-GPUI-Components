use std::time::Duration;

use gpui::{
    Context, IntoElement, KeyUpEvent, Keystroke, Modifiers, MouseButton, ParentElement, Render,
    Styled, TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div, point,
    px,
};

use super::{ConfirmButton, ConfirmMode, HOLD};
use crate::theme::Theme;

struct Probe {
    mode: ConfirmMode,
    confirms: usize,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            ConfirmButton::new("confirm", "Delete", self.mode)
                .on_confirm(move |_, cx| view.update(cx, |probe, _| probe.confirms += 1)),
        )
    }
}

fn open(
    mode: ConfirmMode,
    cx: &mut TestAppContext,
) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| Probe { mode, confirms: 0 });
    cx.update(|window, _| window.activate_window());
    draw(cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn touch(
    phase: TouchPhase,
    id: TouchId,
    center: gpui::Point<gpui::Pixels>,
    cx: &mut VisualTestContext,
) {
    cx.simulate_event(TouchEvent {
        id,
        phase,
        position: center,
        predicted_position: None,
        force: None,
    });
}

fn advance(cx: &mut VisualTestContext, duration: Duration) {
    cx.executor().advance_clock(duration);
    cx.run_until_parked();
}

fn press(key: &'static str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    draw(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).expect("a key"),
    });
    draw(cx);
}

#[gpui::test]
fn touch_long_press_confirms_after_one_hold_duration(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    let center = cx
        .debug_bounds("confirm-button")
        .expect("confirm button")
        .center();
    touch(TouchPhase::Started, TouchId(1), center, cx);
    advance(cx, Duration::from_millis(499));
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    advance(cx, Duration::from_millis(1));
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    advance(cx, Duration::from_millis(600));
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    advance(cx, Duration::from_millis(100));
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 1);
}

#[gpui::test]
fn cancelled_touch_long_press_does_not_confirm(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    let center = cx
        .debug_bounds("confirm-button")
        .expect("confirm button")
        .center();
    touch(TouchPhase::Started, TouchId(1), center, cx);
    advance(cx, Duration::from_millis(500));
    advance(cx, HOLD / 2);
    touch(TouchPhase::Cancelled, TouchId(1), center, cx);
    advance(cx, HOLD);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
}

#[gpui::test]
fn moving_outside_after_recognition_cancels_the_hold(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    let bounds = cx.debug_bounds("confirm-button").expect("confirm button");
    let center = bounds.center();
    let outside = point(bounds.right() + px(20.), center.y);
    touch(TouchPhase::Started, TouchId(1), center, cx);
    advance(cx, Duration::from_millis(500));
    touch(TouchPhase::Moved, TouchId(1), outside, cx);
    advance(cx, HOLD);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    touch(TouchPhase::Ended, TouchId(1), outside, cx);
}

#[gpui::test]
fn mouse_exit_cancels_the_hold(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    let bounds = cx.debug_bounds("confirm-button").expect("confirm button");
    let center = bounds.center();
    let outside = point(bounds.right() + px(20.), center.y);
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
    advance(cx, HOLD);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    cx.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
}

#[gpui::test]
fn cancelled_hold_after_keyboard_arm_requires_two_new_keyboard_activations(
    cx: &mut TestAppContext,
) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    cx.update(|window, cx| window.focus_next(cx));
    draw(cx);

    press("enter", cx);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);

    let center = cx
        .debug_bounds("confirm-button")
        .expect("confirm button")
        .center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
    advance(cx, HOLD / 2);
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
    draw(cx);

    press("enter", cx);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 1);
}

#[gpui::test]
fn moving_before_recognition_keeps_the_gesture_out_of_hold(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    let bounds = cx.debug_bounds("confirm-button").expect("confirm button");
    let center = bounds.center();
    let panned = point(center.x + px(20.), center.y);
    touch(TouchPhase::Started, TouchId(1), center, cx);
    touch(TouchPhase::Moved, TouchId(1), panned, cx);
    advance(cx, Duration::from_millis(500) + HOLD);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    touch(TouchPhase::Ended, TouchId(1), panned, cx);
}

#[gpui::test]
fn touch_tap_does_not_start_a_confirming_hold(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Hold, cx);
    let center = cx
        .debug_bounds("confirm-button")
        .expect("confirm button")
        .center();
    touch(TouchPhase::Started, TouchId(1), center, cx);
    advance(cx, Duration::from_millis(100));
    touch(TouchPhase::Ended, TouchId(1), center, cx);
    advance(cx, HOLD);
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
}

#[gpui::test]
fn twice_mode_keeps_the_double_click_fallback(cx: &mut TestAppContext) {
    let (view, cx) = open(ConfirmMode::Twice, cx);
    let center = cx
        .debug_bounds("confirm-button")
        .expect("confirm button")
        .center();
    cx.simulate_click(center, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_click(center, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 1);
}
