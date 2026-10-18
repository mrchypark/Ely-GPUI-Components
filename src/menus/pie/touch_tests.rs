use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers, MouseButton,
    ParentElement, Pixels, Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext,
    TouchPhase, VisualTestContext, Window, div, point, px, size,
};

use super::{PieItem, PieMenu};
use crate::{
    buttons::Button,
    primitives::{FocusScope, IconName},
    theme::Theme,
};

struct TouchPie {
    root: FocusHandle,
    before: FocusHandle,
    destination: FocusHandle,
    bounds: Bounds<Pixels>,
    primary: usize,
    selected: Vec<usize>,
}

impl Render for TouchPie {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let items = (0..20).map(|ix| {
            let owner = owner.clone();
            let destination = self.destination.clone();
            PieItem::new(IconName::Copy, format!("Item {ix}")).on_click(move |window, cx| {
                owner.update(cx, |view, _| view.selected.push(ix));
                if ix == 0 {
                    window.focus(&destination, cx);
                }
            })
        });
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().track_focus(&self.before))
            .child(Button::new("destination", "Destination").focus_handle(&self.destination))
            .child(
                PieMenu::new("touch-pie", items)
                    .usable_bounds(self.bounds)
                    .child(div().debug_selector(|| "pie-primary".into()).child(
                        Button::new("primary", "Primary").on_click({
                            let owner = owner.clone();
                            move |_, _, cx| {
                                owner.update(cx, |view, _| view.primary += 1);
                            }
                        }),
                    )),
            )
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn stage(cx: &mut TestAppContext) -> (Entity<TouchPie>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.set_rem_size(px(16.));
        let before = cx.focus_handle().tab_stop(true);
        window.focus(&before, cx);
        TouchPie {
            root: cx.focus_handle(),
            before,
            destination: cx.focus_handle().tab_stop(true),
            bounds: Bounds::new(point(px(20.), px(40.)), size(px(280.), px(240.))),
            primary: 0,
            selected: Vec::new(),
        }
    });
    settle(cx);
    (view, cx)
}

fn tap(at: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}

fn open(cx: &mut VisualTestContext) {
    let trigger = cx
        .debug_bounds("pie-touch-trigger")
        .expect("visible Actions button");
    assert!(trigger.size.width >= px(48.) && trigger.size.height >= px(48.));
    tap(trigger.center(), cx);
    assert!(cx.debug_bounds("action-sheet-scroll").is_some());
}

fn scroll(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let frame = cx.debug_bounds("action-sheet-scroll").unwrap();
    cx.simulate_event(ScrollWheelEvent {
        position: frame.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(-10000.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    frame
}

fn contained(inner: Bounds<Pixels>, outer: Bounds<Pixels>) {
    assert!(inner.left() >= outer.left());
    assert!(inner.top() >= outer.top());
    assert!(inner.right() <= outer.right());
    assert!(inner.bottom() <= outer.bottom());
}

fn closed(view: &Entity<TouchPie>, cx: &mut VisualTestContext) {
    assert!(cx.debug_bounds("action-sheet-scroll").is_none());
    let before = view.read_with(cx, |view, _| view.before.clone());
    assert!(cx.update(|window, _| before.is_focused(window)));
}

#[gpui::test]
fn touch_opener_does_not_steal_the_wrapped_primary_action(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    tap(cx.debug_bounds("pie-primary").unwrap().center(), cx);
    assert_eq!(view.read_with(cx, |view, _| view.primary), 1);
    assert!(cx.debug_bounds("action-sheet-scroll").is_none());
    open(cx);
    assert_eq!(view.read_with(cx, |view, _| view.primary), 1);
    assert!(view.read_with(cx, |view, _| view.selected.is_empty()));
}

#[gpui::test]
fn touch_sheet_forwards_resized_bounds_and_runs_last_callback_once(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    open(cx);
    let usable = Bounds::new(point(px(30.), px(60.)), size(px(260.), px(160.)));
    view.update(cx, |view, cx| {
        view.bounds = usable;
        cx.notify();
    });
    settle(cx);
    let frame = scroll(cx);
    contained(frame, usable);
    let last = cx.debug_bounds("action-sheet-row-Item 19").unwrap();
    contained(last, frame);
    assert!(view.read_with(cx, |view, _| view.selected.is_empty()));
    tap(last.center(), cx);
    assert_eq!(view.read_with(cx, |view, _| view.selected.clone()), [19]);
    assert_eq!(view.read_with(cx, |view, _| view.primary), 0);
    closed(&view, cx);
}

#[gpui::test]
fn touch_outside_and_cancel_dismiss_without_selecting(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    open(cx);
    tap(point(px(2.), px(2.)), cx);
    closed(&view, cx);
    open(cx);
    scroll(cx);
    tap(
        cx.debug_bounds("action-sheet-row-Cancel").unwrap().center(),
        cx,
    );
    closed(&view, cx);
    assert!(view.read_with(cx, |view, _| view.selected.is_empty()));
    assert_eq!(view.read_with(cx, |view, _| view.primary), 0);
}

#[gpui::test]
fn touch_escape_restores_focus_and_desktop_has_no_touch_button(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    open(cx);
    cx.simulate_keystrokes("escape");
    settle(cx);
    closed(&view, cx);
    assert!(view.read_with(cx, |view, _| view.selected.is_empty()));
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    assert!(cx.debug_bounds("pie-touch-trigger").is_none());
}

#[gpui::test]
fn touch_action_runs_once_after_close_and_keeps_its_focus_destination(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    open(cx);
    tap(
        cx.debug_bounds("action-sheet-row-Item 0").unwrap().center(),
        cx,
    );
    let destination = view.read_with(cx, |view, _| view.destination.clone());
    assert_eq!(view.read_with(cx, |view, _| view.selected.clone()), [0]);
    assert!(cx.debug_bounds("action-sheet-scroll").is_none());
    assert!(cx.update(|window, _| destination.is_focused(window)));
    open(cx);
    tap(point(px(2.), px(2.)), cx);
    assert_eq!(view.read_with(cx, |view, _| view.selected.clone()), [0]);
    assert!(cx.debug_bounds("action-sheet-scroll").is_none());
    assert!(cx.update(|window, _| destination.is_focused(window)));
}
