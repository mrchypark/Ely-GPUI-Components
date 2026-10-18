use super::super::{at, press, settle};
use crate::{
    calendar::{Answer, Attendee, Event, EventPopover},
    primitives::FocusScope,
    theme::{ActiveTheme, ControlSize, Theme},
};
use gpui::{
    Bounds, Context, FocusHandle, IntoElement, Modifiers, MouseButton, ParentElement, Pixels,
    Render, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase, Window, point, px, size,
};
use jiff::tz::TimeZone;

struct EventDetails {
    root: FocusHandle,
    area: Bounds<Pixels>,
    events: Vec<String>,
}

impl Render for EventDetails {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (edit, delete) = (cx.entity(), cx.entity());
        FocusScope::new(&self.root).root().child(
            EventPopover::new(
                "bounded-event",
                Event::timed("review", "Review", at(22, 11), at(22, 12), 0),
            )
            .zone(TimeZone::UTC)
            .attendees((0..16).map(|ix| {
                Attendee::new(
                    format!("Guest {ix}"),
                    format!("guest{ix}@example.invalid"),
                    Answer::Going,
                )
            }))
            .usable_bounds(self.area)
            .on_edit(move |key, _, cx| {
                edit.update(cx, |view, _| view.events.push(format!("edit {key}")));
            })
            .on_delete(move |key, _, cx| {
                delete.update(cx, |view, _| view.events.push(format!("delete {key}")));
            }),
        )
    }
}

pub(super) fn bounded_edit_after_shrink(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| EventDetails {
        root: cx.focus_handle(),
        area: Bounds::new(point(px(20.), px(40.)), size(px(340.), px(350.))),
        events: Vec::new(),
    });
    cx.simulate_resize(size(px(390.), px(650.)));
    settle(cx);
    cx.update(|window, cx| {
        window.blur(cx);
        window.focus_next(cx);
    });
    let before = cx.update(|window, cx| window.focused(cx));
    let at = cx.debug_bounds("event-chip review").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("event-popover review").is_some());
    let small = Bounds::new(point(px(45.), px(80.)), size(px(250.), px(160.)));
    view.update(cx, |view, cx| {
        view.area = small;
        cx.notify();
    });
    settle(cx);
    let panel = cx.debug_bounds("popover-panel").unwrap();
    assert!(panel.left() >= small.left() && panel.top() >= small.top());
    assert!(panel.right() <= small.right() && panel.bottom() <= small.bottom());
    let unscrolled = cx.debug_bounds("event-popover review").unwrap();
    assert!(
        unscrolled.bottom() > panel.bottom(),
        "actions require scrolling"
    );
    for delta in [point(px(-2000.), px(0.)), point(px(0.), px(-4000.))] {
        cx.simulate_event(ScrollWheelEvent {
            position: panel.center(),
            delta: ScrollDelta::Pixels(delta),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
        });
        settle(cx);
    }
    let details = cx.debug_bounds("event-popover review").unwrap();
    assert!(
        details.bottom() < unscrolled.bottom(),
        "content actually moved"
    );
    let half = cx.update(|window, cx| {
        cx.theme().touch_minimum(
            cx.theme()
                .control_height(ControlSize::Sm)
                .to_pixels(window.rem_size()),
        ) / 2.
    });
    let at = point(details.right() - px(12.), details.bottom() - half);
    assert!(
        panel.contains(&at),
        "Edit is reachable: {at:?} in {panel:?}"
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.events.clone()),
        ["edit review"]
    );
    assert!(cx.debug_bounds("popover-panel").is_none());
    assert_eq!(cx.update(|window, cx| window.focused(cx)), before);
    // A release/escape after the closing action must not repeat either callback.
    press("escape", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.events.clone()),
        ["edit review"]
    );
}
