use ely_gpui_component::{
    editor::FindOptions,
    layout::PaneLayout,
    theme::{ActiveTheme, ControlSize, Theme},
};
use gpui::{
    App, Axis, Bounds, Context, Entity, IntoElement, KeyUpEvent, Keystroke, Modifiers, MouseButton,
    ParentElement, Pixels, Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext,
    TouchPhase, VisualTestContext, Window, div, point, px, size,
};

use super::live::{Controls, draw};
use crate::ui::set;

struct Replay {
    controls: Option<Controls>,
}

impl Render for Replay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let controls = Controls::new(true, window, cx);
        let element = draw(&controls, cx);
        self.controls = Some(controls);
        div().w_full().child(element)
    }
}

fn redraw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn setup(cx: &mut TestAppContext) -> (Entity<Replay>, &mut VisualTestContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, _| Replay { controls: None });
    cx.simulate_resize(size(px(390.), px(844.)));
    redraw(cx);
    (view, cx)
}

fn state<T>(
    view: &Entity<Replay>,
    cx: &VisualTestContext,
    read: impl FnOnce(&Controls, &App) -> T,
) -> T {
    view.read_with(cx, |view, cx| {
        read(view.controls.as_ref().expect("rendered controls"), cx)
    })
}

fn visible(selector: &'static str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let bounds = cx.debug_bounds(selector).expect(selector);
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(
        bounds.size.width > px(0.) && bounds.size.height > px(0.),
        "{selector}"
    );
    assert!(
        bounds.left() >= px(0.) && bounds.right() <= viewport.width,
        "{selector}: {bounds:?}"
    );
    assert!(
        bounds.top() >= px(0.) && bounds.bottom() <= viewport.height,
        "{selector}: {bounds:?}"
    );
    bounds
}

fn click(at: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    redraw(cx);
}

fn tap(at: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    redraw(cx);
}

// The toolbar lays out split, clear, stop after its title, with gap_2 and px_2.
fn toolbar_action(from_right: usize, cx: &mut VisualTestContext) {
    let bounds = visible("terminal-replay-toolbar", cx);
    let (side, gap) = cx.update(|window, cx| {
        let theme = cx.theme();
        (
            theme.touch_minimum(
                theme
                    .control_height(ControlSize::Sm)
                    .to_pixels(window.rem_size()),
            ),
            window.rem_size() / 2.,
        )
    });
    click(
        point(
            bounds.right() - gap - side / 2. - (side + gap) * from_right as f32,
            bounds.center().y,
        ),
        cx,
    );
}

fn tab(name: &'static str, cx: &mut VisualTestContext) {
    let selector = if name == "build" {
        "editor-close-build"
    } else {
        "editor-close-shell"
    };
    let close = visible(selector, cx);
    click(point(close.left() - px(8.), close.center().y), cx);
}

