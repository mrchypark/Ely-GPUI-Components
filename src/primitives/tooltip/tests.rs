use super::*;
use crate::{buttons::Button, primitives::FocusScope, theme::Theme};
use gpui::{FocusHandle, Modifiers, TestAppContext, VisualTestContext, px, size};
use std::cell::Cell;

struct Stage {
    root: FocusHandle,
    before: FocusHandle,
    outside: FocusHandle,
    area: Option<Bounds<Pixels>>,
    help: bool,
    primary: usize,
    builds: Rc<Cell<usize>>,
}

impl Render for Stage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let builds = self.builds.clone();
        let tip = if self.help {
            HelpTooltip::new("help", "Help text ".repeat(200))
                .touch_label("Help for name")
                .when_some(self.area, |tip, area| tip.usable_bounds(area))
                .into_any_element()
        } else {
            TooltipTrigger::new(
                "tip",
                div().debug_selector(|| "primary".into()).child(
                    Button::new("primary-button", "Primary").on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        view.update(cx, |stage, _| stage.primary += 1);
                    }),
                ),
                move |_, _| {
                    builds.set(builds.get() + 1);
                    div()
                        .w(px(900.))
                        .h(px(900.))
                        .child("Recorded content")
                        .into_any_element()
                },
            )
            .touch_label("Primary details")
            .when_some(self.area, |tip, area| tip.usable_bounds(area))
            .into_any_element()
        };
        let parent = cx.entity();
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .relative()
            .child(div().id("before").track_focus(&self.before))
            .child(
                div()
                    .id("outer")
                    .on_click(move |_, _, cx| {
                        parent.update(cx, |stage, _| stage.primary += 100);
                    })
                    .child(tip),
            )
            .child(
                div()
                    .id("passive-dismiss")
                    .debug_selector(|| "passive-dismiss".into())
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .size(px(48.))
                    // A passive dismissal must not focus its enclosing root.
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default()),
            )
            .child(
                div()
                    .id("outside-focus")
                    .debug_selector(|| "outside-focus".into())
                    .track_focus(&self.outside)
                    .absolute()
                    .bottom_0()
                    .right_0()
                    .size(px(48.)),
            )
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn fixture(cx: &mut TestAppContext, help: bool) -> (gpui::Entity<Stage>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Stage {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        outside: cx.focus_handle(),
        area: Some(Bounds::new(
            point(px(20.), px(60.)),
            size(px(280.), px(300.)),
        )),
        help,
        primary: 0,
        builds: Rc::new(Cell::new(0)),
    });
    cx.simulate_resize(size(px(320.), px(500.)));
    settle(cx);
    refocus(&view, cx);
    (view, cx)
}

fn refocus(view: &gpui::Entity<Stage>, cx: &mut VisualTestContext) {
    let before = view.read_with(cx, |stage, _| stage.before.clone());
    cx.update(|window, cx| window.focus(&before, cx));
}

fn restored(view: &gpui::Entity<Stage>, cx: &mut VisualTestContext) {
    let before = view.read_with(cx, |stage, _| stage.before.clone());
    assert!(cx.update(|window, _| before.is_focused(window)));
}

fn tap(cx: &mut VisualTestContext, selector: &'static str) {
    let at = cx.debug_bounds(selector).expect("rendered target").center();
    // No mouse move / hover entry: exercise the touch disclosure event path.
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}

fn contained(cx: &mut VisualTestContext, area: Bounds<Pixels>, selector: &'static str) {
    let panel = cx.debug_bounds(selector).expect("visible panel");
    assert!(
        panel.left() >= area.left() && panel.top() >= area.top(),
        "{panel:?}"
    );
    assert!(
        panel.right() <= area.right() && panel.bottom() <= area.bottom(),
        "{panel:?}"
    );
}

#[gpui::test]
fn touch_tooltip_preserves_primary_and_disclosure_does_not_bubble(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    tap(cx, "primary");
    assert_eq!(view.read_with(cx, |stage, _| stage.primary), 1);
    assert!(cx.debug_bounds("popover-panel").is_none());
    refocus(&view, cx);
    tap(cx, "hover-card-disclosure");
    let area = view.read_with(cx, |stage, _| stage.area.unwrap());
    contained(cx, area, "popover-panel");
    assert_eq!(view.read_with(cx, |stage, _| stage.primary), 1);
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
    restored(&view, cx);
}

