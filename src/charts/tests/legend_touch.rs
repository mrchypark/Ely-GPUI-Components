use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, SharedString,
    TestAppContext, Window, black, div, px,
};

use super::settle;
use crate::charts::ChartLegend;
use crate::theme::Theme;

/// A legend of two entries that hears which one is toggled.
struct Keyed(Rc<RefCell<Vec<SharedString>>>);

impl Render for Keyed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let heard = self.0.clone();
        div().child(
            ChartLegend::new("legend")
                .entry(black(), "Training", false)
                .entry(black(), "Held out", true)
                .on_toggle(move |name, _, _| heard.borrow_mut().push(name.clone())),
        )
    }
}

#[gpui::test]
fn a_legend_entry_keeps_a_touch_target_of_48_pixels(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Keyed(seen));
    cx.simulate_resize(gpui::size(px(400.0), px(200.0)));
    settle(cx);

    let entry = cx
        .debug_bounds("chart-legend-entry-0")
        .expect("the legend draws");
    assert!(
        entry.size.height >= px(48.0),
        "an entry is at least 48 tall in touch mode: {:?}",
        entry.size.height
    );
    assert!(
        entry.size.width >= px(48.0),
        "an entry is at least 48 wide in touch mode: {:?}",
        entry.size.width
    );
}

#[gpui::test]
fn a_legend_entry_still_toggles_when_pressed(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Keyed(seen));
    cx.simulate_resize(gpui::size(px(400.0), px(200.0)));
    settle(cx);

    let entry = cx
        .debug_bounds("chart-legend-entry-0")
        .expect("the legend draws");
    let at = entry.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert_eq!(*heard.borrow(), ["Training"], "the entry heard the press");
}

#[gpui::test]
fn a_legend_entry_keeps_its_size_on_a_desktop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = false);
    });
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Keyed(seen));
    cx.simulate_resize(gpui::size(px(400.0), px(200.0)));
    settle(cx);

    let entry = cx
        .debug_bounds("chart-legend-entry-0")
        .expect("the legend draws");
    assert!(
        entry.size.height < px(48.0),
        "a desktop entry stays as small as it was: {:?}",
        entry.size.height
    );
}
