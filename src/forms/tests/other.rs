use gpui::{
    Context, IntoElement, KeyBinding, KeyUpEvent, Keystroke, Modifiers, MouseButton, ParentElement,
    Render, SharedString, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::setup;
use crate::{
    forms::{Checkbox, Choice, IconPicker, InlineEdit, RadioGroup, SignaturePad, Stroke, Switch},
    primitives::{FocusNext, IconName},
};

struct Icons {
    picked: Option<IconName>,
}

impl Render for Icons {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        IconPicker::new("icons").on_change(move |icon, _, cx| {
            view.update(cx, |view, cx| {
                view.picked = Some(icon);
                cx.notify();
            })
        })
    }
}

#[gpui::test]
fn an_icon_is_searched_walked_and_picked_from_the_keyboard(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Icons { picked: None });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_input("arrow down");
    cx.simulate_keystrokes("down right enter");
    let wanted: Vec<IconName> = IconName::ALL
        .iter()
        .copied()
        .filter(|icon| icon.name().replace('-', " ").contains("arrow down"))
        .collect();
    assert!(wanted.len() > 1, "{wanted:?}");
    assert_eq!(view.read_with(cx, |view, _| view.picked), Some(wanted[1]));
}

struct Pad {
    strokes: Vec<Stroke>,
}

impl Render for Pad {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .size_full()
            .child(
                SignaturePad::new("pad", self.strokes.clone()).on_change(move |next, _, cx| {
                    view.update(cx, |view, cx| {
                        view.strokes = next.to_vec();
                        cx.notify();
                    })
                }),
            )
    }
}

#[gpui::test]
fn a_drag_on_the_pad_lands_as_one_stroke(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Pad {
        strokes: Vec::new(),
    });
    let none = Modifiers::none();
    cx.simulate_mouse_down(point(px(40.0), px(80.0)), MouseButton::Left, none);
    for (x, y) in [(60.0, 70.0), (90.0, 100.0), (120.0, 90.0)] {
        cx.simulate_mouse_move(point(px(x), px(y)), MouseButton::Left, none);
    }
    cx.simulate_mouse_up(point(px(120.0), px(90.0)), MouseButton::Left, none);
    let strokes = view.read_with(cx, |view, _| view.strokes.clone());
    assert_eq!(strokes.len(), 1, "{strokes:?}");
    let stroke = &strokes[0];
    assert!(stroke.len() >= 2, "{stroke:?}");
    let moved = stroke[stroke.len() - 1] - stroke[0];
    assert_eq!(moved, point(px(80.0), px(10.0)), "{stroke:?}");
}

#[gpui::test]
fn one_move_keeps_both_ends_of_the_stroke(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Pad {
        strokes: Vec::new(),
    });
    let none = Modifiers::none();
    cx.simulate_mouse_down(point(px(40.0), px(80.0)), MouseButton::Left, none);
    cx.simulate_mouse_move(point(px(100.0), px(60.0)), MouseButton::Left, none);
    cx.simulate_mouse_up(point(px(100.0), px(60.0)), MouseButton::Left, none);
    let strokes = view.read_with(cx, |view, _| view.strokes.clone());
    assert_eq!(strokes.len(), 1, "{strokes:?}");
    let stroke = &strokes[0];
    assert_eq!(stroke.len(), 2, "{stroke:?}");
    assert_eq!(stroke[1] - stroke[0], point(px(60.0), px(-20.0)));
}

/// An inline edit and the texts it kept.
struct Titled(Vec<SharedString>);

impl Render for Titled {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let value = self.0.last().cloned().unwrap_or_else(|| "Draft".into());
        InlineEdit::new("title", value).on_commit(move |text, _, cx| {
            view.update(cx, |view, cx| {
                view.0.push(text.clone());
                cx.notify();
            })
        })
    }
}

/// Presses and releases `key`, a drawn frame between.
fn release(key: &str, cx: &mut VisualTestContext) {
    let redraw = |cx: &mut VisualTestContext| {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    };
    cx.simulate_keystrokes(key);
    redraw(cx);
    let os = cx.update(|_, cx| cx.platform_info().os);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse_with_os(key, os).expect("a key"),
    });
    redraw(cx);
}

#[gpui::test]
fn an_inline_edit_takes_tab_opens_on_enter_and_takes_focus_back(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]));
    let (view, cx) = cx.add_window_view(|_, _| Titled(Vec::new()));
    cx.update(|window, _| window.activate_window());
    cx.update(|window, cx| window.focus_next(cx));
    release("enter", cx);
    cx.simulate_input("Stairs");
    release("enter", cx);
    release("enter", cx);
    cx.simulate_input("Light");
    release("enter", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.0.clone()),
        ["Stairs", "Light"],
        "focus came back after the first"
    );
}

