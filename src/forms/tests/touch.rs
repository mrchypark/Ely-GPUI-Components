mod design;
mod slider;

use std::{cell::RefCell, rc::Rc};

use crate::{
    forms::{Checkbox, Choice, Radio, RangeSlider, Select, Slider, Switch},
    theme::{ControlSize, Theme},
};
use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px,
};

struct Controls {
    counts: [usize; 3],
    disabled: bool,
}
impl Render for Controls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(
                div().flex().debug_selector(|| "touch-check".into()).child(
                    Checkbox::new("check", false)
                        .disabled(self.disabled)
                        .on_change({
                            let view = cx.entity();
                            move |_, _, cx| view.update(cx, |v, _| v.counts[0] += 1)
                        }),
                ),
            )
            .child(
                div().flex().debug_selector(|| "touch-radio".into()).child(
                    Radio::new("radio", self.counts[1] > 0)
                        .disabled(self.disabled)
                        .on_select({
                            let view = cx.entity();
                            move |_, cx| view.update(cx, |v, _| v.counts[1] += 1)
                        }),
                ),
            )
            .child(
                div().flex().debug_selector(|| "touch-switch".into()).child(
                    Switch::new("switch", false)
                        .disabled(self.disabled)
                        .on_change({
                            let view = cx.entity();
                            move |_, _, cx| view.update(cx, |v, _| v.counts[2] += 1)
                        }),
                ),
            )
            .child(
                Select::new("select", [Choice::new("one", "One")])
                    .size(ControlSize::Sm)
                    .disabled(self.disabled),
            )
    }
}

#[gpui::test]
fn touch_form_edges_activate_once_and_disabled_controls_stay_inert(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| {
            t.touch_mode = true;
            t.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Controls {
            counts: [0; 3],
            disabled: false,
        }
    });
    cx.run_until_parked();
    for name in ["touch-check", "touch-radio", "touch-switch", "field-root"] {
        let b = cx.debug_bounds(name).unwrap();
        assert!(
            b.size.width >= px(48.) && b.size.height >= px(48.),
            "{name}"
        );
    }
    for name in ["touch-check", "touch-radio", "touch-switch"] {
        let b = cx.debug_bounds(name).unwrap();
        cx.simulate_click(
            point(b.right() - px(2.), b.bottom() - px(2.)),
            Modifiers::none(),
        );
    }
    assert_eq!(view.read_with(cx, |v, _| v.counts), [1; 3]);
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    let radio_center = cx.debug_bounds("touch-radio").unwrap().center();
    cx.simulate_click(radio_center, Modifiers::none());
    assert_eq!(view.read_with(cx, |v, _| v.counts[1]), 1);
    let field_center = cx.debug_bounds("field-root").unwrap().center();
    cx.simulate_click(field_center, Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_some());
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_none());
    view.update(cx, |v, cx| {
        v.disabled = true;
        cx.notify();
    });
    cx.run_until_parked();
    for name in ["touch-check", "touch-radio", "touch-switch"] {
        let center = cx.debug_bounds(name).unwrap().center();
        cx.simulate_click(center, Modifiers::none());
    }
    assert_eq!(view.read_with(cx, |v, _| v.counts), [1; 3]);
    let field_center = cx.debug_bounds("field-root").unwrap().center();
    cx.simulate_click(field_center, Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("option-list").is_none());
    cx.update(|_, cx| Theme::update(cx, |t| t.touch_mode = false));
    cx.run_until_parked();
    assert!(cx.debug_bounds("touch-check").unwrap().size.height < px(48.));
}

struct TrackProbe {
    value: f64,
    span: (f64, f64),
    changes: Vec<(f64, f64)>,
    vertical: bool,
    range: bool,
}

struct CallbackOnly(Rc<RefCell<Vec<f64>>>);

impl Render for CallbackOnly {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.0.clone();
        div().w(px(240.)).h(px(240.)).child(
            Slider::new("track", 25.)
                .on_change(move |value, _, _| changes.borrow_mut().push(value)),
        )
    }
}

struct RangeCallbackOnly(Rc<RefCell<Vec<(f64, f64)>>>);

impl Render for RangeCallbackOnly {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let changes = self.0.clone();
        div().w(px(240.)).h(px(240.)).child(
            RangeSlider::new("track", 25., 75.)
                .on_change(move |value, _, _| changes.borrow_mut().push(value)),
        )
    }
}

#[gpui::test]
fn touch_drag_claims_slider_and_range_in_both_orientations(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| TrackProbe {
        value: 50.,
        span: (25., 75.),
        changes: Vec::new(),
        vertical: false,
        range: false,
    });
    for vertical in [false, true] {
        for (range, span, start, end, expected) in [
            (false, (25., 75.), 0.50, 0.80, (80., 0.)),
            (true, (25., 75.), 0.25, 0.40, (40., 75.)),
            (true, (25., 75.), 0.75, 0.90, (25., 90.)),
            (true, (50., 50.), 0.50, 0.80, (50., 80.)),
        ] {
            view.update(cx, |view, cx| {
                view.vertical = vertical;
                view.range = range;
                view.value = 50.;
                view.span = span;
                cx.notify();
            });
            cx.run_until_parked();
            let track = cx.debug_bounds("slider-track").unwrap();
            let at = |f| {
                if vertical {
                    point(track.center().x, track.bottom() - track.size.height * f)
                } else {
                    point(track.left() + track.size.width * f, track.center().y)
                }
            };
            let start_position = at(start);
            for (phase, position) in [
                (TouchPhase::Started, at((start + end) / 2.)),
                (TouchPhase::Moved, at(end)),
                (TouchPhase::Ended, at(end)),
            ] {
                cx.simulate_event(TouchDragEvent {
                    phase,
                    start_position,
                    position,
                });
                cx.run_until_parked();
            }
            let actual = view.read_with(
                cx,
                |view, _| if range { view.span } else { (view.value, 0.) },
            );
            assert_eq!(
                actual, expected,
                "vertical={vertical}, range={range}, start={start}"
            );
            cx.simulate_event(TouchDragEvent {
                phase: TouchPhase::Moved,
                start_position,
                position: at(0.0),
            });
            cx.run_until_parked();
            let after_release =
                view.read_with(
                    cx,
                    |view, _| if range { view.span } else { (view.value, 0.) },
                );
            assert_eq!(
                after_release, expected,
                "movement after release must be ignored"
            );
        }
    }
}