#[gpui::test]
fn replay_phone_controls_split_clear_stop_and_reload_without_a_shell(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    for selector in [
        "terminal-composition",
        "terminal-replay-tabs",
        "terminal-replay-selector",
        "terminal-replay-toolbar",
        "terminal-replay-grid",
        "terminal-replay-target-0",
        "terminal-replay-target-1",
    ] {
        visible(selector, cx);
    }
    let first = state(&view, cx, |state, cx| state.sessions.read(cx)[0].1.clone());
    let before = first.read_with(cx, |terminal, _| terminal.text());
    assert!(before.contains("Recorded bash output"));
    assert!(before.contains("OK ANSI rendering fixture"));
    assert!(!before.contains('\x1b'));
    first.update(cx, |terminal, _| terminal.write(&b"echo unexpected\r"[..]));
    redraw(cx);
    assert_eq!(first.read_with(cx, |terminal, _| terminal.text()), before);

    toolbar_action(2, cx);
    let (group, sessions) = state(&view, cx, |state, cx| {
        (state.group.clone(), state.sessions.read(cx).clone())
    });
    assert_eq!(sessions.len(), 2);
    assert_ne!(sessions[0].1.entity_id(), sessions[1].1.entity_id());
    assert!(matches!(
        group.read_with(cx, |group, _| group.layout()),
        PaneLayout::Split {
            axis: Axis::Vertical,
            ..
        }
    ));
    let untouched = first.read_with(cx, |terminal, _| terminal.text());
    let focused = group.read_with(cx, |group, _| group.focused());
    let second = sessions
        .iter()
        .find(|(id, _)| *id == focused)
        .unwrap()
        .1
        .clone();
    toolbar_action(1, cx);
    assert!(
        second
            .read_with(cx, |terminal, _| terminal.text())
            .is_empty()
    );
    assert_eq!(
        first.read_with(cx, |terminal, _| terminal.text()),
        untouched
    );

    toolbar_action(0, cx);
    assert_eq!(
        state(&view, cx, |state, cx| state.notice.read(cx).to_string()),
        "Recorded output: no process to stop."
    );
    visible("terminal-replay-notice", cx);

    let selector = visible("terminal-replay-selector", cx);
    click(point(selector.left() + px(24.), selector.center().y), cx);
    state(&view, cx, |state, cx| {
        assert_eq!(*state.shell.read(cx), (0, 1));
        assert_eq!(state.group.read(cx).panes().len(), 1);
        assert_ne!(state.sessions.read(cx)[0].1.entity_id(), first.entity_id());
        assert!(
            state.sessions.read(cx)[0]
                .1
                .read(cx)
                .text()
                .contains("Recorded bash output")
        );
    });
    let selector = visible("terminal-replay-selector", cx);
    tap(point(selector.right() - px(24.), selector.center().y), cx);
    let choice = visible("menu-row-zsh replay", cx);
    tap(choice.center(), cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    state(&view, cx, |state, cx| {
        assert_eq!(*state.shell.read(cx), (1, 2));
        assert!(
            state.sessions.read(cx)[0]
                .1
                .read(cx)
                .text()
                .contains("Recorded zsh output")
        );
    });
}

#[gpui::test]
fn replay_selector_keyboard_commits_once_on_enter_release(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let first = state(&view, cx, |state, cx| state.sessions.read(cx)[0].1.clone());
    let selector = visible("terminal-replay-selector", cx);
    tap(point(selector.right() - px(24.), selector.center().y), cx);
    visible("menu-row-zsh replay", cx);
    for key in ["down", "down"] {
        cx.simulate_keystrokes(key);
        redraw(cx);
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse(key).unwrap(),
        });
        redraw(cx);
    }
    // simulate_keystrokes sends key-down only; menus commit on Enter's release.
    cx.simulate_keystrokes("enter");
    redraw(cx);
    assert_eq!(state(&view, cx, |state, cx| *state.shell.read(cx)), (0, 0));
    visible("menu-row-zsh replay", cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    redraw(cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    state(&view, cx, |state, cx| {
        assert_eq!(*state.shell.read(cx), (1, 1));
        assert_eq!(state.group.read(cx).panes().len(), 1);
        assert_ne!(state.sessions.read(cx)[0].1.entity_id(), first.entity_id());
        assert!(
            state.sessions.read(cx)[0]
                .1
                .read(cx)
                .text()
                .contains("Recorded zsh output")
        );
    });
    redraw(cx);
    assert_eq!(state(&view, cx, |state, cx| *state.shell.read(cx)), (1, 1));
}

#[gpui::test]
fn replay_tabs_keep_entities_and_search_reports_errors_at_phone_width(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let first = state(&view, cx, |state, cx| state.sessions.read(cx)[0].1.clone());
    tab("build", cx);
    assert_eq!(
        state(&view, cx, |state, cx| state.tab.read(cx).to_string()),
        "build"
    );
    let (build, query, options) = state(&view, cx, |state, _| {
        (
            state.build.clone(),
            state.query.clone(),
            state.options.clone(),
        )
    });
    assert_eq!(
        build.read_with(cx, |terminal, _| terminal.matches()),
        (2, Some(1))
    );
    let find = visible("terminal-replay-find", cx);
    let before = cx.debug_bounds("terminal-replay-find-content").unwrap();
    cx.simulate_event(ScrollWheelEvent {
        position: find.center(),
        delta: ScrollDelta::Pixels(point(px(-500.), px(0.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    redraw(cx);
    let after = cx.debug_bounds("terminal-replay-find-content").unwrap();
    assert!(
        after.left() < before.left(),
        "fixed-width search controls must scroll on a phone"
    );
    visible("terminal-replay-find", cx);
    let (side, gap) = cx.update(|window, cx| {
        (
            cx.theme().touch_minimum(
                cx.theme()
                    .control_height(ControlSize::Sm)
                    .to_pixels(window.rem_size()),
            ),
            window.rem_size() / 4.,
        )
    });
    let next = point(after.right() - px(1.) - gap - side / 2., after.center().y);
    click(next, cx);
    assert_eq!(
        build.read_with(cx, |terminal, _| terminal.matches()),
        (2, Some(0))
    );
    click(point(next.x - side - gap, next.y), cx);
    assert_eq!(
        build.read_with(cx, |terminal, _| terminal.matches()),
        (2, Some(1))
    );

    query.update(cx, |query, cx| query.set_text("(", cx));
    cx.update(|_, cx| {
        set(
            &options,
            FindOptions {
                regex: true,
                ..FindOptions::default()
            },
            cx,
        )
    });
    redraw(cx);
    assert!(state(&view, cx, |state, cx| state.error.read(cx).is_some()));
    visible("terminal-replay-find", cx);
    query.update(cx, |query, cx| query.set_text("missing", cx));
    redraw(cx);
    assert!(state(&view, cx, |state, cx| state.error.read(cx).is_none()));
    assert_eq!(
        build.read_with(cx, |terminal, _| terminal.matches()),
        (0, None)
    );
    tab("shell", cx);
    assert_eq!(
        state(&view, cx, |state, cx| state.tab.read(cx).to_string()),
        "shell"
    );
    assert_eq!(
        state(&view, cx, |state, cx| state.sessions.read(cx)[0]
            .1
            .entity_id()),
        first.entity_id()
    );
}

#[gpui::test]
fn replay_target_buttons_preview_url_and_path_without_opening_them(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let url = visible("terminal-replay-target-0", cx);
    click(url.center(), cx);
    assert_eq!(
        state(&view, cx, |state, cx| state.notice.read(cx).to_string()),
        "Recorded URL target: https://example.com/replay"
    );
    let path = visible("terminal-replay-target-1", cx);
    click(path.center(), cx);
    assert_eq!(
        state(&view, cx, |state, cx| state.notice.read(cx).to_string()),
        "Recorded path target: src/main.rs, line 47, column 9"
    );
    visible("terminal-replay-notice", cx);
}
