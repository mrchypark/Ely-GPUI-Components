use std::time::Duration;

use gpui::{
    AppContext, Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px, size,
};

use super::{CommandBlock, CommandState, Target, Terminal};
use crate::theme::Theme;

/// A phone-sized window, in touch mode, where a command block and a terminal sit.
struct Phone {
    folded: bool,
}

impl Render for Phone {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().flex().flex_col().child(
            CommandBlock::new(
                "cmd",
                "~/ely",
                "cargo build",
                "built in 3s",
                CommandState::Done {
                    code: 0,
                    took: Duration::from_secs(3),
                },
            )
            .on_fold(move |folded, _, cx| {
                view.update(cx, |phone, cx| {
                    phone.folded = folded;
                    cx.notify();
                })
            }),
        )
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn phone(cx: &mut TestAppContext) -> (gpui::Entity<Phone>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(16.));
        Phone { folded: false }
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn the_fold_toggle_keeps_a_finger_sized_target(cx: &mut TestAppContext) {
    let (_, cx) = phone(cx);
    let fold = cx
        .debug_bounds("terminal-command-fold")
        .expect("the fold toggle draws");
    assert!(
        fold.size.width >= px(48.) && fold.size.height >= px(48.),
        "the fold toggle is too small to tap: {fold:?}"
    );
}

#[gpui::test]
fn a_tap_on_the_fold_toggle_folds_the_output(cx: &mut TestAppContext) {
    let (view, cx) = phone(cx);
    let fold = cx
        .debug_bounds("terminal-command-fold")
        .expect("the fold toggle draws");
    let at = fold.center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(view.read_with(cx, |phone, _| phone.folded));
}

/// A terminal showing a link, and the paths it opened.
struct Linked {
    terminal: gpui::Entity<Terminal>,
    opened: Vec<Target>,
}

impl Render for Linked {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let terminal = self.terminal.clone();
        cx.subscribe(&terminal, move |host: &mut Linked, _, event, _| {
            if let super::TerminalEvent::Open(target) = event {
                host.opened.push(target.clone());
            }
        })
        .detach();
        div().size_full().child(terminal)
    }
}

#[gpui::test]
fn a_finger_tap_opens_the_link_under_it(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let terminal = cx.new(|cx| Terminal::replay(b"src/main.rs:46:22\r\n", 30, 4, cx));
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(16.));
        Linked {
            terminal,
            opened: Vec::new(),
        }
    });
    cx.simulate_resize(size(px(390.), px(400.)));
    settle(cx);

    // The link fills the first row of the grid; a finger carries no modifier,
    // so the press itself opens it.
    let at = view.read_with(cx, |host, cx| {
        let terminal = host.terminal.read(cx);
        terminal.origin
    });
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    let opened = view.read_with(cx, |view, _| view.opened.clone());
    assert!(
        opened.iter().any(|target| matches!(
            target,
            Target::Path { path, .. } if path.as_ref() == "src/main.rs"
        )),
        "a tap did not open the link: {opened:?}"
    );
}