#[gpui::test]
fn touch_help_has_one_large_opener_and_outside_restores_focus(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    let target = cx.debug_bounds("help-tooltip-disclosure").unwrap();
    assert_eq!(
        target.size,
        size(px(48.), px(48.)),
        "the disclosure bounds must match the button, not stretch across its parent"
    );
    assert!(cx.debug_bounds("hover-card-disclosure").is_none());
    tap(cx, "help-tooltip-disclosure");
    let area = view.read_with(cx, |stage, _| stage.area.unwrap());
    contained(cx, area, "popover-panel");
    assert_eq!(view.read_with(cx, |stage, _| stage.primary), 0);
    tap(cx, "passive-dismiss");
    assert!(cx.debug_bounds("popover-panel").is_none());
    restored(&view, cx);
    tap(cx, "help-tooltip-disclosure");
    contained(cx, area, "popover-panel");
    tap(cx, "outside-focus");
    assert!(cx.debug_bounds("popover-panel").is_none());
    let outside = view.read_with(cx, |stage, _| stage.outside.clone());
    assert!(
        cx.update(|window, _| outside.is_focused(window)),
        "dismissal must not steal focus from the outside target the user pressed"
    );
    assert_eq!(view.read_with(cx, |stage, _| stage.primary), 0);
}

#[gpui::test]
fn touch_bounds_shrink_no_room_does_not_build_or_take_focus_and_reopen_is_fresh(
    cx: &mut TestAppContext,
) {
    let (view, cx) = fixture(cx, false);
    let builds = view.read_with(cx, |stage, _| stage.builds.clone());
    for help in [false, true] {
        view.update(cx, |stage, cx| {
            stage.help = help;
            cx.notify();
        });
        settle(cx);
        let selector = if help {
            "help-tooltip-disclosure"
        } else {
            "hover-card-disclosure"
        };
        let normal = Bounds::new(point(px(40.), px(80.)), size(px(180.), px(180.)));
        view.update(cx, |stage, cx| {
            stage.area = Some(normal);
            cx.notify();
        });
        settle(cx);
        tap(cx, selector);
        contained(cx, normal, "popover-panel");
        for dimensions in [
            size(px(0.), px(150.)),
            size(px(150.), px(0.)),
            size(px(1.), px(1.)),
            size(px(20.), px(20.)),
        ] {
            let count = builds.get();
            view.update(cx, |stage, cx| {
                stage.area = Some(Bounds::new(normal.origin, dimensions));
                cx.notify();
            });
            settle(cx);
            assert!(cx.debug_bounds("popover-panel").is_none());
            assert_eq!(builds.get(), count);
            restored(&view, cx);
            tap(cx, selector);
            assert!(cx.debug_bounds("popover-panel").is_none());
            assert_eq!(builds.get(), count);
            restored(&view, cx);
        }
        view.update(cx, |stage, cx| {
            stage.area = Some(normal);
            cx.notify();
        });
        settle(cx);
        assert!(cx.debug_bounds("popover-panel").is_none());
        tap(cx, selector);
        contained(cx, normal, "popover-panel");
        cx.simulate_keystrokes("escape");
        settle(cx);
    }
}

#[gpui::test]
fn desktop_delay_bounds_and_touch_transition_cancel_pending_work(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    assert!(cx.debug_bounds("hover-card-disclosure").is_none());
    let at = cx.debug_bounds("primary").unwrap().center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("bounded-tooltip").is_none());
    cx.executor().advance_clock(Duration::from_millis(500));
    settle(cx);
    let area = view.read_with(cx, |stage, _| stage.area.unwrap());
    contained(cx, area, "bounded-tooltip");
    view.update(cx, |stage, cx| {
        stage.area = Some(Bounds::new(area.origin, size(px(0.), px(0.))));
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("bounded-tooltip").is_none());
    cx.simulate_mouse_move(point(px(310.), px(490.)), None, Modifiers::none());
    view.update(cx, |stage, cx| {
        stage.area = None;
        cx.notify();
    });
    settle(cx);
    cx.simulate_mouse_move(at, None, Modifiers::none());
    settle(cx);
    let before = view.read_with(cx, |stage, _| stage.builds.get());
    cx.executor().advance_clock(Duration::from_millis(500));
    settle(cx);
    assert!(
        view.read_with(cx, |stage, _| stage.builds.get()) > before,
        "unbounded desktop still opens after its delay"
    );
    assert!(cx.debug_bounds("bounded-tooltip").is_none());
    cx.simulate_mouse_move(point(px(310.), px(490.)), None, Modifiers::none());
    settle(cx);
    cx.simulate_mouse_move(at, None, Modifiers::none());
    settle(cx);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    settle(cx);
    let count = view.read_with(cx, |stage, _| stage.builds.get());
    cx.executor().advance_clock(Duration::from_millis(500));
    settle(cx);
    assert_eq!(view.read_with(cx, |stage, _| stage.builds.get()), count);
    assert!(cx.debug_bounds("popover-panel").is_none());
}
