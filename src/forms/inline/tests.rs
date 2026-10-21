use super::InlineEdit;
use crate::{primitives::FocusScope, theme::Theme};
use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyUpEvent, Keystroke,
    Modifiers, MouseButton, ParentElement, Pixels, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, point, prelude::*, px, size,
};

struct Stage {
    root: FocusHandle,
    outside: FocusHandle,
    value: SharedString,
    commits: Vec<SharedString>,
    callback: bool,
    reject: bool,
    redirect: bool,
    remove: bool,
    shown: bool,
}
impl Render for Stage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let edit = InlineEdit::new("name", self.value.clone()).when(self.callback, |edit| {
            edit.on_commit(move |text, window, cx| {
                owner.update(cx, |s, cx| {
                    s.commits.push(text.clone());
                    if !s.reject {
                        s.value = text.clone();
                    }
                    if s.redirect {
                        window.focus(&s.outside, cx);
                    }
                    if s.remove {
                        s.shown = false;
                    }
                    cx.notify();
                });
            })
        });
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .relative()
            .child(
                div()
                    .debug_selector(|| "host".into())
                    .w(px(120.))
                    .flex()
                    .flex_col()
                    .when(self.shown, |host| host.child(edit)),
            )
            .child(
                div()
                    .id("outside")
                    .debug_selector(|| "outside".into())
                    .track_focus(&self.outside)
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .size(px(48.)),
            )
    }
}
fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}
fn fixture(cx: &mut TestAppContext, touch: bool) -> (Entity<Stage>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |t| {
            t.touch_mode = touch;
            t.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Stage {
        root: cx.focus_handle(),
        outside: cx.focus_handle(),
        value: "Brief.md".into(),
        commits: Vec::new(),
        callback: true,
        reject: false,
        redirect: false,
        remove: false,
        shown: true,
    });
    cx.simulate_resize(size(px(320.), px(400.)));
    settle(cx);
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    (view, cx)
}
fn tap(name: &'static str, cx: &mut VisualTestContext) {
    let at = cx.debug_bounds(name).expect("rendered hit box").center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(cx);
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
fn focused(cx: &mut VisualTestContext) -> Option<FocusHandle> {
    cx.update(|window, cx| window.focused(cx))
}
fn hit(name: &'static str, host: Bounds<Pixels>, cx: &mut VisualTestContext) {
    let b = cx.debug_bounds(name).expect("actual control bounds");
    assert!(b.size.width >= px(48.) && b.size.height >= px(48.), "{b:?}");
    assert!(
        b.left() >= host.left() && b.right() <= host.right(),
        "{b:?} in {host:?}"
    );
}

#[gpui::test]
fn touch_cancel_keeps_owner_name_and_reopen_selects_it(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    let opener = focused(cx);
    let host = cx.debug_bounds("host").unwrap();
    hit("inline-edit-opener", host, cx);
    tap("inline-edit-opener", cx);
    hit("inline-edit-done", host, cx);
    hit("inline-edit-cancel", host, cx);
    cx.simulate_input("discard.txt");
    let input_focus = focused(cx);
    let at = cx.debug_bounds("inline-edit-cancel").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        focused(cx),
        input_focus,
        "Cancel press must not blur-commit"
    );
    assert!(view.read_with(cx, |s, _| s.commits.is_empty()));
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.value.clone()), "Brief.md");
    assert!(view.read_with(cx, |s, _| s.commits.is_empty()));
    assert_eq!(focused(cx), opener);
    tap("inline-edit-opener", cx);
    cx.simulate_input("Kept.md");
    tap("inline-edit-done", cx);
    assert_eq!(view.read_with(cx, |s, _| s.commits.clone()), ["Kept.md"]);
    assert_eq!(focused(cx), opener);
    assert!(cx.debug_bounds("inline-edit-cancel").is_none());
}

#[gpui::test]
fn touch_done_is_once_after_release_and_preserves_callback_focus(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    view.update(cx, |s, cx| {
        s.redirect = true;
        cx.notify();
    });
    tap("inline-edit-opener", cx);
    cx.simulate_input("Owner.md");
    let at = cx.debug_bounds("inline-edit-done").unwrap().center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(view.read_with(cx, |s, _| s.commits.is_empty()));
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    settle(cx);
    assert_eq!(view.read_with(cx, |s, _| s.commits.clone()), ["Owner.md"]);
    assert_eq!(
        focused(cx),
        Some(view.read_with(cx, |s, _| s.outside.clone()))
    );
}

