use std::rc::Rc;

use gpui::{Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window, px};
use jiff::civil::{Date, date};

use crate::{forms::Calendar, theme::Theme};

const TODAY: Date = date(2026, 9, 25);
const PRESSED: Date = date(2026, 9, 10);
const OTHER: Date = date(2026, 9, 11);

/// A calendar that records every preview it reports.
struct Watched {
    seen: Vec<Option<Date>>,
}

impl Render for Watched {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        Calendar::new("watched")
            .selected(TODAY)
            .today(TODAY)
            .on_hover(Rc::new(move |day, _, cx| {
                view.update(cx, |view, cx| {
                    view.seen.push(day);
                    cx.notify();
                })
            }))
    }
}

/// Puts the theme in touch mode, so a day cell grows to a finger's size.
fn touch(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
}

#[gpui::test]
fn a_touch_calendar_day_is_tall_enough_for_a_finger(cx: &mut TestAppContext) {
    touch(cx);
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Watched { seen: Vec::new() }
    });
    cx.run_until_parked();
    let day = cx
        .debug_bounds("calendar-day-2026-09-10")
        .expect("the grid shows its days");
    assert!(day.size.height >= px(48.), "a day cell is {day:?}");
}

#[gpui::test]
fn a_tap_on_a_calendar_day_previews_that_day(cx: &mut TestAppContext) {
    touch(cx);
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Watched { seen: Vec::new() }
    });
    cx.run_until_parked();
    assert!(
        !view.read_with(cx, |view, _| view.seen.iter().any(|day| day.is_some())),
        "no day is previewed before the tap"
    );
    let day = cx
        .debug_bounds("calendar-day-2026-09-10")
        .expect("the grid shows its days");
    cx.simulate_mouse_down(day.center(), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    let seen = view.read_with(cx, |view, _| view.seen.clone());
    assert!(
        seen.contains(&Some(PRESSED)),
        "the tap previews the pressed day"
    );
    assert!(
        !seen.contains(&Some(OTHER)),
        "the day beside it stays unpreviewed"
    );
    assert!(
        seen.iter().flatten().all(|day| *day == PRESSED),
        "only the pressed day is previewed"
    );
}
