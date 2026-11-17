mod select;

use gpui::{
    AppContext as _, Context, Entity, IntoElement, KeyUpEvent, Keystroke, Modifiers, ParentElement,
    Render, SharedString, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::setup;
use crate::{
    forms::{
        Checkbox, Choice, Combobox, MultiSelect, RadioGroup, Rating, Select, Slider, TextInput,
    },
    motion,
    theme::{ActiveTheme, ControlSize, Theme},
};

fn abc(middle_off: bool) -> [Choice; 3] {
    let middle = Choice::new("b", "B");
    [
        Choice::new("a", "A"),
        if middle_off {
            middle.disabled()
        } else {
            middle
        },
        Choice::new("c", "C"),
    ]
}

struct Radios {
    chosen: SharedString,
}

fn release(key: &str, cx: &mut VisualTestContext) {
    let os = cx.update(|_, cx| cx.platform_info().os);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse_with_os(key, os).unwrap(),
    });
}

impl Render for Radios {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        RadioGroup::new("radios", abc(true))
            .selected(self.chosen.clone())
            .on_change(move |value, _, cx| {
                view.update(cx, |view, cx| {
                    view.chosen = value.clone();
                    cx.notify();
                })
            })
    }
}

#[gpui::test]
fn radio_arrows_skip_disabled_choices_and_wrap(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Radios { chosen: "a".into() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("down");
    assert_eq!(view.read_with(cx, |view, _| view.chosen.clone()), "c");
    cx.simulate_keystrokes("down");
    assert_eq!(view.read_with(cx, |view, _| view.chosen.clone()), "a");
}

struct Picker {
    chosen: SharedString,
}

impl Render for Picker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            Select::new("select", abc(false))
                .selected(self.chosen.clone())
                .on_change(move |value, _, cx| {
                    view.update(cx, |view, cx| {
                        view.chosen = value.clone();
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn select_opens_moves_and_picks_from_the_keyboard(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picker { chosen: "a".into() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("down down enter");
    assert_eq!(view.read_with(cx, |view, _| view.chosen.clone()), "b");
}

/// Which list of thirty rows a test opens.
#[derive(Clone, Copy)]
enum Kind {
    Select,
    Combobox,
    Multi,
}

/// A list of thirty rows with r25 chosen, low in the window, and what each press picked.
struct Long {
    kind: Kind,
    field: Entity<TextInput>,
    picks: Vec<String>,
}

impl Render for Long {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = (0..30).map(|ix| Choice::new(format!("r{ix:02}"), format!("Row {ix:02}")));
        let view = cx.entity();
        let note = move |pick: String, cx: &mut gpui::App| {
            view.update(cx, |view, cx| {
                view.picks.push(pick);
                cx.notify();
            })
        };
        let list = match self.kind {
            Kind::Select => Select::new("long", rows)
                .selected("r25")
                .on_change(move |value, _, cx| note(value.to_string(), cx))
                .into_any_element(),
            Kind::Combobox => Combobox::new("long", &self.field, rows)
                .selected("r25")
                .on_change(move |value, _, cx| note(value.to_string(), cx))
                .into_any_element(),
            Kind::Multi => MultiSelect::new("long", rows)
                .selected([SharedString::from("r25")])
                .on_change(move |next, _, cx| note(next.join(","), cx))
                .into_any_element(),
        };
        div().pt(px(200.0)).w(px(240.0)).child(list)
    }
}

/// Opens the list with a press on its field, then presses the row at its foot.
fn press_the_foot(kind: Kind, cx: &mut TestAppContext) -> Vec<String> {
    setup(cx);
    cx.update(|cx| Theme::update(cx, |theme| theme.reduced_motion = true));
    let (view, cx) = cx.add_window_view(|window, cx| Long {
        kind,
        field: cx.new(|cx| TextInput::new(window, cx)),
        picks: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    let (trigger, list) = cx.update(|window, cx| {
        let theme = cx.theme();
        (
            theme
                .control_height(ControlSize::Md)
                .to_pixels(window.rem_size()),
            theme.list_max_height().to_pixels(window.rem_size()),
        )
    });
    let field = point(px(40.0), px(200.0) + trigger / 2.0);
    cx.simulate_mouse_move(field, None, Modifiers::none());
    cx.simulate_click(field, Modifiers::none());
    frames(cx);
    let foot = point(
        px(40.0),
        px(200.0) + trigger + motion::NUDGE + list - px(8.0),
    );
    cx.simulate_mouse_move(foot, None, Modifiers::none());
    cx.simulate_click(foot, Modifiers::none());
    frames(cx);
    view.read_with(cx, |view, _| view.picks.clone())
}

#[gpui::test]
fn a_long_select_opens_with_its_choice_in_view(cx: &mut TestAppContext) {
    assert_eq!(press_the_foot(Kind::Select, cx), ["r25"]);
}

#[gpui::test]
fn a_long_combobox_opens_with_its_choice_in_view(cx: &mut TestAppContext) {
    assert_eq!(press_the_foot(Kind::Combobox, cx), ["r25"]);
}

#[gpui::test]
fn a_long_multi_select_opens_with_its_first_choice_in_view(cx: &mut TestAppContext) {
    assert_eq!(
        press_the_foot(Kind::Multi, cx),
        [""],
        "the press unticks r25"
    );
}

/// Frames 2ms apart, past reduced motion's 1ms entrances.
fn frames(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

struct Level {
    value: f64,
}

impl Render for Level {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        Slider::new("level", self.value).on_change(move |value, _, cx| {
            view.update(cx, |view, cx| {
                view.value = value;
                cx.notify();
            })
        })
    }
}

#[gpui::test]
fn slider_keys_step_jump_and_reach_the_end(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Level { value: 10.0 });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("right pageup");
    assert_eq!(view.read_with(cx, |view, _| view.value), 21.0);
    cx.simulate_keystrokes("end");
    assert_eq!(view.read_with(cx, |view, _| view.value), 100.0);
}

struct Agree {
    on: bool,
}

impl Render for Agree {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        Checkbox::new("agree", self.on)
            .label("Agree")
            .on_change(move |on, _, cx| {
                view.update(cx, |view, cx| {
                    view.on = on;
                    cx.notify();
                })
            })
    }
}

#[gpui::test]
fn space_toggles_a_focused_checkbox(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Agree { on: false });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("space");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("space").unwrap(),
    });
    assert!(view.read_with(cx, |view, _| view.on));
}

#[gpui::test]
fn a_disabled_choice_still_leaves_the_group_reachable(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Radios { chosen: "b".into() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("down");
    assert_eq!(view.read_with(cx, |view, _| view.chosen.clone()), "c");
}

struct Several {
    chosen: Vec<SharedString>,
}

impl Render for Several {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        MultiSelect::new("several", abc(false))
            .selected(self.chosen.clone())
            .on_change(move |next, _, cx| {
                view.update(cx, |view, cx| {
                    view.chosen = next.to_vec();
                    cx.notify();
                })
            })
    }
}

#[gpui::test]
fn a_full_enter_press_opens_a_multi_select_and_keeps_it_open(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Several { chosen: Vec::new() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    release("enter", cx);
    cx.simulate_keystrokes("enter");
    assert_eq!(view.read_with(cx, |view, _| view.chosen.clone()), ["a"]);
}

struct DisabledMulti {
    chosen: Vec<SharedString>,
    disabled: bool,
    changes: usize,
    next: Entity<TextInput>,
}

impl Render for DisabledMulti {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .flex()
            .flex_col()
            .child(
                MultiSelect::new("disabled-multi", abc(false))
                    .selected(self.chosen.clone())
                    .disabled(self.disabled)
                    .on_change(move |next, _, cx| {
                        view.update(cx, |view, cx| {
                            view.chosen = next.to_vec();
                            view.changes += 1;
                            cx.notify();
                        });
                    }),
            )
            .child(crate::forms::Input::new(&self.next))
    }
}

#[gpui::test]
fn disabling_an_open_multi_select_closes_it_and_blocks_changes(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| DisabledMulti {
        chosen: vec!["a".into()],
        disabled: false,
        changes: 0,
        next: cx.new(|cx| TextInput::new(window, cx)),
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    release("enter", cx);
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_some());

    view.update(cx, |view, cx| {
        view.disabled = true;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_none());
    let chip = cx.debug_bounds("multi-chip-0").expect("selected chip");
    cx.simulate_click(
        point(chip.right() - px(4.), chip.center().y),
        Modifiers::none(),
    );
    let field = cx.debug_bounds("multi-select-root").expect("multi select");
    cx.simulate_click(field.center(), Modifiers::none());
    cx.simulate_keystrokes("backspace enter space down");
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_none());
    view.read_with(cx, |view, _| {
        assert_eq!(view.chosen, ["a"]);
        assert_eq!(view.changes, 0);
    });
    cx.update(|window, cx| {
        window.blur(cx);
        window.focus_next(cx);
        assert!(view.read(cx).next.read(cx).focus().is_focused(window));
    });
    view.update(cx, |view, cx| {
        view.disabled = false;
        cx.notify();
    });
    cx.run_until_parked();
    let chip = cx.debug_bounds("multi-chip-0").expect("enabled chip");
    cx.simulate_click(
        point(chip.right() - px(10.), chip.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.chosen.is_empty());
        assert_eq!(view.changes, 1);
    });
}

struct MultiWithDisabledChoice {
    chosen: Vec<SharedString>,
    changes: usize,
}

impl Render for MultiWithDisabledChoice {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        MultiSelect::new("multi-disabled-choice", abc(true))
            .selected(self.chosen.clone())
            .on_change(move |next, _, cx| {
                view.update(cx, |view, cx| {
                    view.chosen = next.to_vec();
                    view.changes += 1;
                    cx.notify();
                });
            })
    }
}

#[gpui::test]
fn disabled_selected_choice_cannot_be_removed_by_chip_or_backspace(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| MultiWithDisabledChoice {
        chosen: vec!["a".into(), "b".into()],
        changes: 0,
    });
    cx.run_until_parked();
    let disabled_chip = cx
        .debug_bounds("multi-chip-1")
        .expect("disabled selected chip");
    cx.simulate_click(
        point(disabled_chip.right() - px(4.), disabled_chip.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.chosen, ["a", "b"]);
        assert_eq!(view.changes, 0);
    });
    cx.update(|window, cx| {
        window.blur(cx);
        window.focus_next(cx);
    });
    cx.simulate_keystrokes("backspace");
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.chosen, ["b"]);
        assert_eq!(view.changes, 1);
    });
    cx.simulate_keystrokes("backspace");
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.chosen, ["b"]);
        assert_eq!(view.changes, 1);
    });
}
