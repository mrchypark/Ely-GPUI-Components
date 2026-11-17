use std::cell::RefCell;

use gpui::{
    Context, ElementId, Entity, EntityInputHandler, IntoElement, KeyboardType, Modifiers,
    ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window, div,
};

use super::PinInput;
use crate::{forms::TextInput, theme::Theme};

thread_local! {
    static INPUTS: RefCell<Vec<(ElementId, Entity<TextInput>)>> = const { RefCell::new(Vec::new()) };
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
            .expect("rendered pin")
    })
}

struct Pins;

impl Render for Pins {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .child(PinInput::new("pin-digits", 6))
            .child(PinInput::new("pin-letters", 6).letters())
    }
}

struct OnePin;

impl Render for OnePin {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(PinInput::new("pin-reopen", 6))
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

fn keyboard(input: &Entity<TextInput>, cx: &mut VisualTestContext) -> KeyboardType {
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            EntityInputHandler::text_input_configuration(input, window, cx)
        })
    })
    .keyboard_type
}

#[gpui::test]
fn digits_request_a_number_pad_and_letters_keep_the_default_keyboard(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    INPUTS.with(|inputs| inputs.borrow_mut().clear());
    let (_view, cx) = cx.add_window_view(|_, _| Pins);
    cx.update(|window, _| window.activate_window());
    draw(cx);

    cx.update(|window, cx| window.focus_next(cx));
    draw(cx);
    let digits = input("pin-digits");
    assert_eq!(keyboard(&digits, cx), KeyboardType::NumberPad);
    cx.simulate_input("12a3");
    assert_eq!(
        digits.read_with(cx, |input, _| input.text().to_owned()),
        "123"
    );
    cx.update(|window, cx| window.focus_next(cx));
    draw(cx);
    let letters = input("pin-letters");
    assert_eq!(keyboard(&letters, cx), KeyboardType::Default);
    cx.simulate_input("AB12");
    assert_eq!(
        letters.read_with(cx, |input, _| input.text().to_owned()),
        "AB12"
    );
}

#[gpui::test]
fn tapping_a_focused_pin_box_keeps_focus_for_keyboard_reopen(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    INPUTS.with(|inputs| inputs.borrow_mut().clear());
    let (_view, cx) = cx.add_window_view(|_, _| OnePin);
    cx.update(|window, _| window.activate_window());
    draw(cx);
    cx.update(|window, cx| window.focus_next(cx));
    draw(cx);
    let pin = input("pin-reopen");
    let box_bounds = cx.debug_bounds("pin-cell-0").expect("pin cell");
    cx.update(|window, _| window.dismiss_virtual_keyboard());
    cx.simulate_click(box_bounds.center(), Modifiers::none());
    assert!(cx.update(|window, cx| pin.read(cx).focus().is_focused(window)));
}
