use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};

use super::{Input, JsonInput, TextInput};
use crate::theme::Theme;

struct Reused {
    input: Entity<TextInput>,
    json: bool,
}

impl Render for Reused {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let field = if self.json {
            JsonInput::new(&self.input).into_any_element()
        } else {
            Input::new(&self.input).into_any_element()
        };
        div().size_full().child(field)
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn reveal(input: &Entity<TextInput>, cx: &mut VisualTestContext) -> gpui::Bounds<gpui::Pixels> {
    cx.update(|_, cx| input.read(cx).reveal_bounds().expect("reveal bounds"))
}

#[gpui::test]
fn plain_input_replaces_json_reveal_bounds_for_reused_state(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let input = cx.new(|cx| TextInput::new(window, cx).multi_line(3, 6));
        input.update(cx, |input, cx| input.set_text("{", cx));
        Reused { input, json: true }
    });
    let input = view.read_with(cx, |view, _| view.input.clone());
    cx.update(|window, _| window.activate_window());
    for _ in 0..3 {
        draw(cx);
    }
    let json = reveal(&input, cx);

    view.update(cx, |view, cx| {
        view.json = false;
        cx.notify();
    });
    for _ in 0..3 {
        draw(cx);
    }
    let plain = reveal(&input, cx);
    assert!(json.size.height > plain.size.height);
}
