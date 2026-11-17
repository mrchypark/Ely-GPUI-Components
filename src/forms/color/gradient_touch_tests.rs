#[path = "gradient_touch_axes_tests.rs"]
mod axes;
use std::{cell::Cell, rc::Rc};

use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Pixels, Render,
    ScrollDelta, ScrollEvent, ScrollSource, StatefulInteractiveElement, Styled, TestAppContext,
    TouchPhase, VisualTestContext, Window, div, point, px, rgb,
};

use super::{GradientEditor, GradientStop};
use crate::theme::Theme;

struct GradientProbe {
    stops: Vec<GradientStop>,
}

struct GradientAncestorProbe {
    stops: Vec<GradientStop>,
    taps: usize,
}

struct GradientScrollProbe {
    stops: Vec<GradientStop>,
    wheels: Rc<Cell<usize>>,
    scroll: gpui::ScrollHandle,
}

impl Render for GradientAncestorProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let taps = view.clone();
        div()
            .id("gradient-ancestor")
            .size_full()
            .on_click(move |_, _, cx| {
                taps.update(cx, |view, cx| {
                    view.taps += 1;
                    cx.notify();
                });
            })
            .child(
                GradientEditor::new("gradient-touch", self.stops.clone()).on_change(
                    move |stops, _, cx| {
                        view.update(cx, |view, cx| {
                            view.stops = stops.to_vec();
                            cx.notify();
                        });
                    },
                ),
            )
    }
}

impl Render for GradientProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            GradientEditor::new("gradient-touch", self.stops.clone()).on_change(
                move |stops, _, cx| {
                    view.update(cx, |view, cx| {
                        view.stops = stops.to_vec();
                        cx.notify();
                    });
                },
            ),
        )
    }
}

impl Render for GradientScrollProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let wheels = self.wheels.clone();
        div()
            .id("gradient-scroll-parent")
            .w(px(320.))
            .h(px(160.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll(move |event, _, _| {
                if let ScrollDelta::Pixels(delta) = event.delta
                    && (delta.x != Pixels::ZERO || delta.y != Pixels::ZERO)
                {
                    wheels.set(wheels.get() + 1);
                }
            })
            .child(
                GradientEditor::new("gradient-touch", self.stops.clone()).on_change(
                    move |stops, _, cx| {
                        view.update(cx, |view, cx| {
                            view.stops = stops.to_vec();
                            cx.notify();
                        });
                    },
                ),
            )
            .child(div().h(px(1000.)))
    }
}

fn scroll(
    cx: &mut VisualTestContext,
    phase: TouchPhase,
    position: gpui::Point<gpui::Pixels>,
    x: Pixels,
    y: Pixels,
) {
    cx.simulate_event(ScrollEvent {
        position,
        delta: ScrollDelta::Pixels(point(x, y)),
        modifiers: Modifiers::none(),
        touch_phase: phase,
        source: ScrollSource::Touch,
    });
}

fn places(stops: &[GradientStop]) -> Vec<f32> {
    stops.iter().map(|stop| stop.at).collect()
}

fn place(stops: &[GradientStop], color: u32) -> f32 {
    let color: gpui::Hsla = rgb(color).into();
    stops
        .iter()
        .find(|stop| stop.color == color)
        .expect("gradient stop")
        .at
}

#[gpui::test]
fn gradient_stop_touch_drag_accumulates_moves_and_cancel_restores(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let stop = |at, color| GradientStop {
        at,
        color: rgb(color).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![stop(0.0, 0xff0000), stop(1.0, 0x0000ff)],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();

    let bar = cx.debug_bounds("gradient-touch-bar").expect("gradient bar");
    cx.simulate_click(
        point(bar.left() + bar.size.width * 0.8, bar.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops.len()), 3);

    // Mirror the emulator's horizontal drag (x=420..600 at y=815) on the 390px viewport.
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let at = |t: f32| point(rail.left() + rail.size.width * t, rail.center().y);
    let start = at(0.8);
    let end = at(0.65);
    let step = (end.x - start.x) / 5.;
    scroll(
        cx,
        TouchPhase::Started,
        point(start.x + step, start.y),
        step,
        Pixels::ZERO,
    );
    for moved in 1..=3 {
        scroll(
            cx,
            TouchPhase::Moved,
            point(start.x + step * moved as f32, start.y),
            step,
            Pixels::ZERO,
        );
    }
    scroll(cx, TouchPhase::Ended, end, step, Pixels::ZERO);
    cx.run_until_parked();
    let moved = view.read_with(cx, |view, _| view.stops.clone());
    assert!(
        (places(&moved)[1] - 0.65).abs() < 0.01,
        "touch drag left the stop at {moved:?}"
    );

    let start = at(0.65);
    let cancelled = at(0.3);
    let step = (cancelled.x - start.x) / 3.;
    scroll(
        cx,
        TouchPhase::Started,
        point(start.x + step, start.y),
        step,
        Pixels::ZERO,
    );
    for moved in 1..=2 {
        scroll(
            cx,
            TouchPhase::Moved,
            point(start.x + step * moved as f32, start.y),
            step,
            Pixels::ZERO,
        );
    }
    cx.run_until_parked();
    scroll(
        cx,
        TouchPhase::Cancelled,
        cancelled,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Moved,
        start,
        -rail.size.width * 0.2,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.stops.clone()),
        moved,
        "a cancelled drag must restore its origin and ignore later moves"
    );

    cx.simulate_click(
        point(bar.left() + bar.size.width * 0.9, bar.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.stops.len()),
        4,
        "tapping the bar must still add a stop"
    );
}

