use std::cell::RefCell;

use gpui::{
    Context, ElementId, Entity, EntityInputHandler, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div,
};

use super::{NumberInput, keyboard_hint};
use crate::{forms::TextInput, theme::Theme};

thread_local! {
    static INPUTS: RefCell<Vec<(ElementId, Entity<TextInput>)>> = const { RefCell::new(Vec::new()) };
}

struct Modes;

impl Render for Modes {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(NumberInput::new("hint-unsigned", 2.0).range(0.0, 10.0))
            .child(NumberInput::new("hint-signed", -2.0))
            .child(
                NumberInput::new("hint-decimal", 2.5)
                    .range(0.0, 10.0)
                    .precision(2),
            )
            .child(NumberInput::new("hint-signed-decimal", -2.5).precision(2))
    }
}

pub(super) fn record(id: ElementId, input: Entity<TextInput>) {
    INPUTS.with(|inputs| {
        let mut inputs = inputs.borrow_mut();
        if let Some((_, current)) = inputs.iter_mut().find(|(current, _)| current == &id) {
            *current = input;
        } else {
            inputs.push((id, input));
        }
    });
}

fn input(id: &'static str) -> Entity<TextInput> {
    let id = ElementId::from(id);
    INPUTS.with(|inputs| {
        inputs
            .borrow()
            .iter()
            .find(|(current, _)| current == &id)
            .map(|(_, input)| input.clone())
            .expect("rendered number input")
    })
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn requested_keyboard(input: &Entity<TextInput>, cx: &mut VisualTestContext) -> gpui::KeyboardType {
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            EntityInputHandler::text_input_configuration(input, window, cx)
        })
    })
    .keyboard_type
}

#[test]
fn keyboard_hint_matches_sign_and_precision() {
    assert_eq!(keyboard_hint(0.0, 0), gpui::KeyboardType::NumberPad);
    assert_eq!(keyboard_hint(-1.0, 0), gpui::KeyboardType::SignedNumber);
    assert_eq!(keyboard_hint(0.0, 2), gpui::KeyboardType::Decimal);
    assert_eq!(keyboard_hint(-1.0, 2), gpui::KeyboardType::SignedDecimal);
}

#[gpui::test]
fn focus_changes_request_the_number_fields_hint(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    INPUTS.with(|inputs| inputs.borrow_mut().clear());
    let (_, cx) = cx.add_window_view(|_, _| Modes);
    cx.update(|window, _| window.activate_window());
    draw(cx);

    for (id, expected) in [
        ("hint-unsigned", gpui::KeyboardType::NumberPad),
        ("hint-signed", gpui::KeyboardType::SignedNumber),
        ("hint-decimal", gpui::KeyboardType::Decimal),
        ("hint-signed-decimal", gpui::KeyboardType::SignedDecimal),
    ] {
        cx.update(|window, cx| window.focus_next(cx));
        draw(cx);
        assert_eq!(requested_keyboard(&input(id), cx), expected, "{id}");
    }
}
