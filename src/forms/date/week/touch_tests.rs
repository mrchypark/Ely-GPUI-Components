use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, Window, div, point, px, size,
};
use jiff::civil::date;

use super::WeekPicker;
use crate::theme::Theme;

struct Host {
    picked: Vec<(i16, i8)>,
    behind: usize,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let behind = cx.entity();
        let picker = cx.entity();
        div()
            .size_full()
            .child(
                div()
                    .id("behind")
                    .debug_selector(|| "week-behind".into())
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(px(480.))
                    .on_click(move |_, _, cx| behind.update(cx, |host, _| host.behind += 1)),
            )
            .child(
                div().absolute().top(px(480.)).left(px(8.)).child(
                    WeekPicker::new("overlap-week", Some((2026, 39)))
                        .today(date(2026, 9, 10))
                        .on_change(move |value, _, cx| {
                            picker.update(cx, |host, _| host.picked.push(value))
                        }),
                ),
            )
    }
}

#[gpui::test]
fn week_popup_blocks_background_selects_once_and_dismisses_outside(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Host {
            picked: vec![],
            behind: 0,
        }
    });
    cx.simulate_resize(size(px(320.), px(600.)));
    cx.run_until_parked();
    let field = cx.debug_bounds("field-root").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    let popup = cx.debug_bounds("week-popup").expect("popup opened");
    let day = cx
        .debug_bounds("calendar-day-2026-09-23")
        .expect("week day");
    let behind = cx.debug_bounds("week-behind").unwrap();
    assert!(
        day.bottom() <= field.top(),
        "popup must open above: {day:?} {field:?}"
    );
    assert!(behind.contains(&day.center()));
    let padding = popup.origin + point(px(2.), px(2.));
    assert!(behind.contains(&padding));
    cx.simulate_click(padding, Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("week-popup").is_some());
    assert_eq!(view.read_with(cx, |host, _| host.behind), 0);
    cx.simulate_click(day.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |host, _| (host.picked.clone(), host.behind)),
        (vec![(2026, 39)], 0)
    );
    assert!(cx.debug_bounds("week-popup").is_none());
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("week-popup").is_some());
    cx.simulate_click(point(px(310.), px(590.)), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("week-popup").is_none());
    assert_eq!(
        view.read_with(cx, |host, _| (host.picked.clone(), host.behind)),
        (vec![(2026, 39)], 0)
    );
}
