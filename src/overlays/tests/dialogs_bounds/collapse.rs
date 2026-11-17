use super::*;

fn suppressed(view: &Entity<Stage>, cx: &mut VisualTestContext) {
    draw(cx);
    for selector in ["plain-content", "dialog-message", "prompt-dialog-field"] {
        assert!(cx.debug_bounds(selector).is_none(), "hidden {selector}");
    }
    let before = view.read_with(cx, |stage, _| {
        assert_eq!(stage.events, ["close"]);
        stage.before.clone()
    });
    assert!(cx.update(|window, _| before.is_focused(window)));
    press("enter", cx);
    press("escape", cx);
    assert_eq!(
        view.read_with(cx, |stage, _| stage.events.clone()),
        ["close"]
    );
}

fn edit_reopened_prompt(view: &Entity<Stage>, retained: &str, cx: &mut VisualTestContext) {
    let field = view.read_with(cx, |stage, _| stage.field.clone());
    let focus = field.read_with(cx, |field, _| {
        assert_eq!(field.text(), retained);
        assert_eq!(
            field.selection(),
            0..retained.len(),
            "new mount primes once"
        );
        field.focus().clone()
    });
    assert!(cx.update(|window, _| focus.is_focused(window)));
    press("end", cx);
    field.read_with(cx, |field, _| {
        assert_eq!(field.selection(), retained.len()..retained.len());
    });
    cx.simulate_input("-edited");
    draw(cx);
    let expected = format!("{retained}-edited");
    field.read_with(cx, |field, _| {
        assert_eq!(field.text(), expected);
        assert_eq!(
            field.selection(),
            expected.len()..expected.len(),
            "rerender must keep the insertion caret, not select-all again"
        );
    });
    assert!(cx.update(|window, _| focus.is_focused(window)));
    assert!(view.read_with(cx, |stage, _| stage.events.is_empty()));
    press("enter", cx);
    let submitted = format!("submit {expected}");
    closed(view, &["close", submitted.as_str()], cx);
    press("enter", cx);
    closed(view, &["close", submitted.as_str()], cx);
}

#[gpui::test]
fn initially_collapsed_dialogs_never_take_focus_or_prime_prompt(cx: &mut TestAppContext) {
    for kind in [
        Kind::Plain,
        Kind::Fullscreen,
        Kind::Alert,
        Kind::Confirm,
        Kind::Prompt,
    ] {
        for extent in [
            size(px(0.), px(320.)),
            size(px(280.), px(0.)),
            size(px(1.), px(1.)),
        ] {
            let (view, cx) =
                fixture_with_bounds(cx, kind, Bounds::new(point(px(42.), px(90.)), extent), true);
            let field = view.read_with(cx, |stage, _| stage.field.clone());
            suppressed(&view, cx);
            field.read_with(cx, |field, _| {
                assert_eq!(field.text(), "Draft");
                assert_eq!(field.selection(), 5..5, "hidden Prompt must not select-all");
            });
            view.update(cx, |stage, cx| {
                stage.bounds = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(600.)));
                cx.notify();
            });
            suppressed(&view, cx);
            field.read_with(cx, |field, _| assert_eq!(field.selection(), 5..5));
            if matches!(kind, Kind::Prompt) {
                // Room alone stays suppressed; unmount before reopening this TextInput.
                view.update(cx, |stage, cx| {
                    stage.open = false;
                    cx.notify();
                });
                draw(cx);
                view.update(cx, |stage, cx| {
                    stage.open = true;
                    stage.keep_open = false;
                    stage.events.clear();
                    cx.notify();
                });
                draw(cx);
                content(&view, cx);
                edit_reopened_prompt(&view, "Draft", cx);
            }
        }
    }
}

#[gpui::test]
fn live_collapse_returns_focus_once_and_cannot_submit_until_explicit_reopen(
    cx: &mut TestAppContext,
) {
    for kind in [
        Kind::Plain,
        Kind::Fullscreen,
        Kind::Alert,
        Kind::Confirm,
        Kind::Prompt,
    ] {
        for collapsed in [
            Bounds::new(point(px(42.), px(90.)), size(px(0.), px(320.))),
            Bounds::new(point(px(42.), px(90.)), size(px(280.), px(0.))),
            Bounds::new(point(px(42.), px(90.)), size(px(1.), px(1.))),
            Bounds::new(point(px(500.), px(900.)), size(px(280.), px(320.))),
        ] {
            let (view, cx) = fixture(cx, kind);
            if matches!(kind, Kind::Prompt) {
                cx.simulate_input("notes");
                draw(cx);
            }
            let before = view.read_with(cx, |stage, _| stage.before.clone());
            assert!(!cx.update(|window, _| before.is_focused(window)));
            view.update(cx, |stage, cx| {
                stage.keep_open = true;
                // Positive geometry wholly outside the viewport must also count as no room.
                stage.bounds = collapsed;
                cx.notify();
            });
            suppressed(&view, cx);
            view.update(cx, |stage, cx| {
                stage.bounds = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(600.)));
                cx.notify();
            });
            suppressed(&view, cx);
            view.update(cx, |stage, cx| {
                stage.open = false;
                cx.notify();
            });
            draw(cx);
            view.update(cx, |stage, cx| {
                stage.open = true;
                stage.keep_open = false;
                stage.events.clear();
                cx.notify();
            });
            draw(cx);
            content(&view, cx);
            if matches!(kind, Kind::Prompt) {
                edit_reopened_prompt(&view, "notes", cx);
            } else {
                press("escape", cx);
                closed(&view, &["close"], cx);
            }
        }
    }
}
