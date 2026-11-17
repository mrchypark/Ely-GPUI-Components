use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, Window, div, point, px, size,
};
use jiff::civil::date;

use super::{MonthPicker, QuarterPicker, YearPicker};
use crate::theme::Theme;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Month,
    Quarter,
    Year,
}

struct Host {
    kind: Kind,
    picked: Vec<(i16, i8)>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let host = cx.entity();
        let today = date(2026, 9, 10);
        let picker = match self.kind {
            Kind::Month => MonthPicker::new("picker", Some((2026, 1)))
                .today(today)
                .on_change(move |value, _, cx| host.update(cx, |host, _| host.picked.push(value)))
                .into_any_element(),
            Kind::Quarter => QuarterPicker::new("picker", Some((2026, 1)))
                .today(today)
                .on_change(move |value, _, cx| host.update(cx, |host, _| host.picked.push(value)))
                .into_any_element(),
            Kind::Year => YearPicker::new("picker", Some(2026))
                .today(today)
                .on_change(move |value, _, cx| {
                    host.update(cx, |host, _| host.picked.push((value, 0)))
                })
                .into_any_element(),
        };
        div().p_4().child(picker)
    }
}

struct OverlapHost {
    picked: Vec<(i16, i8)>,
    behind: usize,
}

impl Render for OverlapHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let behind = cx.entity();
        let picker = cx.entity();
        div()
            .size_full()
            .child(
                div()
                    .id("behind")
                    .debug_selector(|| "period-behind".into())
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(px(480.))
                    .on_click(move |_, _, cx| behind.update(cx, |host, _| host.behind += 1)),
            )
            .child(
                div().absolute().top(px(480.)).left(px(8.)).child(
                    MonthPicker::new("overlap-picker", Some((2026, 9)))
                        .today(date(2026, 9, 10))
                        .on_change(move |value, _, cx| {
                            picker.update(cx, |host, _| host.picked.push(value))
                        }),
                ),
            )
    }
}

#[gpui::test]
fn period_popup_blocks_overlapping_clicks_and_dismisses_outside(cx: &mut TestAppContext) {
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
        OverlapHost {
            picked: vec![],
            behind: 0,
        }
    });
    cx.simulate_resize(size(px(320.), px(600.)));
    cx.run_until_parked();
    let field = cx.debug_bounds("field-root").unwrap();
    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    let popup = cx.debug_bounds("period-popup").expect("popup opened");
    let jan = cx.debug_bounds("period-cell-0").expect("January cell");
    let behind = cx.debug_bounds("period-behind").unwrap();
    assert!(
        jan.bottom() <= field.top(),
        "popup did not open above field: jan={jan:?} field={field:?} popup={popup:?}"
    );
    assert!(
        behind.contains(&jan.center()),
        "cell must overlap the button"
    );
    let padding = popup.origin + point(px(2.), px(2.));
    assert!(behind.contains(&padding));
    cx.simulate_click(padding, Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("period-popup").is_some());
    assert_eq!(view.read_with(cx, |host, _| host.behind), 0);
    cx.simulate_click(jan.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |host, _| (host.picked.clone(), host.behind)),
        (vec![(2026, 1)], 0)
    );
    assert!(cx.debug_bounds("period-popup").is_none());

    cx.simulate_click(field.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("period-popup").is_some());
    cx.simulate_click(point(px(310.), px(590.)), Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("period-popup").is_none(),
        "outside click closes the popup"
    );
    assert_eq!(
        view.read_with(cx, |host, _| (host.picked.clone(), host.behind)),
        (vec![(2026, 1)], 0)
    );
}

#[gpui::test]
fn touch_period_cells_fit_phone_and_select(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    for width in [320., 390.] {
        for kind in [Kind::Month, Kind::Quarter, Kind::Year] {
            let (view, cx) = cx.add_window_view(|window, _| {
                window.set_rem_size(px(8.));
                Host {
                    kind,
                    picked: vec![],
                }
            });
            cx.simulate_resize(size(px(width), px(844.)));
            cx.update(|window, cx| window.focus_next(cx));
            cx.simulate_keystrokes("enter");
            cx.run_until_parked();
            let count = if matches!(kind, Kind::Quarter) { 4 } else { 12 };
            let selectors = [
                "period-cell-0",
                "period-cell-1",
                "period-cell-2",
                "period-cell-3",
                "period-cell-4",
                "period-cell-5",
                "period-cell-6",
                "period-cell-7",
                "period-cell-8",
                "period-cell-9",
                "period-cell-10",
                "period-cell-11",
            ];
            for selector in &selectors[..count] {
                let bounds = cx.debug_bounds(selector).unwrap();
                assert!(
                    bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
                    "{kind:?} {width}: {bounds:?}"
                );
                assert!(
                    bounds.left() >= px(0.) && bounds.right() <= px(width),
                    "{kind:?} {width}: {bounds:?}"
                );
            }
            let cell = cx.debug_bounds("period-cell-1").unwrap();
            cx.simulate_click(cell.center(), Modifiers::none());
            cx.run_until_parked();
            let expected = match kind {
                Kind::Month | Kind::Quarter => (2026, 2),
                Kind::Year => (2017, 0),
            };
            assert_eq!(
                view.read_with(cx, |host, _| host.picked.clone()),
                vec![expected]
            );
            assert!(cx.debug_bounds("period-cell-1").is_none());
        }
    }
}

#[gpui::test]
fn desktop_period_cells_keep_their_height(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = false;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Host {
            kind: Kind::Month,
            picked: vec![],
        }
    });
    cx.simulate_resize(size(px(320.), px(844.)));
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("period-cell-1").unwrap().size.height < px(48.));
}
