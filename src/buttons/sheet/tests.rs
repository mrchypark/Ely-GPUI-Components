use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers,
    ParentElement, Pixels, Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext,
    TouchPhase, VisualTestContext, Window, div, point, px, size,
};

use super::ActionSheet;
use crate::{
    primitives::FocusScope,
    theme::{ControlSize, Theme},
};

struct Sheet {
    root: FocusHandle,
    before: FocusHandle,
    bounds: Option<Bounds<Pixels>>,
    count: usize,
    open: bool,
    events: Vec<String>,
}

impl Render for Sheet {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let mut body = FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().track_focus(&self.before));
        if self.open {
            let close = owner.clone();
            let mut sheet = ActionSheet::new("test-sheet", move |_, cx| {
                close.update(cx, |view, cx| {
                    view.events.push("close".into());
                    view.open = false;
                    cx.notify();
                });
            })
            .title("Options")
            .message("Choose an action");
            if let Some(bounds) = self.bounds {
                sheet = sheet.usable_bounds(bounds);
            }
            for ix in 0..self.count {
                let owner = owner.clone();
                sheet = sheet.action(format!("Action {ix}"), move |_, cx| {
                    owner.update(cx, |view, _| view.events.push(format!("action {ix}")));
                });
            }
            body = body.child(sheet);
        }
        body
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn stage(cx: &mut TestAppContext, count: usize) -> (Entity<Sheet>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.reduced_motion = true;
            theme.touch_mode = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.set_rem_size(px(16.));
        let before = cx.focus_handle().tab_stop(true);
        window.focus(&before, cx);
        Sheet {
            root: cx.focus_handle(),
            before,
            bounds: Some(Bounds::new(
                point(px(12.), px(20.)),
                size(px(320.), px(300.)),
            )),
            count,
            open: false,
            events: Vec::new(),
        }
    });
    settle(cx);
    view.update(cx, |view, cx| {
        view.open = true;
        cx.notify();
    });
    settle(cx);
    (view, cx)
}

fn contained(inner: Bounds<Pixels>, outer: Bounds<Pixels>) {
    assert!(inner.left() >= outer.left(), "left: {inner:?} in {outer:?}");
    assert!(inner.top() >= outer.top(), "top: {inner:?} in {outer:?}");
    assert!(
        inner.right() <= outer.right(),
        "right: {inner:?} in {outer:?}"
    );
    assert!(
        inner.bottom() <= outer.bottom(),
        "bottom: {inner:?} in {outer:?}"
    );
}

fn scroll_to_end(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let frame = cx
        .debug_bounds("action-sheet-scroll")
        .expect("scroll viewport");
    cx.simulate_event(ScrollWheelEvent {
        position: frame.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(-10000.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    frame
}

fn assert_closed(view: &Entity<Sheet>, cx: &mut VisualTestContext, expected: &[&str]) {
    assert_eq!(view.read_with(cx, |view, _| view.events.clone()), expected);
    assert!(!view.read_with(cx, |view, _| view.open));
    let before = view.read_with(cx, |view, _| view.before.clone());
    assert!(cx.update(|window, _| before.is_focused(window)));
}

#[gpui::test]
fn bounded_actions_scroll_to_last_action_and_run_before_close(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, 20);
    let usable = view.read_with(cx, |view, _| view.bounds.unwrap());
    let first = cx.debug_bounds("action-sheet-row-Action 0").unwrap();
    let frame = scroll_to_end(cx);
    contained(frame, usable);
    assert!(cx.debug_bounds("action-sheet-row-Action 0").unwrap().top() < first.top());
    let last = cx.debug_bounds("action-sheet-row-Action 19").unwrap();
    contained(last, frame);
    assert!(last.size.height >= px(48.));
    assert!(
        view.read_with(cx, |view, _| view.events.is_empty()),
        "scroll did not activate"
    );
    cx.simulate_click(last.center(), Modifiers::none());
    settle(cx);
    assert_closed(&view, cx, &["action 19", "close"]);
}

#[gpui::test]
fn cancel_remains_reachable_after_bounds_shrink(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, 20);
    let usable = Bounds::new(point(px(24.), px(40.)), size(px(280.), px(80.)));
    view.update(cx, |view, cx| {
        view.bounds = Some(usable);
        cx.notify();
    });
    settle(cx);
    let frame = scroll_to_end(cx);
    contained(frame, usable);
    let cancel = cx.debug_bounds("action-sheet-row-Cancel").unwrap();
    contained(cancel, frame);
    cx.simulate_click(cancel.center(), Modifiers::none());
    settle(cx);
    assert_closed(&view, cx, &["close"]);
}

#[gpui::test]
fn touch_rows_preserve_large_rem_and_desktop_sizes(cx: &mut TestAppContext) {
    let (_, cx) = stage(cx, 1);
    for (rem, touch) in [(8., true), (48., true), (8., false)] {
        let expected = cx.update(|window, cx| {
            window.set_rem_size(px(rem));
            Theme::update(cx, |theme| theme.touch_mode = touch);
            let theme = cx.global::<Theme>();
            theme.touch_minimum(
                (theme.control_height(ControlSize::Lg)
                    + theme.control_height(ControlSize::Sm) / 2.0)
                    .to_pixels(window.rem_size()),
            )
        });
        settle(cx);
        for name in ["action-sheet-row-Action 0", "action-sheet-row-Cancel"] {
            let row = cx.debug_bounds(name).unwrap();
            assert_eq!(
                row.size.height, expected,
                "{name}, rem={rem}, touch={touch}"
            );
            if touch {
                assert!(row.size.width >= px(48.));
                assert!(row.size.height >= px(48.));
            }
        }
    }
}

#[gpui::test]
fn escape_and_outside_press_restore_focus(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, 20);
    cx.simulate_keystrokes("tab");
    let before = view.read_with(cx, |view, _| view.before.clone());
    assert!(!cx.update(|window, _| before.is_focused(window)));
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert_closed(&view, cx, &["close"]);
    view.update(cx, |view, cx| {
        view.events.clear();
        view.open = true;
        cx.notify();
    });
    settle(cx);
    cx.simulate_click(point(px(2.), px(2.)), Modifiers::none());
    settle(cx);
    assert_closed(&view, cx, &["close"]);
}

