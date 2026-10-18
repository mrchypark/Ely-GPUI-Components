use std::{cell::Cell, rc::Rc};

use gpui::{
    AppContext as _, Bounds, Context, ElementInputHandler, Entity, IntoElement, ParentElement,
    PlatformInputHandler, Render, Styled, TestAppContext, VisualTestContext, Window, div,
};

use super::{InputEvent, TextInput};
use crate::{forms::Input, theme::Theme};

struct Fields(Vec<Entity<TextInput>>);

impl Render for Fields {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().children(self.0.iter().map(Input::new))
    }
}

fn open(
    constrained: bool,
    cx: &mut TestAppContext,
) -> (Vec<Entity<TextInput>>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
        super::bind_keys(cx);
    });
    let (root, cx) = cx.add_window_view(|window, cx| {
        Fields(
            (0..2)
                .map(|_| {
                    cx.new(|cx| {
                        let input = TextInput::new(window, cx);
                        if constrained {
                            input.filter(|ch| ch.is_ascii_digit()).max_len(1)
                        } else {
                            input
                        }
                    })
                })
                .collect(),
        )
    });
    let fields = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (fields, cx)
}

fn paint(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn focus(field: &Entity<TextInput>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let focus = field.read(cx).focus.clone();
        window.focus(&focus, cx);
    });
    paint(cx);
}

// TestWindow hides its installed handler; wrap the same painted entity directly.
fn handler(field: &Entity<TextInput>, cx: &mut VisualTestContext) -> PlatformInputHandler {
    cx.update(|window, cx| {
        PlatformInputHandler::new(
            window.to_async(cx),
            Box::new(ElementInputHandler::new(Bounds::default(), field.clone())),
        )
    })
}

fn text(field: &Entity<TextInput>, cx: &mut VisualTestContext) -> String {
    field.read_with(cx, |input, _| input.text().to_owned())
}

#[gpui::test]
fn ime_blur_finalizes_before_reentry_without_duplicate_history(cx: &mut TestAppContext) {
    let (fields, cx) = open(false, cx);
    let (a, b) = (&fields[0], &fields[1]);
    focus(b, cx);
    let mut b_handler = handler(b, cx);
    b_handler.replace_and_mark_text_in_range(None, "나", Some(1..1));
    assert_eq!(b_handler.marked_text_range(), Some(0..1));
    focus(a, cx);
    assert_eq!(text(b, cx), "나");
    assert_eq!(b_handler.marked_text_range(), None);
    assert!(b.read_with(cx, |input, _| input.composing.is_none()));
    let history = b.read_with(cx, |input, _| input.history.held(|_| 1));
    assert!(history > 0);
    b_handler.unmark_text();
    b_handler.unmark_text();
    assert_eq!(
        b.read_with(cx, |input, _| input.history.held(|_| 1)),
        history
    );
    handler(a, cx).replace_text_in_range(None, "A");
    focus(b, cx);
    b_handler.replace_and_mark_text_in_range(None, "x", Some(1..1));
    b_handler.replace_text_in_range(None, "x");
    assert_eq!(text(b, cx), "나x");
    assert_eq!(text(a, cx), "A");
    assert_eq!(b_handler.marked_text_range(), None);
}

#[gpui::test]
fn ime_repaint_preserves_same_field_composition(cx: &mut TestAppContext) {
    let (fields, cx) = open(false, cx);
    let field = &fields[0];
    focus(field, cx);
    let mut input = handler(field, cx);
    input.replace_and_mark_text_in_range(None, "나", Some(1..1));
    paint(cx);
    paint(cx);
    assert_eq!(text(field, cx), "나");
    assert_eq!(input.marked_text_range(), Some(0..1));
    assert!(field.read_with(cx, |input, _| input.composing.is_some()));
    assert!(!field.read_with(cx, |input, _| input.history.can_undo()));
    input.replace_and_mark_text_in_range(None, "난", Some(1..1));
    input.replace_text_in_range(None, "난");
    assert_eq!(text(field, cx), "난");
    assert_eq!(input.marked_text_range(), None);
}

#[gpui::test]
fn ime_disable_finalizes_before_guard_and_reenable_appends(cx: &mut TestAppContext) {
    let (fields, cx) = open(false, cx);
    let field = &fields[1];
    focus(field, cx);
    let mut input = handler(field, cx);
    input.replace_and_mark_text_in_range(None, "나", Some(1..1));
    field.update(cx, |input, cx| input.set_disabled(true, cx));
    assert_eq!(text(field, cx), "나");
    assert_eq!(input.marked_text_range(), None);
    assert!(field.read_with(cx, |input, _| input.composing.is_none()));
    input.replace_text_in_range(None, "ignored");
    input.replace_and_mark_text_in_range(None, "ignored", None);
    assert_eq!(text(field, cx), "나");
    focus(&fields[0], cx);
    field.update(cx, |input, cx| input.set_disabled(false, cx));
    focus(field, cx);
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(field, cx), "");
    cx.simulate_keystrokes("secondary-shift-z");
    assert_eq!(text(field, cx), "나");
    input.replace_and_mark_text_in_range(None, "x", Some(1..1));
    input.replace_text_in_range(None, "x");
    assert_eq!(text(field, cx), "나x");
}

#[gpui::test]
fn ime_disable_obeys_admission_and_repeated_finish_is_noop(cx: &mut TestAppContext) {
    let (fields, cx) = open(true, cx);
    let field = &fields[0];
    let changes = Rc::new(Cell::new(0));
    let counted = changes.clone();
    let _subscription = cx.update(|_, cx| {
        cx.subscribe(field, move |_, event, _| {
            if matches!(event, InputEvent::Changed) {
                counted.set(counted.get() + 1);
            }
        })
    });
    focus(field, cx);
    let mut input = handler(field, cx);
    input.replace_and_mark_text_in_range(None, "a12", None);
    assert_eq!(text(field, cx), "a12");
    field.update(cx, |input, cx| input.set_disabled(true, cx));
    paint(cx);
    assert_eq!(text(field, cx), "1");
    assert_eq!(input.marked_text_range(), None);
    assert_eq!(changes.get(), 1);
    let history = field.read_with(cx, |input, _| input.history.held(|_| 1));
    field.update(cx, |input, cx| input.set_disabled(true, cx));
    input.unmark_text();
    input.unmark_text();
    focus(&fields[1], cx);
    assert_eq!(text(field, cx), "1");
    assert_eq!(changes.get(), 1);
    assert_eq!(
        field.read_with(cx, |input, _| input.history.held(|_| 1)),
        history
    );
}
