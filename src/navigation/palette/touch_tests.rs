use super::{Command, CommandPalette, SearchPalette};
use crate::{forms::Choice, primitives::FocusScope, theme::Theme};
use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyUpEvent, Keystroke,
    Modifiers, MouseButton, ParentElement, Pixels, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px, size,
};
use std::{cell::RefCell, rc::Rc};

struct Stage {
    root: FocusHandle,
    before: FocusHandle,
    area: Bounds<Pixels>,
    open: bool,
    keep_open: bool,
    commands: bool,
    events: Vec<String>,
    queries: Rc<RefCell<Vec<String>>>,
}

fn area() -> Bounds<Pixels> {
    Bounds::new(point(px(20.), px(60.)), size(px(280.), px(300.)))
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
        let owner = cx.entity();
        let close = move |_: &mut Window, cx: &mut gpui::App| {
            owner.update(cx, |stage, cx| {
                stage.events.push("close".into());
                stage.open = stage.keep_open;
                cx.notify();
            })
        };
        let owner = cx.entity();
        let pick = move |value: &SharedString, _: &mut Window, cx: &mut gpui::App| {
            owner.update(cx, |stage, _| stage.events.push(format!("pick:{value}")));
        };
        let palette = if self.commands {
            CommandPalette::new("palette", close)
                .group(
                    "Commands",
                    (0..20).map(|ix| Command::new(format!("{ix}"), format!("Item {ix}"))),
                )
                .on_run(pick)
                .usable_bounds(self.area)
                .into_any_element()
        } else {
            let queries = self.queries.clone();
            SearchPalette::new(
                "palette",
                move |query| {
                    queries.borrow_mut().push(query.to_string());
                    vec![(
                        "Results".into(),
                        (0..20)
                            .map(|ix| Choice::new(format!("{ix}"), format!("{query} item {ix}")))
                            .collect(),
                    )]
                },
                close,
            )
            .on_open(pick)
            .usable_bounds(self.area)
            .into_any_element()
        };
        root.child(palette)
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn fixture(cx: &mut TestAppContext, commands: bool) -> (Entity<Stage>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Stage {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        area: area(),
        open: false,
        keep_open: false,
        commands,
        events: Vec::new(),
        queries: Rc::new(RefCell::new(Vec::new())),
    });
    cx.simulate_resize(size(px(320.), px(500.)));
    settle(cx);
    let before = view.read_with(cx, |s, _| s.before.clone());
    cx.update(|window, cx| window.focus(&before, cx));
    (view, cx)
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
    let b = cx.debug_bounds(name).expect("rendered box");
    assert!(
        b.left() >= area.left()
            && b.top() >= area.top()
            && b.right() <= area.right()
            && b.bottom() <= area.bottom(),
        "{b:?} in {area:?}"
    );
}

#[gpui::test]
fn search_touch_click_completes_once_and_cancel_does_not_pick(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    open(&view, cx);
    contained("palette-card", area(), cx);
    let row = cx.debug_bounds("palette-row-0").unwrap();
    let close = cx.debug_bounds("palette-close").unwrap();
    assert!(
        row.size.height >= px(48.) && close.size.width >= px(48.) && close.size.height >= px(48.)
    );
    cx.simulate_mouse_down(row.center(), MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(view.read_with(cx, |s, _| s.events.is_empty()));
    let outside = point(px(310.), px(490.));
    cx.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
    cx.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(view.read_with(cx, |s, _| s.events.is_empty()));
    tap("palette-row-0", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.events.clone()),
        ["close", "pick:0"]
    );
    restored(&view, cx);
}

#[gpui::test]
fn command_touch_scroll_and_keyboard_keep_real_values(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    open(&view, cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.events.clone()),
        ["close", "pick:1"]
    );
    open(&view, cx);
    let row = cx.debug_bounds("palette-row-0").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: row.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-2000.))),
        ..Default::default()
    });
    settle(cx);
    contained("palette-row-19", area(), cx);
    tap("palette-row-19", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.events.clone()),
        ["close", "pick:1", "close", "pick:19"]
    );
}

#[gpui::test]
fn search_live_bounds_preserve_query_caret_and_close_restores_focus(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    open(&view, cx);
    cx.simulate_input("ab");
    press("left", cx);
    let focused = cx.update(|window, cx| window.focused(cx));
    let smaller = Bounds::new(point(px(30.), px(90.)), size(px(260.), px(210.)));
    view.update(cx, |s, cx| {
        s.area = smaller;
        cx.notify();
    });
    settle(cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focused);
    contained("palette-card", smaller, cx);
    cx.simulate_input("X");
    settle(cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.queries.borrow().last().cloned()),
        Some("aXb".into())
    );
    tap("palette-close", cx);
    assert_eq!(view.read_with(cx, |s, _| s.events.clone()), ["close"]);
    restored(&view, cx);
}

#[gpui::test]
fn search_no_room_closes_once_without_focus_or_hidden_enter_and_reopens(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    view.update(cx, |s, _| {
        s.area.size.height = px(0.);
        s.keep_open = true;
    });
    open(&view, cx);
    restored(&view, cx);
    assert!(cx.debug_bounds("palette-card").is_none());
    press("enter", cx);
    view.update(cx, |s, cx| {
        s.area = area();
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("palette-card").is_none());
    assert_eq!(view.read_with(cx, |s, _| s.events.clone()), ["close"]);
    view.update(cx, |s, cx| {
        s.open = false;
        cx.notify();
    });
    settle(cx);
    open(&view, cx);
    cx.simulate_input("retained");
    settle(cx);
    view.update(cx, |s, cx| {
        s.area.size.height = px(0.);
        cx.notify();
    });
    settle(cx);
    restored(&view, cx);
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.events.clone()),
        ["close", "close"]
    );
}

#[gpui::test]
fn search_owner_removal_restores_focus_without_close_callback(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    open(&view, cx);
    view.update(cx, |s, cx| {
        s.open = false;
        cx.notify();
    });
    settle(cx);
    restored(&view, cx);
    assert!(view.read_with(cx, |s, _| s.events.is_empty()));
}
