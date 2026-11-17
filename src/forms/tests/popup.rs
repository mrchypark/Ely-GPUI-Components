use gpui::{
    Context, Entity, IntoElement, KeyUpEvent, Keystroke, Modifiers, ParentElement, Render,
    ScrollDelta, ScrollEvent, ScrollHandle, Styled, TestAppContext, TouchPhase, VisualTestContext,
    Window, div, point, prelude::*, px,
};

use super::setup;
use crate::forms::{Choice, Combobox, Select, TextInput};
use crate::theme::{ActiveTheme, ControlSize};

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

/// A long select at the top of a page that scrolls.
struct Page(ScrollHandle);

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let choices = (0..40).map(|ix| Choice::new(format!("{ix}"), format!("Choice {ix}")));
        div()
            .id("page")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.0)
            .child(div().w(px(200.0)).child(Select::new("long", choices)))
            .child(div().h(px(2000.0)))
    }
}

#[gpui::test]
fn a_wheel_over_an_open_list_leaves_the_page(cx: &mut TestAppContext) {
    setup(cx);
    let page = ScrollHandle::new();
    let (_, cx) = cx.add_window_view({
        let page = page.clone();
        |_, _| Page(page)
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    cx.run_until_parked();
    let list = cx.debug_bounds("option-list").expect("the list opens");
    cx.simulate_event(ScrollEvent {
        source: gpui::ScrollSource::Pointer,
        position: list.center(),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    cx.run_until_parked();
    assert_eq!(page.offset().y, px(0.0), "the page stayed");
}

struct LowTouchSelect;

impl Render for LowTouchSelect {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .pt(px(650.))
            .child(div().w(px(200.)).child(Select::new(
                "low",
                [
                    Choice::new("a", "A"),
                    Choice::new("b", "B"),
                    Choice::new("c", "C"),
                ],
            )))
    }
}

#[gpui::test]
fn touch_popup_opens_above_when_touch_rows_do_not_fit_below(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| {
        crate::theme::Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        })
    });
    let (_, cx) = cx.add_window_view(|_, _| LowTouchSelect);
    cx.update(|window, _| {
        window.resize(gpui::size(px(390.), px(844.)));
        window.activate_window();
    });
    cx.run_until_parked();
    let field = cx.debug_bounds("field-root").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    let popup = cx.debug_bounds("option-list").expect("open select");
    assert!(
        popup.top() < field.top() && popup.bottom() <= field.top() + px(2.),
        "popup {popup:?} must open above {field:?}"
    );
}

struct TallTouchSelect;

impl Render for TallTouchSelect {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let choices = (0..10).map(|ix| Choice::new(format!("{ix}"), format!("Choice {ix}")));
        div()
            .size_full()
            .pt(px(240.))
            .child(div().w(px(240.)).child(Select::new("tall", choices)))
    }
}

#[gpui::test]
fn touch_popup_shrinks_instead_of_snapping_over_its_anchor(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| {
        crate::theme::Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        })
    });
    let (_, cx) = cx.add_window_view(|_, _| TallTouchSelect);
    cx.update(|window, _| {
        window.resize(gpui::size(px(390.), px(360.)));
        window.activate_window();
    });
    cx.run_until_parked();
    let field = cx.debug_bounds("field-root").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    let popup = cx.debug_bounds("option-list").expect("open select");
    assert!(
        popup.bottom() <= field.top() + px(2.),
        "popup {popup:?} must stop above field {field:?}"
    );
}

struct ShortTouchSelect;

impl Render for ShortTouchSelect {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let choices = (0..3).map(|ix| Choice::new(format!("{ix}"), format!("Choice {ix}")));
        div()
            .size_full()
            .pt(px(70.))
            .child(div().w(px(240.)).child(Select::new("short", choices)))
    }
}

#[gpui::test]
fn short_popup_keeps_one_touch_row_above_the_keyboard(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| {
        crate::theme::Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        })
    });
    let (_, cx) = cx.add_window_view(|_, _| ShortTouchSelect);
    cx.update(|window, _| {
        window.resize(gpui::size(px(390.), px(170.)));
        window.activate_window();
    });
    cx.run_until_parked();
    let field = cx.debug_bounds("field-root").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();

    let popup = cx.debug_bounds("option-list").expect("open select");
    let row = cx.update(|window, cx| {
        let theme = cx.theme();
        theme.touch_minimum(
            theme
                .control_height(ControlSize::Md)
                .to_pixels(window.rem_size()),
        )
    });
    assert!(
        popup.size.height >= row,
        "popup {popup:?} must show one touch row"
    );
    assert!(
        popup.bottom() <= px(170.),
        "popup {popup:?} must stay above the keyboard edge"
    );
}

/// An open autocomplete in a page that keyboard reveal can scroll.
struct ScrolledCombobox {
    scroll: ScrollHandle,
    field: Entity<TextInput>,
}

impl Render for ScrolledCombobox {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let choices = (0..30).map(|ix| Choice::new(format!("{ix}"), format!("Choice {ix}")));
        div()
            .id("page")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(div().h(px(650.)))
            .child(
                div()
                    .w(px(240.))
                    .child(Combobox::new("autocomplete", &self.field, choices).free()),
            )
            .child(div().h(px(1200.)))
    }
}

#[gpui::test]
fn open_autocomplete_follows_its_field_after_the_page_scrolls(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| {
        crate::theme::Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        })
    });
    let (view, cx) = cx.add_window_view(|window, cx| ScrolledCombobox {
        scroll: ScrollHandle::new(),
        field: cx.new(|cx| TextInput::new(window, cx)),
    });
    cx.update(|window, _| {
        window.resize(gpui::size(px(390.), px(844.)));
        window.activate_window();
    });
    cx.run_until_parked();

    let field = cx.debug_bounds("input-root").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_some());

    view.update(cx, |view, cx| {
        view.scroll.set_offset(point(px(0.), px(120.)));
        cx.notify();
    });
    for _ in 0..2 {
        cx.update(|window, cx| window.simulate_next_frame(cx));
        draw(cx);
    }

    let field = cx.debug_bounds("input-root").unwrap();
    let popup = cx.debug_bounds("option-list").unwrap();
    assert!(
        popup.bottom() <= field.top() + px(2.) || popup.top() >= field.bottom() - px(2.),
        "popup {popup:?} must follow scrolled field {field:?}"
    );
}
