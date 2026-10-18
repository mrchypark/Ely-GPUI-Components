use std::{cell::Cell, rc::Rc};

use gpui::{
    AppContext as _, Bounds, Context, ElementInputHandler, Entity, IntoElement, ParentElement,
    PlatformInputHandler, Render, Styled, TestAppContext, VisualTestContext, Window, div,
};

use super::{TextInput, TextInputOwner};
use crate::{forms::Input, theme::Theme};

#[derive(Clone, Default)]
struct Probe {
    active: Rc<Cell<bool>>,
    owner: Rc<Cell<Option<TextInputOwner>>>,
}

impl Probe {
    fn report(&self, owner: TextInputOwner) {
        if self.active.get() {
            assert!(self.owner.replace(Some(owner)).is_none());
        }
    }

    fn query(&self, handler: &mut PlatformInputHandler) -> Option<TextInputOwner> {
        self.owner.set(None);
        self.active.set(true);
        let accepts = handler.query_accepts_text_input();
        self.active.set(false);
        assert!(accepts, "the hook preserves the default acceptance result");
        self.owner.take()
    }
}

struct Fields(Vec<Entity<TextInput>>);

impl Render for Fields {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().children(self.0.iter().map(Input::new))
    }
}

fn open(cx: &mut TestAppContext) -> (Vec<Entity<TextInput>>, Probe, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let probe = Probe::default();
    let observer = probe.clone();
    let (root, cx) = cx.add_window_view(move |window, cx| {
        Fields(
            (0..2)
                .map(|_| {
                    let observer = observer.clone();
                    cx.new(|cx| {
                        TextInput::new(window, cx)
                            .input_owner_observer(move |owner| observer.report(owner))
                    })
                })
                .collect(),
        )
    });
    let fields = root.read_with(cx, |root, _| root.0.clone());
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (fields, probe, cx)
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

// TestWindow's installed handler is private; use the same real entity-backed wrapper.
fn handler(field: &Entity<TextInput>, cx: &mut VisualTestContext) -> PlatformInputHandler {
    cx.update(|window, cx| {
        PlatformInputHandler::new(
            window.to_async(cx),
            Box::new(ElementInputHandler::new(Bounds::default(), field.clone())),
        )
    })
}

#[gpui::test]
fn owner_probe_blur_finalizes_composition_before_reentry(cx: &mut TestAppContext) {
    let (fields, probe, cx) = open(cx);
    let a = &fields[0];
    let b = &fields[1];
    focus(b, cx);
    let mut b_handler = handler(b, cx);
    let before = probe.query(&mut b_handler).expect("focused B owner");
    b_handler.replace_and_mark_text_in_range(None, "나", Some(1..1));
    assert_eq!(b_handler.marked_text_range(), Some(0..1));

    focus(a, cx);
    assert_eq!(b.read_with(cx, |input, _| input.text().to_owned()), "나");
    assert_eq!(b_handler.marked_text_range(), None);
    assert!(b.read_with(cx, |input, _| input.composing.is_none()));
    assert!(b.read_with(cx, |input, _| input.history.can_undo()));
    b_handler.unmark_text();
    assert_eq!(b.read_with(cx, |input, _| input.text().to_owned()), "나");
    let mut a_handler = handler(a, cx);
    a_handler.replace_text_in_range(None, "A");

    focus(b, cx);
    let mut b_handler = handler(b, cx);
    let after = probe.query(&mut b_handler).expect("refocused B owner");
    assert_ne!(before.activation, after.activation);
    b_handler.set_selected_text_range(1..1);
    b_handler.replace_and_mark_text_in_range(None, "x", Some(1..1));
    assert_eq!(b.read_with(cx, |input, _| input.text().to_owned()), "나x");
    b_handler.replace_text_in_range(None, "x");
    assert_eq!(b_handler.marked_text_range(), None);
    assert_eq!(b.read_with(cx, |input, _| input.text().to_owned()), "나x");
    assert_eq!(a.read_with(cx, |input, _| input.text().to_owned()), "A");
}

#[gpui::test]
fn owner_probe_preserves_activation_across_paint_and_composition(cx: &mut TestAppContext) {
    let (fields, probe, cx) = open(cx);
    let field = &fields[0];
    focus(field, cx);
    let mut candidate = handler(field, cx);
    let first = probe
        .query(&mut candidate)
        .expect("real handler reports its entity");
    assert_eq!(first.editor, field.entity_id());
    assert_eq!(
        first.window,
        cx.update(|window, _| window.window_handle().window_id())
    );
    assert!(first.focused && !first.disabled);
    assert!(first.activation > 0);

    paint(cx);
    let mut repainted = handler(field, cx);
    assert_eq!(probe.query(&mut repainted), Some(first));
    candidate.replace_and_mark_text_in_range(None, "한🦀", None);
    paint(cx);
    assert_eq!(
        field.read_with(cx, |input, _| input.text().to_owned()),
        "한🦀"
    );
    assert_eq!(probe.query(&mut candidate), Some(first));
    candidate.replace_text_in_range(None, "한🦀");
    paint(cx);
    assert_eq!(candidate.marked_text_range(), None);
    assert_eq!(probe.query(&mut candidate), Some(first));
    assert_eq!(
        field.read_with(cx, |input, _| input.text().to_owned()),
        "한🦀"
    );
    assert!(
        probe.owner.get().is_none(),
        "paint reports outside the probe are ignored"
    );
}

#[gpui::test]
fn owner_probe_distinguishes_painted_focus_round_trip(cx: &mut TestAppContext) {
    let (fields, probe, cx) = open(cx);
    focus(&fields[0], cx);
    let mut a = handler(&fields[0], cx);
    let before = probe.query(&mut a).unwrap();
    focus(&fields[1], cx);
    let mut b = handler(&fields[1], cx);
    let other = probe.query(&mut b).unwrap();
    let blurred = probe.query(&mut a).unwrap();
    assert_ne!(other.editor, before.editor);
    assert!(other.focused);
    assert!(!blurred.focused);
    assert!(blurred.activation > before.activation);
    focus(&fields[0], cx);
    let returned = probe.query(&mut a).unwrap();
    assert_eq!(returned.editor, before.editor);
    assert!(returned.focused);
    assert!(returned.activation > blurred.activation);
    assert!(!probe.query(&mut b).unwrap().focused);
}

#[gpui::test]
fn owner_probe_tracks_disabled_changes_and_current_focus(cx: &mut TestAppContext) {
    let (fields, probe, cx) = open(cx);
    focus(&fields[0], cx);
    let mut a = handler(&fields[0], cx);
    let initial = probe.query(&mut a).unwrap();
    fields[0].update(cx, |input, cx| input.set_disabled(true, cx));
    paint(cx);
    let disabled = probe.query(&mut a).unwrap();
    assert!(disabled.disabled);
    assert!(disabled.activation > initial.activation);
    fields[0].update(cx, |input, cx| input.set_disabled(true, cx));
    paint(cx);
    assert_eq!(probe.query(&mut a), Some(disabled));
    fields[0].update(cx, |input, cx| input.set_disabled(false, cx));
    paint(cx);
    let enabled = probe.query(&mut a).unwrap();
    assert!(!enabled.disabled);
    assert!(enabled.activation > disabled.activation);

    cx.update(|window, cx| {
        let focus = fields[1].read(cx).focus.clone();
        window.focus(&focus, cx);
        let mut old_handler = ElementInputHandler::new(Bounds::default(), fields[0].clone());
        probe.active.set(true);
        assert!(gpui::InputHandler::accepts_text_input(
            &mut old_handler,
            window,
            cx
        ));
        probe.active.set(false);
        assert!(!probe.owner.take().unwrap().focused);
    });
}

#[gpui::test]
fn owner_probe_is_opt_in_and_default_input_still_commits(cx: &mut TestAppContext) {
    let (_, probe, cx) = open(cx);
    let plain = cx.update(|window, cx| cx.new(|cx| TextInput::new(window, cx)));
    let mut candidate = handler(&plain, cx);
    assert_eq!(probe.query(&mut candidate), None);
    candidate.replace_text_in_range(None, "123");
    assert_eq!(
        plain.read_with(cx, |input, _| input.text().to_owned()),
        "123"
    );
    plain.update(cx, |input, cx| input.set_disabled(true, cx));
    assert_eq!(probe.query(&mut candidate), None);
    candidate.replace_text_in_range(None, "ignored");
    assert_eq!(
        plain.read_with(cx, |input, _| input.text().to_owned()),
        "123"
    );
}

#[gpui::test]
fn owner_probe_empty_commit_clears_real_marked_text(cx: &mut TestAppContext) {
    let (fields, probe, cx) = open(cx);
    let field = &fields[0];
    focus(field, cx);
    let mut candidate = handler(field, cx);
    let owner = probe.query(&mut candidate).expect("real handler identity");
    candidate.replace_and_mark_text_in_range(None, "한", Some(1..1));
    assert_eq!(field.read_with(cx, |input, _| input.text().to_owned()), "한");
    assert!(candidate.marked_text_range().is_some());
    // Companion production-helper test verifies action0 empty reaches this operation.
    candidate.replace_text_in_range(None, "");
    assert_eq!(field.read_with(cx, |input, _| input.text().to_owned()), "");
    assert_eq!(candidate.marked_text_range(), None);
    assert_eq!(probe.query(&mut candidate), Some(owner));
}