#[gpui::test]
fn short_sheet_keeps_natural_height_and_defaults_to_window_bounds(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, 1);
    let frame = cx.debug_bounds("action-sheet-scroll").unwrap();
    assert!(frame.size.height < px(300.));
    view.update(cx, |view, cx| {
        view.bounds = None;
        cx.notify();
    });
    settle(cx);
    let viewport =
        cx.update(|window, _| Bounds::new(point(px(0.), px(0.)), window.viewport_size()));
    contained(cx.debug_bounds("action-sheet-scroll").unwrap(), viewport);
}

#[gpui::test]
fn tab_reveals_last_action_cancel_and_wraparound(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, 20);
    let frame = cx.debug_bounds("action-sheet-scroll").unwrap();
    assert!(cx.debug_bounds("action-sheet-row-Action 19").unwrap().top() > frame.bottom());
    let mut first = None;
    let mut previous = cx.update(|window, cx| window.focused(cx));
    for ix in 0..20 {
        cx.simulate_keystrokes("tab");
        settle(cx);
        let focused = cx.update(|window, cx| window.focused(cx));
        assert!(
            focused.is_some() && focused != previous,
            "Tab advances to action {ix}"
        );
        if ix == 0 {
            first = focused.clone();
            contained(cx.debug_bounds("action-sheet-row-Action 0").unwrap(), frame);
        }
        previous = focused;
    }
    contained(
        cx.debug_bounds("action-sheet-row-Action 19").unwrap(),
        frame,
    );
    cx.simulate_keystrokes("tab");
    settle(cx);
    let cancel = cx.update(|window, cx| window.focused(cx));
    assert!(cancel.is_some() && cancel != previous);
    contained(cx.debug_bounds("action-sheet-row-Cancel").unwrap(), frame);

    cx.simulate_keystrokes("tab");
    settle(cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), first);
    contained(cx.debug_bounds("action-sheet-row-Action 0").unwrap(), frame);
    cx.simulate_keystrokes("shift-tab");
    settle(cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), cancel);
    contained(cx.debug_bounds("action-sheet-row-Cancel").unwrap(), frame);

    view.update(cx, |view, cx| {
        view.bounds = Some(Bounds::new(
            point(px(24.), px(40.)),
            size(px(280.), px(80.)),
        ));
        cx.notify();
    });
    settle(cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), cancel);
    contained(
        cx.debug_bounds("action-sheet-row-Cancel").unwrap(),
        cx.debug_bounds("action-sheet-scroll").unwrap(),
    );
    assert!(view.read_with(cx, |view, _| view.events.is_empty()));
}