#[gpui::test]
fn first_scroll_position_is_not_offset_by_its_delta(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let start = point(rail.left(), rail.center().y);
    let slop = px(8.);
    let first = point(start.x + slop, start.y);
    let second = point(start.x + rail.size.width * 0.2, start.y);
    let end = point(start.x + rail.size.width * 0.4, start.y);
    scroll(cx, TouchPhase::Started, first, slop, Pixels::ZERO);
    scroll(
        cx,
        TouchPhase::Moved,
        second,
        second.x - first.x,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Ended, end, end.x - second.x, Pixels::ZERO);
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert!(
        (stops[0].at - 0.4).abs() < 0.01,
        "start slop offset: {stops:?}"
    );
}

#[gpui::test]
fn gradient_touch_targets_are_forty_eight_pixels_and_reach_the_ends(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let stop = |at, color| GradientStop {
        at,
        color: rgb(color).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![stop(0.0, 0xff0000), stop(1.0, 0x0000ff)],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();

    let hit = cx.debug_bounds("gradient-touch-hit").expect("touch target");
    assert_eq!(hit.size.height, px(48.), "touch target: {hit:?}");
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    for (start, end, color) in [
        (
            point(rail.left() - px(20.), rail.center().y - px(20.)),
            point(rail.left() + rail.size.width * 0.1, rail.center().y),
            0xff0000,
        ),
        (
            point(rail.right() + px(20.), rail.center().y - px(20.)),
            point(rail.left() + rail.size.width * 0.9, rail.center().y),
            0x0000ff,
        ),
    ] {
        scroll(cx, TouchPhase::Started, end, end.x - start.x, Pixels::ZERO);
        scroll(cx, TouchPhase::Moved, end, Pixels::ZERO, Pixels::ZERO);
        scroll(cx, TouchPhase::Ended, end, Pixels::ZERO, Pixels::ZERO);
        cx.run_until_parked();
        let stops = view.read_with(cx, |view, _| view.stops.clone());
        let expected: gpui::Hsla = rgb(color).into();
        let actual = stops.iter().find(|stop| stop.color == expected).unwrap().at;
        assert!(
            (actual - if color == 0xff0000 { 0.1 } else { 0.9 }).abs() < 0.01,
            "endpoint touch left {stops:?}"
        );
    }
}

#[gpui::test]
fn adding_a_stop_does_not_click_its_ancestor(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| GradientAncestorProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
        taps: 0,
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let bar = cx.debug_bounds("gradient-touch-bar").expect("gradient bar");
    cx.simulate_click(
        point(bar.left() + bar.size.width * 0.6, bar.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops.len()), 3);
    assert_eq!(
        view.read_with(cx, |view, _| view.taps),
        0,
        "adding a stop must not activate its ancestor"
    );
}

#[gpui::test]
fn overlapping_equal_stops_touch_prefers_the_selected_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let stop = |at, color| GradientStop {
        at,
        color: rgb(color).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![
            stop(0.5, 0xff0000),
            stop(0.6, 0x00ff00),
            stop(0.8, 0x0000ff),
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let selected = point(rail.left() + rail.size.width * 0.6, rail.center().y);
    cx.simulate_click(selected, Modifiers::none());
    view.update(cx, |view, cx| {
        view.stops[1].at = 0.5;
        cx.notify();
    });
    cx.run_until_parked();
    let overlap = point(rail.left() + rail.size.width * 0.5, rail.center().y);
    let end = point(rail.left() + rail.size.width * 0.6, rail.center().y);
    scroll(
        cx,
        TouchPhase::Started,
        end,
        end.x - overlap.x,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Moved, end, Pixels::ZERO, Pixels::ZERO);
    scroll(cx, TouchPhase::Ended, end, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert_eq!(
        place(&stops, 0xff0000),
        0.5,
        "unselected stop moved: {stops:?}"
    );
    assert!(
        (place(&stops, 0x00ff00) - 0.6).abs() < 0.01,
        "selected stop: {stops:?}"
    );
}

#[gpui::test]
fn overlapping_equal_stops_touch_uses_the_topmost_when_selection_is_elsewhere(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let stop = |at, color| GradientStop {
        at,
        color: rgb(color).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| GradientProbe {
        stops: vec![
            stop(0.5, 0xff0000),
            stop(0.5, 0x00ff00),
            stop(0.8, 0x0000ff),
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let selected = point(rail.left() + rail.size.width * 0.8, rail.center().y);
    cx.simulate_click(selected, Modifiers::none());
    cx.run_until_parked();
    let overlap = point(rail.left() + rail.size.width * 0.5, rail.center().y);
    let end = point(rail.left() + rail.size.width * 0.6, rail.center().y);
    scroll(
        cx,
        TouchPhase::Started,
        end,
        end.x - overlap.x,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Moved, end, Pixels::ZERO, Pixels::ZERO);
    scroll(cx, TouchPhase::Ended, end, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert_eq!(place(&stops, 0xff0000), 0.5, "lower stop moved: {stops:?}");
    assert!(
        (place(&stops, 0x00ff00) - 0.6).abs() < 0.01,
        "topmost stop: {stops:?}"
    );
}
