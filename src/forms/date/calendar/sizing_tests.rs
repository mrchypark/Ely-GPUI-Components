use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, Window, div, px, size,
};
use jiff::civil::{Date, date};

use super::Calendar;
use crate::theme::Theme;

struct CalendarHost {
    picked: Option<Date>,
}

impl Render for CalendarHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let host = cx.entity();
        div()
            .id("calendar-size-host")
            .debug_selector(|| "calendar-size-host".to_string())
            .w_full()
            .p_4()
            .child(
                Calendar::new("sized-calendar")
                    .today(date(2026, 9, 10))
                    .selected(date(2026, 9, 10))
                    .on_pick(move |day, _, cx| host.update(cx, |host, _| host.picked = Some(day))),
            )
    }
}

#[gpui::test]
fn calendar_fits_phone_content_and_keeps_days_tappable(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        CalendarHost { picked: None }
    });
    for width in [320., 390.] {
        cx.simulate_resize(size(px(width), px(844.)));
        cx.run_until_parked();
        let host = cx.debug_bounds("calendar-size-host").unwrap();
        let first = cx.debug_bounds("calendar-day-2026-09-07").unwrap();
        let last = cx.debug_bounds("calendar-day-2026-09-13").unwrap();
        assert!(first.left() >= host.left() + px(8.));
        assert!(
            last.right() <= host.right() - px(8.),
            "{width}: {last:?} outside {host:?}"
        );
        assert!(first.size.height >= px(48.) && last.size.height >= px(48.));
        assert!(first.size.width >= px(36.) && last.size.width >= px(36.));
        cx.simulate_click(last.center(), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.picked),
            Some(date(2026, 9, 13))
        );
    }
}

#[gpui::test]
fn desktop_calendar_keeps_its_fixed_width(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = false;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        CalendarHost { picked: None }
    });
    cx.simulate_resize(size(px(600.), px(844.)));
    cx.run_until_parked();
    let wide = cx.debug_bounds("calendar-day-2026-09-10").unwrap();
    cx.simulate_resize(size(px(320.), px(844.)));
    cx.run_until_parked();
    let narrow = cx.debug_bounds("calendar-day-2026-09-10").unwrap();
    assert_eq!(wide.size, narrow.size);
}

struct PickerHost;

impl Render for PickerHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p_4().child(crate::forms::DatePicker::new(
            "sized-picker",
            Some(date(2026, 9, 10)),
        ))
    }
}

#[gpui::test]
fn date_picker_popup_fits_phone_window(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    for width in [320., 390.] {
        let (_, cx) = cx.add_window_view(|window, _| {
            window.set_rem_size(px(8.));
            PickerHost
        });
        cx.simulate_resize(size(px(width), px(844.)));
        cx.update(|window, cx| window.focus_next(cx));
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        let first = cx.debug_bounds("calendar-day-2026-09-07").unwrap();
        let last = cx.debug_bounds("calendar-day-2026-09-13").unwrap();
        assert!(first.left() >= px(0.), "{width}: {first:?}");
        assert!(last.right() <= px(width), "{width}: {last:?}");
        assert!(first.size.height >= px(48.) && last.size.width >= px(36.));
    }
}
