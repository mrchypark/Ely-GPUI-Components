use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler, IntoElement, Modifiers, ParentElement,
    Render, TestAppContext, TextInputConfiguration, VisualTestContext, Window, div,
};

use super::TextInput;
use crate::{
    forms::{PasswordInput, bind_keys},
    theme::Theme,
};

struct Password {
    input: Entity<TextInput>,
    plain: Entity<TextInput>,
}

impl Render for Password {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(PasswordInput::new(&self.input))
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn configuration(input: &Entity<TextInput>, cx: &mut VisualTestContext) -> TextInputConfiguration {
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            EntityInputHandler::text_input_configuration(input, window, cx)
        })
    })
}

#[gpui::test]
fn password_visibility_toggle_keeps_secure_configuration(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(|window, cx| Password {
        input: cx.new(|cx| TextInput::new(window, cx).masked()),
        plain: cx.new(|cx| TextInput::new(window, cx)),
    });
    let (password, plain) = view.read_with(cx, |view, _| (view.input.clone(), view.plain.clone()));
    cx.update(|window, _| window.activate_window());
    draw(cx);

    assert!(configuration(&password, cx).secure);
    assert!(!configuration(&plain, cx).secure);

    let eye = cx
        .debug_bounds("password-visibility")
        .expect("password eye");
    cx.simulate_click(eye.center(), Modifiers::none());
    assert!(!password.read_with(cx, |input, _| input.is_masked()));
    assert!(configuration(&password, cx).secure);

    let eye = cx
        .debug_bounds("password-visibility")
        .expect("password eye");
    cx.simulate_click(eye.center(), Modifiers::none());
    assert!(password.read_with(cx, |input, _| input.is_masked()));
    assert!(configuration(&password, cx).secure);
}
