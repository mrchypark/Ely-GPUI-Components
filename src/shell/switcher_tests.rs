use super::{WindowManager, WindowSwitcher};
use crate::{primitives::FocusScope, theme::Theme};
use gpui::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement,
    IntoElement, KeyUpEvent, Keystroke, Modifiers, MouseButton, ParentElement, Pixels, Render,
    Styled, TestAppContext, VisualTestContext, Window, div, point, px, size,
};

struct Blank;
impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct Stage {
    root: FocusHandle,
    before: FocusHandle,
    area: Bounds<Pixels>,
    open: bool,
    keep_open: bool,
    closes: usize,
}
impl Render for Stage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().id("before").track_focus(&self.before));
        if !self.open {
            return root;
        }
        let view = cx.entity();
        root.child(
            WindowSwitcher::new("windows", move |_, cx| {
                view.update(cx, |s, cx| {
                    s.closes += 1;
                    s.open = s.keep_open;
                    cx.notify();
                });
            })
            .usable_bounds(self.area),
        )
    }
}
fn area() -> Bounds<Pixels> {
    Bounds::new(point(px(20.), px(60.)), size(px(280.), px(280.)))
}
fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}
fn fixture(
    cx: &mut TestAppContext,
    count: usize,
) -> (Entity<Stage>, Vec<AnyWindowHandle>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |t| {
            t.touch_mode = true;
            t.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Stage {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        area: area(),
        open: false,
        keep_open: false,
        closes: 0,
    });
    let handles = cx.update(|_, cx| {
        (0..count)
            .map(|ix| {
                WindowManager::open(
                    format!("Actual window {ix}"),
                    size(px(320.), px(400.)),
                    cx,
                    |_, cx| cx.new(|_| Blank),
                )
                .expect("managed test window")
                .into()
            })
            .collect()
    });
    cx.simulate_resize(size(px(320.), px(500.)));
    settle(cx);
    let before = view.read_with(cx, |s, _| s.before.clone());
    cx.update(|window, cx| {
        window.activate_window();
        window.focus(&before, cx);
    });
    (view, handles, cx)
}
fn open(view: &Entity<Stage>, cx: &mut VisualTestContext) {
    view.update(cx, |s, cx| {
        s.open = true;
        cx.notify();
    });
    settle(cx);
}
fn tap(name: &'static str, cx: &mut VisualTestContext) {
    let at = cx.debug_bounds(name).expect("rendered control").center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}
fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    settle(cx);
}
fn restored(view: &Entity<Stage>, cx: &mut VisualTestContext) {
    let before = view.read_with(cx, |s, _| s.before.clone());
    assert!(cx.update(|window, _| before.is_focused(window)));
}
fn contained(name: &'static str, area: Bounds<Pixels>, cx: &mut VisualTestContext) {
    let b = cx.debug_bounds(name).expect("rendered bounds");
    assert!(
        b.left() >= area.left()
            && b.top() >= area.top()
            && b.right() <= area.right()
            && b.bottom() <= area.bottom(),
        "{b:?} in {area:?}"
    );
}

#[gpui::test]
fn touch_switcher_uses_real_handles_and_waits_for_completed_click(cx: &mut TestAppContext) {
    let (view, handles, cx) = fixture(cx, 8);
    assert_eq!(
        cx.update(|_, cx| WindowManager::windows(cx).len()),
        handles.len()
    );
    open(&view, cx);
    contained("window-switcher-panel", area(), cx);
    let row = cx.debug_bounds("window-switcher-row-0").unwrap();
    let close = cx.debug_bounds("window-switcher-close").unwrap();
    assert!(
        row.size.height >= px(48.) && close.size.height >= px(48.) && close.size.width >= px(48.)
    );
    cx.simulate_mouse_down(row.center(), MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 0);
    let outside = point(px(310.), px(490.));
    cx.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 0);
    let list = cx.debug_bounds("window-switcher-list").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: list.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-2000.))),
        ..Default::default()
    });
    settle(cx);
    contained("window-switcher-row-7", list, cx);
    tap("window-switcher-row-7", cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 1);
    assert_eq!(cx.update(|_, cx| cx.active_window()), Some(handles[7]));
    restored(&view, cx);
}

#[gpui::test]
fn switcher_live_bounds_and_enter_release_preserve_selection(cx: &mut TestAppContext) {
    let (view, handles, cx) = fixture(cx, 3);
    open(&view, cx);
    press("down", cx);
    let smaller = Bounds::new(point(px(30.), px(90.)), size(px(260.), px(200.)));
    view.update(cx, |s, cx| {
        s.area = smaller;
        cx.notify();
    });
    settle(cx);
    contained("window-switcher-panel", smaller, cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 0);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 1);
    assert_eq!(cx.update(|_, cx| cx.active_window()), Some(handles[1]));
}

#[gpui::test]
fn switcher_no_room_never_takes_focus_or_replays_and_close_is_explicit(cx: &mut TestAppContext) {
    let (view, _, cx) = fixture(cx, 1);
    view.update(cx, |s, _| {
        s.area.size.height = px(0.);
        s.keep_open = true;
    });
    open(&view, cx);
    restored(&view, cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 1);
    view.update(cx, |s, cx| {
        s.area = area();
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("window-switcher-panel").is_none());
    view.update(cx, |s, cx| {
        s.open = false;
        s.keep_open = false;
        cx.notify();
    });
    settle(cx);
    open(&view, cx);
    tap("window-switcher-close", cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 2);
    restored(&view, cx);
    open(&view, cx);
    view.update(cx, |s, cx| {
        s.area.size.height = px(0.);
        s.keep_open = true;
        cx.notify();
    });
    settle(cx);
    restored(&view, cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 3);
}

#[gpui::test]
fn switcher_empty_manager_and_scrim_only_close(cx: &mut TestAppContext) {
    let (view, handles, cx) = fixture(cx, 0);
    assert!(handles.is_empty());
    open(&view, cx);
    assert!(cx.debug_bounds("window-switcher-row-0").is_none());
    press("enter", cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 0);
    cx.simulate_mouse_down(
        point(px(310.), px(490.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        point(px(310.), px(490.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 1);
}

#[gpui::test]
fn switcher_owner_removal_restores_focus_without_close_callback(cx: &mut TestAppContext) {
    let (view, _, cx) = fixture(cx, 1);
    open(&view, cx);
    view.update(cx, |s, cx| {
        s.open = false;
        cx.notify();
    });
    settle(cx);
    restored(&view, cx);
    assert_eq!(view.read_with(cx, |s, _| s.closes), 0);
}
