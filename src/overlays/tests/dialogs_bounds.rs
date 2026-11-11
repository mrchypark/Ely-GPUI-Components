use super::{click, press, settle};
use crate::{
    buttons::Button,
    forms::{Input, TextInput},
    layout::reveal_when_focused_in_bounds,
    overlays::{AlertDialog, ConfirmDialog, Dialog, PromptDialog},
    primitives::{FocusScope, Severity},
    theme::Theme,
};
use gpui::{
    AppContext, Axis, Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, Render, ScrollHandle, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px, size,
};
#[derive(Clone, Copy)]
enum Kind {
    Plain,
    Fullscreen,
    Alert,
    Confirm,
    Prompt,
    Short,
    ShortForm,
}

struct Stage {
    root: FocusHandle,
    before: FocusHandle,
    field: Entity<TextInput>,
    second: Entity<TextInput>,
    scroll: ScrollHandle,
    kind: Kind,
    bounds: Bounds<Pixels>,
    open: bool,
    events: Vec<String>,
    keep_open: bool,
    behind: usize,
}

impl Render for Stage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().id("before").track_focus(&self.before))
            .child({
                let owner = cx.entity();
                div()
                    .id("dialog-background")
                    .absolute()
                    .inset_0()
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        owner.update(cx, |stage, _| stage.behind += 1);
                    })
            });
        if !self.open {
            return root;
        }
        let owner = cx.entity();
        let close = move |_: &mut Window, cx: &mut gpui::App| {
            owner.update(cx, |stage, cx| {
                stage.open = stage.keep_open;
                stage.events.push("close".into());
                cx.notify();
            });
        };
        let owner = cx.entity();
        let overlay = match self.kind {
            Kind::Plain | Kind::Fullscreen => {
                let dialog = Dialog::new("plain", "Title", close).usable_bounds(self.bounds);
                let dialog = if matches!(self.kind, Kind::Fullscreen) {
                    dialog.fullscreen()
                } else {
                    dialog
                };
                dialog
                    .child(
                        div()
                            .debug_selector(|| "plain-content".into())
                            .child("Body"),
                    )
                    .into_any_element()
            }
            Kind::Alert => AlertDialog::new("alert", Severity::Warning, "Alert", "Look", close)
                .usable_bounds(self.bounds)
                .into_any_element(),
            Kind::Confirm => ConfirmDialog::new("confirm", "Confirm", "Continue?", close)
                .usable_bounds(self.bounds)
                .on_confirm(move |_, cx| {
                    owner.update(cx, |stage, _| stage.events.push("confirm".into()));
                })
                .into_any_element(),
            Kind::Prompt => PromptDialog::new("prompt", "Name", &self.field, close)
                .usable_bounds(self.bounds)
                .check(|text| {
                    if text.contains('/') {
                        Err("No slash".into())
                    } else {
                        Ok(())
                    }
                })
                .on_submit(move |text, _, cx| {
                    owner.update(cx, |stage, _| stage.events.push(format!("submit {text}")));
                })
                .into_any_element(),
            Kind::Short => Dialog::new("short", "Short", close)
                .usable_bounds(self.bounds)
                .child(div().h(px(400.)))
                .action(move |_| {
                    Button::new("short-action", "Act")
                        .debug_selector(|| "short-action".into())
                        .on_click(move |_, _, cx| {
                            owner.update(cx, |stage, _| stage.events.push("action".into()));
                        })
                })
                .into_any_element(),
            Kind::ShortForm => Dialog::new("short-form", "Edit profile", close)
                .detail("How others see you across the workspace.")
                .usable_bounds(self.bounds)
                .body_scroll(&self.scroll)
                .focus_first(self.field.read(cx).focus().clone())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .debug_selector(|| "short-form-field".into())
                                .relative()
                                .child(Input::new(&self.field))
                                .child(reveal_when_focused_in_bounds(
                                    "short-form-first-reveal",
                                    &self.scroll,
                                    &self.field.read(cx).focus().clone(),
                                    Axis::Vertical,
                                    self.bounds,
                                    window,
                                    cx,
                                )),
                        )
                        .child(
                            div()
                                .debug_selector(|| "short-form-second".into())
                                .relative()
                                .child(Input::new(&self.second))
                                .child(reveal_when_focused_in_bounds(
                                    "short-form-second-reveal",
                                    &self.scroll,
                                    &self.second.read(cx).focus().clone(),
                                    Axis::Vertical,
                                    self.bounds,
                                    window,
                                    cx,
                                )),
                        ),
                )
                .action(move |_| {
                    Button::new("short-save", "Save")
                        .debug_selector(|| "short-save".into())
                        .on_click(move |_, _, cx| {
                            owner.update(cx, |stage, _| stage.events.push("save".into()));
                        })
                })
                .into_any_element(),
        };
        root.child(overlay)
    }
}

