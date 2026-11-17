use gpui::{
    AppContext as _, Bounds, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext,
    Window, div, point, px, size,
};

use super::JsonInput;
use crate::{
    forms::{TextInput, bind_keys},
    layout::reveal_in_bounds,
    theme::Theme,
};

struct JsonFields {
    fields: [Entity<TextInput>; 2],
    scroll: ScrollHandle,
}

impl Render for JsonFields {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .id("json-fields-scroll")
                .h(px(320.))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(div().h(px(260.)))
                .child(JsonInput::new(&self.fields[0]))
                .child(div().h(px(32.)))
                .child(JsonInput::new(&self.fields[1]))
                .child(div().h(px(220.))),
        )
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn open(cx: &mut TestAppContext) -> (Entity<JsonFields>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let valid = cx.new(|cx| TextInput::new(window, cx).multi_line(4, 8));
        let invalid = cx.new(|cx| TextInput::new(window, cx).multi_line(4, 8));
        invalid.update(cx, |input, cx| input.set_text("x", cx));
        JsonFields {
            fields: [valid, invalid],
            scroll: ScrollHandle::new(),
        }
    });
    cx.update(|window, _| window.activate_window());
    for _ in 0..3 {
        draw(cx);
    }
    (view, cx)
}

fn focus_next(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.focus_next(cx));
    for _ in 0..3 {
        draw(cx);
    }
}

fn focused_bounds(
    cx: &mut VisualTestContext,
) -> (Bounds<gpui::Pixels>, Option<Bounds<gpui::Pixels>>) {
    cx.update(|window, _| {
        let (element, caret) = window
            .focused_text_input_geometry()
            .expect("focused JSON input");
        (element.expect("focused JSON envelope"), caret)
    })
}

#[gpui::test]
fn validation_message_is_included_and_scrolled_above_the_keyboard(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    focus_next(cx);
    let valid = focused_bounds(cx).0;
    focus_next(cx);
    let (invalid, _) = focused_bounds(cx);
    assert!(invalid.size.height > valid.size.height);

    let visible = view.read_with(cx, |view, _| view.scroll.bounds());
    assert!(invalid.bottom() > visible.bottom());
    let scroll = view.read_with(cx, |view, _| view.scroll.clone());
    assert!(reveal_in_bounds(&scroll, invalid, visible));
    for _ in 0..3 {
        draw(cx);
    }
    let (moved, _) = focused_bounds(cx);
    assert!(moved.bottom() <= visible.bottom());
}

#[gpui::test]
fn focus_switch_uses_the_new_fields_message_envelope(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    focus_next(cx);
    let (first, _) = focused_bounds(cx);
    let first_expected = view.read_with(cx, |view, cx| {
        view.fields[0]
            .read(cx)
            .reveal_bounds()
            .expect("valid JSON envelope")
    });
    assert_eq!(first, first_expected);

    focus_next(cx);
    let (second, _) = focused_bounds(cx);
    let second_expected = view.read_with(cx, |view, cx| {
        view.fields[1]
            .read(cx)
            .reveal_bounds()
            .expect("invalid JSON envelope")
    });
    assert_eq!(second, second_expected);
    assert!(second.size.height > first.size.height);
}

#[gpui::test]
fn short_viewport_keeps_the_caret_geometry_as_fallback(cx: &mut TestAppContext) {
    let (view, cx) = open(cx);
    focus_next(cx);
    focus_next(cx);
    let (field, caret) = focused_bounds(cx);
    let caret = caret.expect("caret geometry");
    let editor = view.read_with(cx, |view, cx| {
        view.fields[1]
            .read(cx)
            .layout
            .as_ref()
            .expect("laid-out editor")
            .bounds
    });
    let short = Bounds::new(
        point(field.left(), field.top()),
        size(field.size.width, px(80.)),
    );

    assert!(field.size.height > short.size.height);
    assert!(caret.top() >= editor.top() && caret.bottom() <= editor.bottom());
    assert!(caret.size.height <= short.size.height);
}