/// A checkbox, a radio and a switch whose labels run past their narrow box.
struct Worded;

impl Render for Worded {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let label = "Projects/Atrium/Recordings/Walkthrough of the atrium at noon.mov";
        div()
            .w(px(200.0))
            .child(Checkbox::new("worded", false).label(label))
            .child(RadioGroup::new(
                "worded-radios",
                [Choice::new("one", label)],
            ))
            .child(Switch::new("worded-switch", false).label(label))
    }
}

#[gpui::test]
fn long_labels_wrap_inside_their_box(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Worded);
    cx.run_until_parked();
    for selector in ["checkbox-label", "radio-label", "switch-label"] {
        let label = cx.debug_bounds(selector).expect("the label draws");
        assert!(label.right() <= px(200.0), "{selector}: {label:?}");
    }
}

struct PairFixture {
    changes: Vec<Vec<(String, String)>>,
    disabled: bool,
    all_disabled: bool,
}

impl Render for PairFixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let rows = vec![("FIRST".into(), "1".into()), ("READ_ONLY".into(), "locked".into()), ("LAST".into(), "3".into())];
        crate::forms::KeyValueInput::new("pairs", rows)
            .disabled_rows(if self.disabled { vec![1] } else { vec![] })
            .disabled(self.all_disabled)
            .on_change(move |pairs, _, cx| view.update(cx, |view, cx| {
                view.changes.push(pairs);
                cx.notify();
            }))
    }
}

#[gpui::test]
fn pairs_disabled_row_survives_shifts_and_control_toggle(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| PairFixture { changes: Vec::new(), disabled: true, all_disabled: true });
    cx.update(|window, _| window.resize(gpui::size(px(390.), px(844.))));
    cx.run_until_parked();
    let click = |cx: &mut VisualTestContext, selector: &'static str| {
        let bounds = cx.debug_bounds(selector).unwrap_or_else(|| panic!("missing {selector}"));
        cx.simulate_click(bounds.center(), Modifiers::none());
        cx.run_until_parked();
    };
    click(cx, "pairs-add");
    click(cx, "pairs-remove-0");
    click(cx, "pairs-field-0-1");
    cx.simulate_input("BAD");
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.changes.is_empty()));
    view.update(cx, |view, cx| { view.all_disabled = false; cx.notify(); });
    cx.run_until_parked();
    click(cx, "pairs-remove-1");
    click(cx, "pairs-field-1-0");
    cx.simulate_input("BAD");
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.changes.is_empty()));
    click(cx, "pairs-remove-0");
    let last = view.read_with(cx, |view, _| view.changes.last().cloned().unwrap());
    assert_eq!(last, [("READ_ONLY".into(), "locked".into()), ("LAST".into(), "3".into())]);
    click(cx, "pairs-remove-0");
    click(cx, "pairs-field-0-1");
    cx.simulate_input("BAD");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.changes.len()), 1);
    click(cx, "pairs-field-1-1");
    cx.simulate_input("x");
    cx.run_until_parked();
    let changed = view.read_with(cx, |view, _| view.changes.last().cloned().unwrap());
    assert_eq!(changed[0], ("READ_ONLY".into(), "locked".into()));
    assert!(changed[1].1.contains('x'));
    view.update(cx, |view, cx| { view.disabled = false; cx.notify(); });
    cx.run_until_parked();
    click(cx, "pairs-remove-0");
    assert_eq!(view.read_with(cx, |view, _| view.changes.last().unwrap().len()), 1);
    click(cx, "pairs-add");
    assert_eq!(view.read_with(cx, |view, _| view.changes.last().unwrap().len()), 2);
}

struct ListFixture(Vec<Vec<String>>);

impl Render for ListFixture {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        crate::forms::ListInput::new("list", vec!["one".to_string(), "two".to_string()])
            .on_change(move |values, _, cx| view.update(cx, |view, cx| {
                view.0.push(values);
                cx.notify();
            }))
    }
}

#[gpui::test]
fn pairs_list_input_still_removes_and_adds(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| ListFixture(Vec::new()));
    cx.run_until_parked();
    let remove = cx.debug_bounds("pairs-remove-0").unwrap();
    cx.simulate_click(remove.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.0.last().cloned().unwrap()), ["two"]);
    let add = cx.debug_bounds("pairs-add").unwrap();
    cx.simulate_click(add.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.0.last().cloned().unwrap()), ["two", ""]);
}
