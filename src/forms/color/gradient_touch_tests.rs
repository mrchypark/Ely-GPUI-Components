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

    // The emulator reported x=420..600 at y=815; this is the same horizontal drag
    // of the stop just added by a tap, against a stable 390px test viewport.
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

#[gpui::test]
fn gradient_add_bar_uses_the_full_touch_height(cx: &mut TestAppContext) {
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
    let bar = cx.debug_bounds("gradient-touch-bar").expect("gradient bar");
    assert_eq!(bar.size.height, px(48.), "gradient bar: {bar:?}");
    for (x, y) in [(0.3, bar.top() + px(4.)), (0.7, bar.bottom() - px(4.))] {
        cx.simulate_click(point(bar.left() + bar.size.width * x, y), Modifiers::none());
        cx.run_until_parked();
    }
    assert_eq!(view.read_with(cx, |view, _| view.stops.len()), 4);
}

#[gpui::test]
fn gradient_touch_axes_share_scroll_and_cancel_restores(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let wheels = Rc::new(Cell::new(0));
    let scroll_handle = gpui::ScrollHandle::new();
    let (probe_wheels, probe_scroll) = (wheels.clone(), scroll_handle.clone());
    let (view, cx) = cx.add_window_view(move |_, _| GradientScrollProbe {
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
        wheels: probe_wheels.clone(),
        scroll: probe_scroll.clone(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let start = point(rail.left(), rail.center().y);
    let initial = scroll_handle.offset();

    scroll(cx, TouchPhase::Started, start, Pixels::ZERO, px(-30.));
    scroll(cx, TouchPhase::Ended, start, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops[0].at), 0.);
    assert!(wheels.get() > 0, "the vertical swipe reached its ancestor");
    assert!(scroll_handle.offset().y < initial.y);
    let wheels_after_vertical = wheels.get();

    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("scrolled gradient rail");
    let start = point(rail.left(), rail.center().y);
    let horizontal = point(start.x + rail.size.width * 0.2, start.y);
    scroll(
        cx,
        TouchPhase::Started,
        horizontal,
        rail.size.width * 0.2,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Moved,
        horizontal,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Ended,
        horizontal,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    let moved = view.read_with(cx, |view, _| view.stops.clone());
    assert!(
        (moved[0].at - 0.2).abs() < 0.01,
        "horizontal drag: {moved:?}"
    );
    assert_eq!(
        wheels.get(),
        wheels_after_vertical,
        "horizontal stays local"
    );

    scroll(
        cx,
        TouchPhase::Moved,
        start,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Ended, start, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert_eq!(
        wheels.get(),
        wheels_after_vertical,
        "horizontal momentum stays local"
    );
    assert_eq!(view.read_with(cx, |view, _| view.stops.clone()), moved);

    let start = point(rail.left() + rail.size.width * 0.2, rail.center().y);
    let cancelled = point(start.x + rail.size.width * 0.1, start.y);
    scroll(
        cx,
        TouchPhase::Started,
        cancelled,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Cancelled,
        cancelled,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops.clone()), moved);
    assert_eq!(wheels.get(), wheels_after_vertical, "cancel stays local");

    let wheels_before_vertical = wheels.get();
    scroll(cx, TouchPhase::Started, start, Pixels::ZERO, px(-30.));
    scroll(cx, TouchPhase::Ended, start, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert!(wheels.get() > wheels_before_vertical);
    assert_eq!(view.read_with(cx, |view, _| view.stops.clone()), moved);
}

#[gpui::test]
fn caught_fling_zero_start_activates_the_new_horizontal_drag(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let wheels = Rc::new(Cell::new(0));
    let scroll_handle = gpui::ScrollHandle::new();
    let (probe_wheels, probe_scroll) = (wheels.clone(), scroll_handle.clone());
    let (view, cx) = cx.add_window_view(move |_, _| GradientScrollProbe {
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
        wheels: probe_wheels.clone(),
        scroll: probe_scroll.clone(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let first = point(rail.left(), rail.center().y);
    let first_end = point(first.x + rail.size.width * 0.2, first.y);
    scroll(
        cx,
        TouchPhase::Started,
        first_end,
        rail.size.width * 0.2,
        Pixels::ZERO,
    );
    scroll(cx, TouchPhase::Ended, first_end, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert!(
        (view.read_with(cx, |view, _| view.stops[0].at) - 0.2).abs() < 0.01,
        "the preceding fling must move the first stop"
    );
    let wheels_before = wheels.get();

    let caught = point(rail.left() + rail.size.width * 0.2, rail.center().y);
    scroll(cx, TouchPhase::Ended, caught, Pixels::ZERO, Pixels::ZERO);
    scroll(cx, TouchPhase::Started, caught, Pixels::ZERO, Pixels::ZERO);
    let caught_first = point(caught.x + rail.size.width * 0.1, caught.y);
    scroll(
        cx,
        TouchPhase::Started,
        caught_first,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    let caught_second = point(caught.x + rail.size.width * 0.2, caught.y);
    scroll(
        cx,
        TouchPhase::Moved,
        caught_second,
        rail.size.width * 0.1,
        Pixels::ZERO,
    );
    scroll(
        cx,
        TouchPhase::Ended,
        caught_second,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert!(
        (stops[0].at - 0.4).abs() < 0.01,
        "caught fling drag: {stops:?}"
    );
    assert_eq!(wheels.get(), wheels_before, "horizontal catch stays local");
}

#[gpui::test]
fn pending_vertical_move_does_not_focus_or_select_a_stop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let wheels = Rc::new(Cell::new(0));
    let (view, cx) = cx.add_window_view(|_, _| GradientScrollProbe {
        stops: vec![
            GradientStop {
                at: 0.0,
                color: rgb(0xff0000).into(),
            },
            GradientStop {
                at: 0.5,
                color: rgb(0x00ff00).into(),
            },
            GradientStop {
                at: 1.0,
                color: rgb(0x0000ff).into(),
            },
        ],
        wheels: wheels.clone(),
        scroll: gpui::ScrollHandle::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    let middle = point(rail.left() + rail.size.width * 0.5, rail.center().y);
    scroll(cx, TouchPhase::Started, middle, Pixels::ZERO, Pixels::ZERO);
    scroll(cx, TouchPhase::Moved, middle, Pixels::ZERO, px(-30.));
    scroll(cx, TouchPhase::Ended, middle, Pixels::ZERO, Pixels::ZERO);
    cx.run_until_parked();
    assert_eq!(wheels.get(), 1, "the vertical pan reaches its ancestor");
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops[1].at), 0.5);
}

#[gpui::test]
fn pending_cancel_does_not_focus_or_select_a_stop(cx: &mut TestAppContext) {
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
                at: 0.5,
                color: rgb(0x00ff00).into(),
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
    let middle = point(rail.left() + rail.size.width * 0.5, rail.center().y);
    scroll(cx, TouchPhase::Started, middle, Pixels::ZERO, Pixels::ZERO);
    scroll(
        cx,
        TouchPhase::Cancelled,
        middle,
        Pixels::ZERO,
        Pixels::ZERO,
    );
    cx.run_until_parked();
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.stops[1].at), 0.5);
}

#[gpui::test]
fn tapping_a_stop_does_not_click_its_ancestor(cx: &mut TestAppContext) {
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
                at: 0.5,
                color: rgb(0x00ff00).into(),
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
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    cx.simulate_click(
        point(rail.left() + rail.size.width * 0.5, rail.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.taps), 0);
    assert_eq!(view.read_with(cx, |view, _| view.stops.len()), 3);
}

#[gpui::test]
fn touch_remove_button_deletes_the_selected_stop(cx: &mut TestAppContext) {
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
            stop(0.0, 0xff0000),
            stop(0.5, 0x00ff00),
            stop(1.0, 0x0000ff),
        ],
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let rail = cx
        .debug_bounds("gradient-touch-rail")
        .expect("gradient rail");
    cx.simulate_click(
        point(rail.left() + rail.size.width * 0.5, rail.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    let remove = cx
        .debug_bounds("gradient-touch-remove")
        .expect("touch remove button");
    assert_eq!(remove.size, gpui::size(px(48.), px(48.)));
    cx.simulate_click(remove.center(), Modifiers::none());
    cx.run_until_parked();
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    assert_eq!(places(&stops), [0.0, 1.0]);
    assert_eq!(place(&stops, 0xff0000), 0.0);
    assert_eq!(place(&stops, 0x0000ff), 1.0);
    assert!(
        cx.debug_bounds("gradient-touch-remove").is_none(),
        "the two-stop minimum has no removable stop"
    );
}
