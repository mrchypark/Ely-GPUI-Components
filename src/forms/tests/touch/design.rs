use super::*;
use gpui::AppContext as _;

struct InputDesign {
    group: gpui::Entity<crate::forms::TextInput>,
    password: gpui::Entity<crate::forms::TextInput>,
}

impl Render for InputDesign {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let enabled = Choice::new("a", "Enabled");
        let disabled = Choice::new("b", "Disabled").disabled();
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(
                crate::forms::InputGroup::new(&self.group)
                    .before(crate::forms::InputAddon::text("https://")),
            )
            .child(crate::forms::PasswordInput::new(&self.password))
            .child(
                crate::forms::options::option_row("enabled", &enabled, false, None, cx)
                    .debug_selector(|| "design-option-enabled".into()),
            )
            .child(
                crate::forms::options::option_row("disabled", &disabled, false, None, cx)
                    .debug_selector(|| "design-option-disabled".into()),
            )
    }
}

#[gpui::test]
fn touch_input_group_options_and_password_visibility(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let group = cx.new(|cx| {
            let mut field = crate::forms::TextInput::new(window, cx);
            field.set_disabled(true, cx);
            field
        });
        let password = cx.new(|cx| {
            let mut field = crate::forms::TextInput::new(window, cx).masked();
            field.set_disabled(true, cx);
            field
        });
        InputDesign { group, password }
    });
    cx.update(|window, _| {
        window.resize(gpui::size(px(390.), px(844.)));
        window.set_rem_size(px(8.));
        window.activate_window();
    });
    cx.run_until_parked();
    let (group, password) =
        view.read_with(cx, |view, _| (view.group.clone(), view.password.clone()));
    let bounds = cx.debug_bounds("group-root").unwrap();
    assert!(bounds.size.width >= px(48.) && bounds.size.height >= px(48.));
    let enabled = cx.debug_bounds("design-option-enabled").unwrap();
    let disabled = cx.debug_bounds("design-option-disabled").unwrap();
    assert!(enabled.size.height >= px(48.) && disabled.size.height >= px(48.));
    assert_eq!(enabled.size.height, disabled.size.height);
    cx.simulate_click(bounds.center(), Modifiers::none());
    assert!(!cx.update(|window, cx| group.read(cx).focus().is_focused(window)));
    let eye = cx.debug_bounds("password-visibility").unwrap();
    cx.simulate_click(eye.center(), Modifiers::none());
    assert!(password.read_with(cx, |input, _| input.is_masked()));
    password.update(cx, |input, cx| input.set_disabled(false, cx));
    cx.run_until_parked();
    let eye = cx.debug_bounds("password-visibility").unwrap();
    cx.simulate_click(eye.center(), Modifiers::none());
    assert!(!password.read_with(cx, |input, _| input.is_masked()));
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    cx.run_until_parked();
    assert!(cx.debug_bounds("group-root").unwrap().size.height < px(48.));
}

struct TouchSizing {
    selected: bool,
    value: f64,
}

impl Render for TouchSizing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let values = if self.selected { vec!["a"] } else { vec![] };
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(
                crate::forms::MultiSelect::new("size-multi", [Choice::new("a", "Alpha")])
                    .selected(values),
            )
            .child(
                crate::forms::Stepper::new("size-stepper", self.value).on_change(
                    move |value, _, cx| {
                        view.update(cx, |view, cx| {
                            view.value = value;
                            cx.notify();
                        })
                    },
                ),
            )
    }
}

#[gpui::test]
fn touch_multi_and_stepper_keep_their_full_hit_bounds(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        TouchSizing {
            selected: false,
            value: 0.,
        }
    });
    cx.update(|window, _| {
        window.resize(gpui::size(px(390.), px(844.)));
        window.activate_window();
    });
    cx.run_until_parked();
    for selected in [false, true] {
        view.update(cx, |view, cx| {
            view.selected = selected;
            cx.notify();
        });
        cx.run_until_parked();
        let multi = cx.debug_bounds("multi-select-root").unwrap();
        assert!(
            multi.size.height >= px(48.),
            "selected={selected}: {multi:?}"
        );
    }
    let frame = cx.debug_bounds("stepper-root").unwrap();
    let less = cx.debug_bounds("stepper-less").unwrap();
    let more = cx.debug_bounds("stepper-more").unwrap();
    for button in [less, more] {
        assert!(button.size.height >= px(48.));
        assert!(
            button.top() >= frame.top() && button.bottom() <= frame.bottom(),
            "{button:?} outside {frame:?}"
        );
    }
    cx.simulate_click(more.center(), Modifiers::none());
    assert_eq!(view.read_with(cx, |view, _| view.value), 1.);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    cx.run_until_parked();
    assert!(cx.debug_bounds("stepper-root").unwrap().size.height < px(48.));
}
