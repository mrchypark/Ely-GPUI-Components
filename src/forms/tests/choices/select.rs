use super::*;

struct Typed {
    state: Entity<TextInput>,
    chosen: Option<SharedString>,
}

impl Render for Typed {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let mut combo = Combobox::new(
            "typed",
            &self.state,
            [
                Choice::new("a", "A").disabled(),
                Choice::new("b", "B"),
                Choice::new("c", "C"),
            ],
        )
        .on_change(move |value, _, cx| {
            view.update(cx, |view, cx| {
                view.chosen = Some(value.clone());
                cx.notify();
            })
        });
        if let Some(value) = self.chosen.clone() {
            combo = combo.selected(value);
        }
        combo
    }
}

fn typed<'a>(
    chosen: Option<&str>,
    cx: &'a mut TestAppContext,
) -> (Entity<Typed>, &'a mut VisualTestContext) {
    setup(cx);
    let chosen = chosen.map(|value| SharedString::from(value.to_string()));
    cx.add_window_view(move |window, cx| Typed {
        state: cx.new(|cx| TextInput::new(window, cx)),
        chosen,
    })
}

#[gpui::test]
fn enter_in_a_combobox_skips_a_disabled_first_row(cx: &mut TestAppContext) {
    let (view, cx) = typed(None, cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    assert_eq!(
        view.read_with(cx, |view, _| view.chosen.clone()),
        Some("b".into())
    );
}

#[gpui::test]
fn a_combobox_shows_the_value_its_owner_sets(cx: &mut TestAppContext) {
    let (view, cx) = typed(Some("b"), cx);
    let state = view.read_with(cx, |view, _| view.state.clone());
    assert_eq!(
        state.read_with(cx, |input, _| input.text().to_string()),
        "B"
    );
    view.update(cx, |view, cx| {
        view.chosen = Some("c".into());
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(
        state.read_with(cx, |input, _| input.text().to_string()),
        "C"
    );
}

#[gpui::test]
fn a_combobox_reopens_on_tap_after_selecting_an_option(cx: &mut TestAppContext) {
    let (view, cx) = typed(None, cx);
    cx.update(|_, cx| {
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        })
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let input = cx.debug_bounds("input-root").unwrap().center();
    cx.simulate_click(input, Modifiers::none());
    cx.run_until_parked();
    let choice = cx.debug_bounds("option-row-option-1").expect("choice B");
    cx.simulate_click(choice.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.chosen.clone()),
        Some("b".into())
    );
    assert!(cx.debug_bounds("option-list").is_none());
    cx.simulate_click(input, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("option-list").is_some(),
        "a second tap reopens the choices"
    );
    cx.simulate_click(point(px(380.), px(200.)), Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("option-list").is_none(),
        "outside tap dismisses the choices"
    );
    cx.simulate_click(input, Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("option-list").is_some(),
        "the input can reopen the dismissed list"
    );
}

struct Stars {
    value: u8,
    disabled: bool,
}

impl Render for Stars {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        Rating::new("stars", self.value)
            .disabled(self.disabled)
            .on_change(move |value, _, cx| {
                view.update(cx, |view, cx| {
                    view.value = value;
                    cx.notify();
                })
            })
    }
}

#[gpui::test]
fn a_disabled_rating_ignores_the_arrows(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Stars {
        value: 2,
        disabled: false,
    });
    cx.update(|window, cx| window.focus_next(cx));
    view.update(cx, |view, cx| {
        view.disabled = true;
        cx.notify();
    });
    cx.simulate_keystrokes("right");
    assert_eq!(view.read_with(cx, |view, _| view.value), 2);
}

struct Free {
    state: Entity<TextInput>,
}

impl Render for Free {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Combobox::new("free", &self.state, abc(false)).free()
    }
}

#[gpui::test]
fn a_free_combobox_keeps_the_text_it_mounts_with(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Free {
        state: cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text("custom label", cx);
            input
        }),
    });
    let state = view.read_with(cx, |view, _| view.state.clone());
    assert_eq!(
        state.read_with(cx, |input, _| input.text().to_string()),
        "custom label"
    );
}

/// A select with nothing to pick, open or shut.
struct Empty(bool);

impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            Select::new("empty", [])
                .placeholder("Nothing yet")
                .disabled(self.0),
        )
    }
}

#[gpui::test]
fn a_shut_select_may_be_empty(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Empty(true));
    cx.run_until_parked();
    cx.update(|window, cx| window.focus_next(cx));
    assert!(cx.update(|window, cx| window.focused(cx).is_none()));
}

#[gpui::test]
#[should_panic(expected = "has no choices")]
fn an_open_select_with_nothing_to_pick_fails(cx: &mut TestAppContext) {
    setup(cx);
    cx.add_window_view(|_, _| Empty(false));
}

/// A select whose one choice runs long.
struct Named;

impl Render for Named {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let long = Choice::new("long", "A very long microphone name that runs past the box");
        div()
            .w(px(200.0))
            .child(Select::new("long", [long, Choice::new("b", "B")]))
    }
}

#[gpui::test]
fn a_long_choice_keeps_its_list_inside_the_window(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Named);
    cx.simulate_resize(gpui::size(px(280.0), px(600.0)));
    cx.run_until_parked();
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    release("enter", cx);
    cx.run_until_parked();
    let list = cx.debug_bounds("option-list").expect("the list opens");
    assert!(
        list.left() >= px(0.0) && list.right() <= px(280.0),
        "{list:?}"
    );
}