fn draw(cx: &mut VisualTestContext) {
    settle(cx);
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn fixture(cx: &mut TestAppContext, kind: Kind) -> (Entity<Stage>, &mut VisualTestContext) {
    fixture_with_bounds(
        cx,
        kind,
        Bounds::new(point(px(20.), px(24.)), size(px(350.), px(600.))),
        false,
    )
}

fn fixture_with_bounds(
    cx: &mut TestAppContext,
    kind: Kind,
    bounds: Bounds<Pixels>,
    keep_open: bool,
) -> (Entity<Stage>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, cx| Stage {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        field: cx.new(|cx| TextInput::new(window, cx)),
        second: cx.new(|cx| TextInput::new(window, cx)),
        scroll: ScrollHandle::new(),
        kind,
        bounds,
        open: false,
        events: Vec::new(),
        keep_open,
        behind: 0,
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    let (before, field) =
        view.read_with(cx, |stage, _| (stage.before.clone(), stage.field.clone()));
    field.update(cx, |field, cx| field.set_text("Draft", cx));
    draw(cx);
    cx.update(|window, cx| window.focus(&before, cx));
    view.update(cx, |stage, cx| {
        stage.open = true;
        cx.notify();
    });
    draw(cx);
    (view, cx)
}

fn content(view: &Entity<Stage>, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    let (kind, usable) = view.read_with(cx, |stage, _| (stage.kind, stage.bounds));
    let selector = match kind {
        Kind::Plain | Kind::Fullscreen | Kind::Short | Kind::ShortForm => "plain-content",
        Kind::Alert | Kind::Confirm => "dialog-message",
        Kind::Prompt => "prompt-dialog-field",
    };
    let bounds = cx.debug_bounds(selector).expect("rendered dialog content");
    assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
    assert!(bounds.left() >= usable.left() && bounds.right() <= usable.right());
    assert!(bounds.top() >= usable.top() && bounds.bottom() <= usable.bottom());
    bounds
}

fn shrink(view: &Entity<Stage>, cx: &mut VisualTestContext) {
    let before = content(view, cx);
    view.update(cx, |stage, cx| {
        stage.bounds = Bounds::new(point(px(42.), px(90.)), size(px(280.), px(320.)));
        cx.notify();
    });
    draw(cx);
    let after = content(view, cx);
    assert!(
        after.size.width < before.size.width,
        "render uses new bounds"
    );
    assert_ne!(after.origin, before.origin);
    assert!(view.read_with(cx, |stage, _| stage.open && stage.events.is_empty()));
}

fn closed(view: &Entity<Stage>, expected: &[&str], cx: &mut VisualTestContext) {
    draw(cx);
    let before = view.read_with(cx, |stage, _| {
        assert!(!stage.open);
        assert_eq!(stage.events, expected);
        stage.before.clone()
    });
    assert!(cx.update(|window, _| before.is_focused(window)));
}

#[gpui::test]
fn alert_uses_inset_and_shrunk_bounds_and_remains_held(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, Kind::Alert);
    shrink(&view, cx);
    click(2., 2., cx);
    assert!(view.read_with(cx, |stage, _| stage.open && stage.events.is_empty()));
    press("tab", cx);
    press("enter", cx);
    closed(&view, &["close"], cx);
}

#[gpui::test]
fn confirm_uses_inset_and_shrunk_bounds_and_closes_before_callback(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, Kind::Confirm);
    shrink(&view, cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    closed(&view, &["close", "confirm"], cx);
}

#[gpui::test]
fn prompt_resize_preserves_text_selection_focus_and_validation(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, Kind::Prompt);
    cx.simulate_input("bad/name");
    press("enter", cx);
    assert!(view.read_with(cx, |stage, _| stage.open && stage.events.is_empty()));
    press("secondary-a", cx);
    cx.simulate_input("notes");
    draw(cx);
    shrink(&view, cx);
    let field = view.read_with(cx, |stage, _| stage.field.clone());
    field.read_with(cx, |field, _| assert_eq!(field.text(), "notes"));
    let focus = field.read_with(cx, |field, _| field.focus().clone());
    assert!(cx.update(|window, _| focus.is_focused(window)));
    // Insertion at the retained caret detects accidental re-priming/select-all on resize.
    cx.simulate_input("-kept");
    press("enter", cx);
    closed(&view, &["close", "submit notes-kept"], cx);
}

