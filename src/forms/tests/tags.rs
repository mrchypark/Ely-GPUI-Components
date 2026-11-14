use gpui::{
    Context, Entity, IntoElement, KeyUpEvent, Keystroke, Modifiers, ParentElement, Render,
    SharedString, Styled, TestAppContext, VisualTestContext, Window, div, px,
};

use super::setup;
use crate::forms::{Choice, TagInput};

/// A tag field offering two people, and the tags it asked for.
struct Recipients {
    tags: Vec<SharedString>,
    asked: Vec<Vec<SharedString>>,
}

impl Render for Recipients {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        TagInput::new("to", self.tags.clone())
            .suggestions([
                Choice::new("ana@atrium.studio", "Ana Lima"),
                Choice::new("ben@atrium.studio", "Ben Ito"),
            ])
            .on_change(move |next, _, cx| {
                owner.update(cx, |host, cx| {
                    host.asked.push(next.clone());
                    host.tags = next;
                    cx.notify();
                })
            })
    }
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    settle(cx);
    let os = cx.update(|_, cx| cx.platform_info().os);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse_with_os(key, os).expect("a key"),
    });
    settle(cx);
}

fn typing<'a>(
    words: &str,
    cx: &'a mut TestAppContext,
) -> (Entity<Recipients>, &'a mut VisualTestContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Recipients {
        tags: Vec::new(),
        asked: Vec::new(),
    });
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    settle(cx);
    cx.simulate_input(words);
    settle(cx);
    (host, cx)
}

fn asked(host: &Entity<Recipients>, cx: &mut VisualTestContext) -> Vec<Vec<SharedString>> {
    host.read_with(cx, |host, _| host.asked.clone())
}

#[gpui::test]
fn touch_add_commits_once_and_desktop_has_no_add_button(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (host, cx) = cx.add_window_view(|_, _| Recipients {
        tags: Vec::new(),
        asked: Vec::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    settle(cx);
    cx.simulate_input("beta");
    settle(cx);
    let add = cx.debug_bounds("tag-input-add").expect("touch add action");
    assert!(add.size.width >= px(44.) && add.size.height >= px(44.));
    cx.simulate_click(add.center(), Modifiers::none());
    settle(cx);
    assert_eq!(asked(&host, cx), [vec![SharedString::from("beta")]]);
    cx.simulate_click(add.center(), Modifiers::none());
    settle(cx);
    assert_eq!(asked(&host, cx).len(), 1, "empty draft must not commit");
    cx.simulate_input("beta");
    settle(cx);
    cx.simulate_click(add.center(), Modifiers::none());
    settle(cx);
    assert_eq!(asked(&host, cx).len(), 1, "duplicate must not commit");
    cx.update(|window, cx| {
        crate::theme::Theme::update(cx, |theme| theme.touch_mode = false);
        window.refresh();
    });
    settle(cx);
    assert!(cx.debug_bounds("tag-input-add").is_none());
}

#[gpui::test]
fn enter_adds_the_suggestion_the_words_find(cx: &mut TestAppContext) {
    let (host, cx) = typing("be", cx);
    press("enter", cx);
    assert_eq!(
        asked(&host, cx),
        [vec![SharedString::from("ben@atrium.studio")]]
    );
}

#[gpui::test]
fn down_moves_to_the_next_suggestion(cx: &mut TestAppContext) {
    let (host, cx) = typing("i", cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(
        asked(&host, cx),
        [vec![SharedString::from("ben@atrium.studio")]],
        "Ana Lima and Ben Ito both hold an i; Down reaches Ben"
    );
}

#[gpui::test]
fn words_with_no_suggestion_are_added_as_typed(cx: &mut TestAppContext) {
    let (host, cx) = typing("dev@atrium.studio", cx);
    press("enter", cx);
    assert_eq!(
        asked(&host, cx),
        [vec![SharedString::from("dev@atrium.studio")]]
    );
}

#[gpui::test]
fn escape_sets_the_suggestions_aside(cx: &mut TestAppContext) {
    let (host, cx) = typing("be", cx);
    press("escape", cx);
    press("enter", cx);
    assert_eq!(asked(&host, cx), [vec![SharedString::from("be")]]);
}

/// A narrow field holding one address whose name runs long.
struct Narrow;

impl Render for Narrow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(200.0))
            .child(
                TagInput::new("to", ["ana@atrium.studio"]).suggestions([Choice::new(
                    "ana@atrium.studio",
                    "Anastasia Lima-Fernández de la Torre, of the Atrium studio",
                )]),
            )
    }
}

#[gpui::test]
fn a_long_chip_ends_inside_its_field(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Narrow);
    settle(cx);
    let chip = cx.debug_bounds("tag to-tag-0").expect("the chip draws");
    let field = cx.debug_bounds("tag-input to").expect("the field draws");
    assert!(chip.right() <= field.right(), "{chip:?} inside {field:?}");
}
