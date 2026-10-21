use super::super::{settle, tap};
use crate::{
    buttons::Button,
    onboarding::Hotspot,
    primitives::FocusScope,
    theme::{ActiveTheme, ControlSize, Theme},
};
use gpui::{
    Bounds, Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, MouseButton,
    ParentElement, Pixels, Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext,
    TouchPhase, Window, div, point, px, size,
};

struct Tip {
    root: FocusHandle,
    area: Bounds<Pixels>,
    hidden: bool,
    events: Vec<&'static str>,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (dismiss, after) = (cx.entity(), cx.entity());
        FocusScope::new(&self.root).root().size_full().child(
            div()
                .flex()
                .flex_col()
                .items_start()
                .children((!self.hidden).then(|| {
                    div()
                        .debug_selector(|| "bounded-hotspot-trigger".into())
                        .child(
                            Hotspot::new(
                                "bounded-tip",
                                "Pin a view",
                                "Keep this view available while working. ".repeat(80),
                                move |_, cx| {
                                    dismiss.update(cx, |view, cx| {
                                        view.events.push("seen");
                                        view.hidden = true;
                                        cx.notify();
                                    })
                                },
                            )
                            .usable_bounds(self.area),
                        )
                }))
                .child(
                    Button::new("after-bounded-tip", "After").on_click(move |_, _, cx| {
                        after.update(cx, |view, _| view.events.push("after"));
                    }),
                ),
        )
    }
}

pub(super) fn bounded_dismiss_after_shrink(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Tip {
        root: cx.focus_handle(),
        area: Bounds::new(point(px(20.), px(40.)), size(px(340.), px(350.))),
        hidden: false,
        events: Vec::new(),
    });
    cx.simulate_resize(size(px(390.), px(650.)));
    settle(cx);
    cx.update(|window, cx| {
        window.blur(cx);
        window.focus_next(cx);
    });
    let at = cx.debug_bounds("bounded-hotspot-trigger").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("hotspot-tip").is_some());
    let small = Bounds::new(point(px(45.), px(80.)), size(px(250.), px(160.)));
    view.update(cx, |view, cx| {
        view.area = small;
        cx.notify();
    });
    settle(cx);
    let panel = cx.debug_bounds("popover-panel").unwrap();
    assert!(panel.left() >= small.left() && panel.top() >= small.top());
    assert!(panel.right() <= small.right() && panel.bottom() <= small.bottom());
    let unscrolled = cx.debug_bounds("hotspot-tip").unwrap();
    assert!(
        unscrolled.bottom() > panel.bottom(),
        "Got it requires scrolling"
    );
    for delta in [point(px(-2000.), px(0.)), point(px(0.), px(-10000.))] {
        cx.simulate_event(ScrollWheelEvent {
            position: panel.center(),
            delta: ScrollDelta::Pixels(delta),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
        });
        settle(cx);
    }
    let tip = cx.debug_bounds("hotspot-tip").unwrap();
    assert!(tip.bottom() < unscrolled.bottom(), "content actually moved");
    let half = cx.update(|window, cx| {
        cx.theme().touch_minimum(
            cx.theme()
                .control_height(ControlSize::Sm)
                .to_pixels(window.rem_size()),
        ) / 2.
    });
    let at = point(tip.right() - px(12.), tip.bottom() - half);
    assert!(panel.contains(&at), "Got it is reachable");
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(view.read_with(cx, |view, _| view.hidden));
    assert_eq!(view.read_with(cx, |view, _| view.events.clone()), ["seen"]);
    assert!(cx.debug_bounds("popover-panel").is_none());
    assert!(cx.debug_bounds("bounded-hotspot-trigger").is_none());
    tap("tab", cx);
    tap("space", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.events.clone()),
        ["seen", "after"]
    );
}