#[gpui::test]
fn touch_release_applies_its_final_position(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| TrackProbe {
        value: 25.,
        span: (25., 75.),
        changes: Vec::new(),
        vertical: false,
        range: false,
    });
    for vertical in [false, true] {
        for (range, span, value, start, end, expected) in [
            (false, (25., 75.), 25., 0.25, 0.80, (80., 0.)),
            (true, (25., 75.), 50., 0.75, 0.40, (25., 40.)),
        ] {
            view.update(cx, |view, cx| {
                view.vertical = vertical;
                view.range = range;
                view.value = value;
                view.span = span;
                cx.notify();
            });
            cx.run_until_parked();
            let track = cx.debug_bounds("slider-track").unwrap();
            let at = |fraction| {
                if vertical {
                    point(
                        track.center().x,
                        track.bottom() - track.size.height * fraction,
                    )
                } else {
                    point(track.left() + track.size.width * fraction, track.center().y)
                }
            };
            let start_position = at(start);
            cx.simulate_event(TouchDragEvent {
                phase: TouchPhase::Started,
                start_position,
                position: start_position,
            });
            cx.simulate_event(TouchDragEvent {
                phase: TouchPhase::Ended,
                start_position,
                position: at(end),
            });
            cx.run_until_parked();
            assert_eq!(
                view.read_with(cx, |view, _| if range {
                    view.span
                } else {
                    (view.value, 0.)
                }),
                expected,
                "vertical={vertical}, range={range}"
            );
            cx.simulate_event(TouchDragEvent {
                phase: TouchPhase::Moved,
                start_position,
                position: at(if end > 0.5 { 0.0 } else { 1.0 }),
            });
            cx.run_until_parked();
            assert_eq!(
                view.read_with(cx, |view, _| if range {
                    view.span
                } else {
                    (view.value, 0.)
                }),
                expected,
                "movement after release must be ignored"
            );
        }
    }
}

#[gpui::test]
fn touch_cancel_restores_the_value_before_the_drag(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| TrackProbe {
        value: 25.,
        span: (25., 75.),
        changes: Vec::new(),
        vertical: false,
        range: false,
    });
    for (range, span, value, start, end, expected) in [
        (false, (25., 75.), 25., 0.25, 0.80, (25., 0.)),
        (true, (25., 75.), 50., 0.75, 0.40, (25., 75.)),
    ] {
        view.update(cx, |view, cx| {
            view.range = range;
            view.value = value;
            view.span = span;
            view.changes.clear();
            cx.notify();
        });
        cx.run_until_parked();
        let track = cx.debug_bounds("slider-track").unwrap();
        let at = |fraction| point(track.left() + track.size.width * fraction, track.center().y);
        let start_position = at(start);
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Started,
            start_position,
            position: start_position,
        });
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Moved,
            start_position,
            position: at(end),
        });
        cx.run_until_parked();
        assert_ne!(
            view.read_with(cx, |view, _| if range {
                view.span
            } else {
                (view.value, 0.)
            }),
            expected
        );
        view.update(cx, |view, _| view.changes.clear());
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Cancelled,
            start_position,
            position: at(end),
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| if range {
                view.span
            } else {
                (view.value, 0.)
            }),
            expected,
            "range={range}"
        );
        assert_eq!(
            view.read_with(cx, |view, _| view.changes.clone()),
            vec![expected],
            "the cancel emits one restoration"
        );
    }
}

#[gpui::test]
fn touch_end_does_not_repeat_the_last_stepped_value(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let changes = Rc::new(RefCell::new(Vec::new()));
    let (_view, cx) = cx.add_window_view(|_, _| CallbackOnly(changes.clone()));
    cx.run_until_parked();
    let track = cx.debug_bounds("slider-track").unwrap();
    let at = |fraction| point(track.left() + track.size.width * fraction, track.center().y);
    let start_position = at(0.25);
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Started,
        start_position,
        position: start_position,
    });
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Moved,
        start_position,
        position: at(0.80),
    });
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Ended,
        start_position,
        position: at(0.8025),
    });
    cx.run_until_parked();
    assert_eq!(*changes.borrow(), [80.]);
}

#[gpui::test]
fn touch_cancel_restores_without_a_parent_render(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let changes = Rc::new(RefCell::new(Vec::new()));
    let (_view, cx) = cx.add_window_view(|_, _| CallbackOnly(changes.clone()));
    cx.run_until_parked();
    let track = cx.debug_bounds("slider-track").unwrap();
    let at = |fraction| point(track.left() + track.size.width * fraction, track.center().y);
    let start_position = at(0.25);
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Started,
        start_position,
        position: start_position,
    });
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Moved,
        start_position,
        position: at(0.80),
    });
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Cancelled,
        start_position,
        position: at(0.80),
    });
    cx.run_until_parked();
    assert_eq!(*changes.borrow(), [80., 25.]);
}