#[gpui::test]
fn fullscreen_blocks_presses_outside_its_usable_bounds(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, Kind::Fullscreen);
    click(2., 2., cx);
    assert_eq!(view.read_with(cx, |stage, _| stage.behind), 0);
    assert!(view.read_with(cx, |stage, _| stage.open && stage.events.is_empty()));
}

#[gpui::test]
fn a_short_bounded_dialog_keeps_its_action_reachable(cx: &mut TestAppContext) {
    let usable = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(140.)));
    let (view, cx) = fixture_with_bounds(cx, Kind::Short, usable, false);
    let action = cx.debug_bounds("short-action").expect("action row");
    assert!(
        action.top() >= usable.top() && action.bottom() <= usable.bottom(),
        "action is inside usable bounds: {action:?}"
    );
    click(
        f32::from(action.center().x),
        f32::from(action.center().y),
        cx,
    );
    assert_eq!(
        view.read_with(cx, |stage, _| stage.events.clone()),
        ["action"]
    );
}

#[gpui::test]
fn a_short_bounded_form_reveals_each_focused_field_and_action(cx: &mut TestAppContext) {
    let usable = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(126.)));
    let (view, cx) = fixture_with_bounds(cx, Kind::ShortForm, usable, false);
    let field = cx.debug_bounds("short-form-field").expect("focused field");
    let action = cx.debug_bounds("short-save").expect("save action");

    for (name, bounds) in [("field", field), ("action", action)] {
        assert!(
            bounds.top() >= usable.top()
                && bounds.bottom() <= usable.bottom()
                && bounds.left() >= usable.left()
                && bounds.right() <= usable.right(),
            "{name} is visible inside usable bounds: {bounds:?}"
        );
    }
    assert!(field.size.height >= px(48.), "field keeps a touch target");
    assert!(
        field.bottom() <= action.top(),
        "field and action do not overlap"
    );

    let input = view.read_with(cx, |stage, _| stage.field.clone());
    let focus = input.read_with(cx, |input, _| input.focus().clone());
    assert!(cx.update(|window, _| focus.is_focused(window)));
    cx.simulate_input("Ada");
    input.read_with(cx, |input, _| assert_eq!(input.text(), "DraftAda"));

    let second = view.read_with(cx, |stage, _| stage.second.clone());
    let second_focus = second.read_with(cx, |input, _| input.focus().clone());
    cx.update(|window, cx| second_focus.focus(window, cx));
    draw(cx);
    let second_bounds = cx.debug_bounds("short-form-second").expect("second field");
    assert!(
        second_bounds.top() >= usable.top() && second_bounds.bottom() <= usable.bottom(),
        "second field is visible inside usable bounds: {second_bounds:?}"
    );
    assert!(
        second_bounds.bottom() <= action.top(),
        "second field and action do not overlap"
    );
    cx.simulate_input("Lovelace");
    second.read_with(cx, |second, _| assert_eq!(second.text(), "Lovelace"));

    click(
        f32::from(action.center().x),
        f32::from(action.center().y),
        cx,
    );
    assert_eq!(
        view.read_with(cx, |stage, _| stage.events.clone()),
        ["save"]
    );
}

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
                // Restoring room alone stayed suppressed; remove for a full frame
                // before explicitly reopening the same retained TextInput.
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
