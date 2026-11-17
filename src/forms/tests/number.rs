use super::*;

struct Stepped {
    value: f64,
}

impl Render for Stepped {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        NumberInput::new("stepped", self.value).on_change(move |value, _, cx| {
            view.update(cx, |view, cx| {
                view.value = value;
                cx.notify();
            })
        })
    }
}

#[gpui::test]
fn stepping_starts_from_the_typed_number(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Stepped { value: 4.0 });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("20");
    cx.simulate_keystrokes("up");
    assert_eq!(view.read_with(cx, |view, _| view.value), 21.0);
}

#[gpui::test]
fn a_typed_number_reaches_the_owner_before_any_blur(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Stepped { value: 100.0 });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("250");
    assert_eq!(
        view.read_with(cx, |view, _| view.value),
        250.0,
        "a press elsewhere reads the typed number"
    );
    cx.simulate_input(".");
    assert_eq!(
        view.read_with(cx, |view, _| view.value),
        250.0,
        "a half-typed number waits"
    );
}

struct Committed {
    values: Vec<f64>,
}

impl Render for Committed {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        NumberInput::new("committed", 4.0)
            .on_commit(move |value, _, cx| view.update(cx, |view, _| view.values.push(value)))
    }
}

#[gpui::test]
fn typing_commits_only_on_enter_and_steps(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Committed { values: Vec::new() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("12");
    assert!(
        view.read_with(cx, |view, _| view.values.is_empty()),
        "typing waits"
    );
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("up");
    assert_eq!(
        view.read_with(cx, |view, _| view.values.clone()),
        [12.0, 13.0]
    );
}

struct Blank {
    values: Vec<f64>,
}

impl Render for Blank {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        NumberInput::new("blank", 0.0)
            .blank("Mixed")
            .on_commit(move |value, _, cx| view.update(cx, |view, _| view.values.push(value)))
    }
}

#[gpui::test]
fn a_blank_field_commits_only_what_is_typed(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Blank { values: Vec::new() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    cx.update(|window, cx| window.focus_next(cx));
    assert!(
        view.read_with(cx, |view, _| view.values.is_empty()),
        "Enter and blur on an empty field commit nothing"
    );
    cx.update(|window, cx| window.focus_prev(cx));
    cx.simulate_input("0");
    cx.simulate_keystrokes("enter");
    assert_eq!(view.read_with(cx, |view, _| view.values.clone()), [0.0]);
}

#[gpui::test]
fn a_blur_commits_the_typed_number(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Committed { values: Vec::new() });
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("12");
    cx.update(|window, cx| window.blur(cx));
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.values.clone()), [12.0]);
}

#[gpui::test]
fn disabled_number_input_ignores_steps_and_focus(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    struct DisabledNumber {
        changes: Vec<f64>,
    }
    impl Render for DisabledNumber {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity();
            NumberInput::new("disabled-num", 5.0)
                .range(0.0, 10.0)
                .disabled(true)
                .on_change(move |v, _, cx| view.update(cx, |view, _| view.changes.push(v)))
        }
    }
    let (view, cx) = cx.add_window_view(|_, _| DisabledNumber {
        changes: Vec::new(),
    });
    cx.run_until_parked();
    let root = cx.debug_bounds("number-root").expect("number-root");
    cx.simulate_click(root.center(), gpui::Modifiers::none());
    cx.simulate_keystrokes("9 up");
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.changes.is_empty()));
    let settled = crate::forms::number::settle(0.15, 0.1, 0.2, 0);
    assert!((0.1..=0.2).contains(&settled));
}