#[gpui::test]
fn touch_action_release_outside_does_not_finish_but_real_blur_commits(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    tap("inline-edit-opener", cx);
    cx.simulate_input("Outside.md");
    for action in ["inline-edit-done", "inline-edit-cancel"] {
        let at = cx.debug_bounds(action).unwrap().center();
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        settle(cx);
        let away = point(px(250.), px(250.));
        cx.simulate_mouse_move(away, Some(MouseButton::Left), Modifiers::none());
        cx.simulate_mouse_up(away, MouseButton::Left, Modifiers::none());
        settle(cx);
        assert!(view.read_with(cx, |s, _| s.commits.is_empty()));
        assert!(cx.debug_bounds("inline-edit-cancel").is_some());
    }
    tap("outside", cx);
    assert_eq!(view.read_with(cx, |s, _| s.commits.clone()), ["Outside.md"]);
    assert_eq!(
        focused(cx),
        Some(view.read_with(cx, |s, _| s.outside.clone()))
    );
}

#[gpui::test]
fn touch_callback_removal_and_rejection_do_not_persist_draft(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    tap("inline-edit-opener", cx);
    cx.simulate_input("Unheard.md");
    view.update(cx, |s, cx| {
        s.callback = false;
        cx.notify();
    });
    settle(cx);
    tap("inline-edit-done", cx);
    assert!(view.read_with(cx, |s, _| s.commits.is_empty()));
    assert_eq!(view.read_with(cx, |s, _| s.value.clone()), "Brief.md");
    view.update(cx, |s, cx| {
        s.callback = true;
        s.reject = true;
        cx.notify();
    });
    settle(cx);
    tap("inline-edit-opener", cx);
    cx.simulate_input("Rejected.md");
    tap("inline-edit-done", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.commits.clone()),
        ["Rejected.md"]
    );
    assert_eq!(view.read_with(cx, |s, _| s.value.clone()), "Brief.md");
}

#[gpui::test]
fn touch_commit_can_remove_owner_then_reopen_without_old_blur(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    view.update(cx, |s, cx| {
        s.remove = true;
        s.redirect = true;
        cx.notify();
    });
    tap("inline-edit-opener", cx);
    cx.simulate_input("First.md");
    tap("inline-edit-done", cx);
    assert!(cx.debug_bounds("inline-edit-opener").is_none());
    assert_eq!(
        focused(cx),
        Some(view.read_with(cx, |s, _| s.outside.clone()))
    );
    view.update(cx, |s, cx| {
        s.shown = true;
        s.remove = false;
        cx.notify();
    });
    settle(cx);
    tap("inline-edit-opener", cx);
    cx.simulate_input("Second.md");
    tap("inline-edit-done", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.commits.clone()),
        ["First.md", "Second.md"]
    );
}

#[gpui::test]
fn desktop_keeps_enter_release_escape_and_outside_blur(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    let opener = focused(cx);
    press("enter", cx);
    assert!(cx.debug_bounds("inline-edit-done").is_none());
    cx.simulate_input("Keyboard.md");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert!(view.read_with(cx, |s, _| s.commits.is_empty()));
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    settle(cx);
    assert_eq!(focused(cx), opener);
    press("enter", cx);
    cx.simulate_input("Discard.md");
    press("escape", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.commits.clone()),
        ["Keyboard.md"]
    );
    press("enter", cx);
    cx.simulate_input("Blur.md");
    tap("outside", cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.commits.clone()),
        ["Keyboard.md", "Blur.md"]
    );
    assert_eq!(
        focused(cx),
        Some(view.read_with(cx, |s, _| s.outside.clone()))
    );
}

#[gpui::test]
fn touch_shows_the_pencil_without_a_hover(cx: &mut TestAppContext) {
    let (_, cx) = fixture(cx, true);
    let pencil = cx
        .debug_bounds("inline-edit-pencil-touch")
        .expect("the pencil shows in touch mode");
    assert!(
        pencil.size.width >= px(48.) && pencil.size.height >= px(48.),
        "the pencil is {pencil:?}"
    );
}

#[gpui::test]
fn desktop_keeps_the_pencil_for_the_hover(cx: &mut TestAppContext) {
    let (_, cx) = fixture(cx, false);
    assert!(
        cx.debug_bounds("inline-edit-pencil-touch").is_none(),
        "the pencil waits for a hover on the desktop"
    );
}
