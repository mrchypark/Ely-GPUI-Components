use super::{QuickOpen, QuickSwitcher};
use crate::{
    editor::{GoToSymbol, Project, ProjectSwitcher, Symbol, SymbolKind},
    forms::Choice,
    git::{Branch, BranchSelector},
    primitives::FocusScope,
    terminal::{CommandHistory, PastCommand},
    theme::Theme,
};
use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyUpEvent, Keystroke,
    Modifiers, MouseButton, ParentElement, Pixels, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px, size,
};

#[derive(Clone, Copy)]
enum Kind {
    Files,
    Switcher,
    Symbols,
    Projects,
    Branches,
    History,
}

struct Stage {
    root: FocusHandle,
    before: FocusHandle,
    kind: Kind,
    bounds: Bounds<Pixels>,
    open: bool,
    keep_open: bool,
    events: Vec<String>,
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
            owner.update(cx, |s, cx| {
                s.events.push("close".into());
                s.open = s.keep_open;
                cx.notify();
            })
        };
        let owner = cx.entity();
        let pick = move |value: &SharedString, _: &mut Window, cx: &mut gpui::App| {
            owner.update(cx, |s, _| s.events.push(format!("pick:{value}")));
        };
        let names = ["alpha", "beta", "papaya"];
        let bounds = self.bounds;
        let palette = match self.kind {
            Kind::Files => QuickOpen::new("wrapper", names, close)
                .usable_bounds(bounds)
                .on_open(pick)
                .into_any_element(),
            Kind::Switcher => {
                QuickSwitcher::new("wrapper", names.map(|name| Choice::new(name, name)), close)
                    .usable_bounds(bounds)
                    .on_switch(pick)
                    .into_any_element()
            }
            Kind::Symbols => GoToSymbol::new(
                "wrapper",
                names
                    .into_iter()
                    .enumerate()
                    .map(|(ix, name)| Symbol::new(name, SymbolKind::Function, 42 + ix)),
                close,
            )
            .usable_bounds(bounds)
            .on_pick(move |line, window, cx| pick(&line.to_string().into(), window, cx))
            .into_any_element(),
            Kind::Projects => ProjectSwitcher::new(
                "wrapper",
                names.map(|name| Project {
                    name: name.into(),
                    path: name.into(),
                    branch: None,
                    opened: "today".into(),
                    pinned: false,
                }),
                close,
            )
            .usable_bounds(bounds)
            .on_open(move |ix, window, cx| pick(&ix.to_string().into(), window, cx))
            .into_any_element(),
            Kind::Branches => BranchSelector::new(
                "wrapper",
                names.map(|name| Branch {
                    name: name.into(),
                    remote: false,
                    current: false,
                    ahead: 0,
                    behind: 0,
                    subject: "Recorded commit".into(),
                    when: "today".into(),
                }),
                close,
            )
            .usable_bounds(bounds)
            .on_pick(pick)
            .on_create(|_, _, _| panic!("the exact alpha query must pick its branch, not create"))
            .into_any_element(),
            Kind::History => CommandHistory::new(
                "wrapper",
                names.map(|name| PastCommand {
                    command: name.into(),
                    cwd: "/recorded".into(),
                    when: "today".into(),
                    code: Some(0),
                }),
                close,
            )
            .usable_bounds(bounds)
            .on_pick(pick)
            .into_any_element(),
        };
        root.child(palette)
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}
fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).unwrap(),
    });
    settle(cx);
}
fn area() -> Bounds<Pixels> {
    Bounds::new(point(px(20.), px(60.)), size(px(280.), px(300.)))
}
fn fixture(kind: Kind, cx: &mut TestAppContext) -> (Entity<Stage>, &mut VisualTestContext) {
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
        kind,
        bounds: area(),
        open: false,
        keep_open: false,
        events: Vec::new(),
    });
    cx.simulate_resize(size(px(320.), px(500.)));
    settle(cx);
    let before = view.read_with(cx, |s, _| s.before.clone());
    cx.update(|window, cx| window.focus(&before, cx));
    (view, cx)
}
fn restored(view: &Entity<Stage>, cx: &mut VisualTestContext) {
    let before = view.read_with(cx, |s, _| s.before.clone());
    assert!(cx.update(|window, _| before.is_focused(window)));
}
fn check(kind: Kind, expected: &str, cx: &mut TestAppContext) {
    let (view, cx) = fixture(kind, cx);
    view.update(cx, |s, cx| {
        s.open = true;
        cx.notify();
    });
    settle(cx);
    cx.simulate_input("alha");
    press("left", cx);
    press("left", cx);
    let focused = cx.update(|window, cx| window.focused(cx));
    let smaller = Bounds::new(point(px(30.), px(90.)), size(px(260.), px(210.)));
    view.update(cx, |s, cx| {
        s.bounds = smaller;
        cx.notify();
    });
    settle(cx);
    let card = cx.debug_bounds("palette-card").expect("bounded palette");
    assert!(
        card.left() >= smaller.left()
            && card.top() >= smaller.top()
            && card.right() <= smaller.right()
            && card.bottom() <= smaller.bottom(),
        "{card:?} in {smaller:?}"
    );
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focused);
    cx.simulate_input("p"); // Insert at the retained caret: al|ha -> alpha.
    settle(cx);
    assert!(
        cx.debug_bounds("palette-row-1").is_none(),
        "only alpha matches"
    );
    let row = cx.debug_bounds("palette-row-0").expect("alpha result");
    assert!(row.size.height >= px(48.));
    cx.simulate_mouse_down(row.center(), MouseButton::Left, Modifiers::none());
    assert!(view.read_with(cx, |s, _| s.events.is_empty()));
    cx.simulate_mouse_up(row.center(), MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        view.read_with(cx, |s, _| s.events.clone()),
        vec!["close".to_string(), format!("pick:{expected}")]
    );
    restored(&view, cx);

    view.update(cx, |s, cx| {
        s.open = true;
        s.keep_open = true;
        s.bounds.size.height = px(0.);
        cx.notify();
    });
    settle(cx);
    restored(&view, cx);
    assert!(cx.debug_bounds("palette-card").is_none());
    press("enter", cx);
    view.update(cx, |s, cx| {
        s.bounds = area();
        cx.notify();
    });
    settle(cx);
    assert!(
        cx.debug_bounds("palette-card").is_none(),
        "canceled open cannot replay"
    );
    assert_eq!(
        view.read_with(cx, |s, _| s.events.clone()),
        vec![
            "close".to_string(),
            format!("pick:{expected}"),
            "close".into()
        ]
    );
}

#[gpui::test]
fn quick_open_forwards_live_bounds_and_keeps_path(cx: &mut TestAppContext) {
    check(Kind::Files, "alpha", cx);
}
#[gpui::test]
fn quick_switcher_forwards_live_bounds_and_keeps_value(cx: &mut TestAppContext) {
    check(Kind::Switcher, "alpha", cx);
}
#[gpui::test]
fn go_to_symbol_forwards_live_bounds_and_keeps_line(cx: &mut TestAppContext) {
    check(Kind::Symbols, "42", cx);
}
#[gpui::test]
fn project_switcher_forwards_live_bounds_and_keeps_index(cx: &mut TestAppContext) {
    check(Kind::Projects, "0", cx);
}
#[gpui::test]
fn branch_selector_forwards_live_bounds_and_picks_instead_of_create(cx: &mut TestAppContext) {
    check(Kind::Branches, "alpha", cx);
}
#[gpui::test]
fn command_history_forwards_live_bounds_and_keeps_recorded_command(cx: &mut TestAppContext) {
    check(Kind::History, "alpha", cx);
}
