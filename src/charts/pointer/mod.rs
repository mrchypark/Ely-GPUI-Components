use std::rc::Rc;

use gpui::{
    App, Bounds, DispatchPhase, Div, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, PinchEvent, Pixels,
    Point, ScrollEvent, ScrollSource, Stateful, StatefulInteractiveElement, Styled, TouchDragEvent,
    TouchPhase, Window, canvas, fill, prelude::*, size,
};

use super::{
    geometry::{Dot, Geometry, Ink, Rect},
    paint::{Pen, at, finish, paint, place, ring},
    plot::{Pick, Plot, State},
    scale::Band,
};

/// What the pointer at a place picks: a category's band, the nearest category, or the nearest dot within reach.
mod pick;
pub(crate) use pick::{Focus, drawing, focus, pick_at};
use pick::{local, pan_xy_view, pick_hover, zoom_xy_view};

/// Wires the pointer: hover picks, and on a zoomable plot a drag brushes a span and a double press shows all.
pub(crate) fn answer(
    root: Stateful<Div>,
    state: &Entity<State>,
    (pick, horizontal, zoomable): (Pick, bool, bool),
    (rect, span, reach): (Rect, (usize, usize), f32),
    geometry: Rc<Geometry>,
) -> Stateful<Div> {
    let (moved, left, pressed, released) =
        (state.clone(), state.clone(), state.clone(), state.clone());
    let (dot_down, dot_up, dot_scroll, dot_pinch) =
        (state.clone(), state.clone(), state.clone(), state.clone());
    let tap_pressed = state.clone();
    let move_geometry = geometry.clone();
    let end_brush = move |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
        finish_brush(&released, pick, rect, span, cx);
    };
    root.on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
        let place = local(event.position, moved.read(cx).bounds);
        moved.update(cx, |state, cx| {
            if pick == Pick::Dot
                && event.pressed_button == Some(MouseButton::Left)
                && let Some(last) = state.drag_last
            {
                let (dx, dy) = (place.0 - last.0, place.1 - last.1);
                if dx.hypot(dy) > 1.5 {
                    state.xy_view = pan_xy_view(state.xy_view, (dx, dy), rect);
                    state.drag_last = Some(place);
                    state.hover = None;
                    cx.notify();
                    return;
                }
            }
            if event.pressed_button == Some(MouseButton::Left)
                && let Some(brush) = state.brush.as_mut()
            {
                brush.1 = place.0.clamp(rect.x, rect.x + rect.w);
                cx.notify();
                return;
            }
            let next = pick_at(
                pick,
                place,
                rect,
                horizontal,
                span,
                &move_geometry.dots,
                reach,
            );
            if state.hover != next || (next.is_some() && pick != Pick::Dot) {
                state.hover = next;
                state.pointer = place;
                cx.notify();
            }
        })
    })
    .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
        let place = local(event.position, tap_pressed.read(cx).bounds);
        pick_hover(
            &tap_pressed,
            place,
            (pick, horizontal),
            (rect, span, reach),
            &geometry,
            cx,
        );
    })
    .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
    .on_hover(move |inside, _, cx| {
        if !*inside {
            left.update(cx, |state, cx| {
                state.hover = None;
                cx.notify();
            })
        }
    })
    .when(pick == Pick::Dot, |root| {
        let dot_up_out = dot_up.clone();
        root.on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
            let place = local(event.position, dot_down.read(cx).bounds);
            dot_down.update(cx, |state, cx| {
                if event.click_count >= 2 {
                    state.xy_view = None;
                    state.drag_last = None;
                    cx.notify();
                } else if rect.contains(place) {
                    state.drag_last = Some(place);
                }
            });
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            dot_up.update(cx, |state, _| state.drag_last = None);
        })
        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
            dot_up_out.update(cx, |state, _| state.drag_last = None);
        })
        .on_scroll(move |ev: &ScrollEvent, _, cx| {
            let place = local(ev.position, dot_scroll.read(cx).bounds);
            if !rect.contains(place) || rect.w <= 1.0 || rect.h <= 1.0 {
                return;
            }
            let delta = ev.delta.pixel_delta(Pixels::from(16.0));
            dot_scroll.update(cx, |state, cx| {
                if ev.source == ScrollSource::Touch {
                    let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                    if dx.abs() >= 0.1 || dy.abs() >= 0.1 {
                        state.xy_view = pan_xy_view(state.xy_view, (dx, dy), rect);
                        state.hover = None;
                        cx.notify();
                    }
                } else {
                    let dy = f32::from(delta.y);
                    if dy.abs() >= 0.05 {
                        let factor = (1.0 - f64::from(dy) * 0.002).clamp(0.2, 5.0);
                        let tx = f64::from((place.0 - rect.x) / rect.w);
                        let ty = f64::from((rect.y + rect.h - place.1) / rect.h);
                        state.xy_view = zoom_xy_view(state.xy_view, (tx, ty), factor);
                        cx.notify();
                    }
                }
            });
            cx.stop_propagation();
        })
        .on_pinch(move |ev: &PinchEvent, _, cx| {
            let place = local(ev.position, dot_pinch.read(cx).bounds);
            if !rect.contains(place) || rect.w <= 1.0 || rect.h <= 1.0 {
                return;
            }
            let scale = ev.delta.exp().clamp(0.2, 5.0);
            if (scale - 1.0).abs() > 1e-4 {
                dot_pinch.update(cx, |state, cx| {
                    state.drag_last = None;
                    state.brush = None;
                    let tx = f64::from((place.0 - rect.x) / rect.w);
                    let ty = f64::from((rect.y + rect.h - place.1) / rect.h);
                    state.xy_view = zoom_xy_view(state.xy_view, (tx, ty), 1.0 / f64::from(scale));
                    cx.notify();
                });
                cx.stop_propagation();
            }
        })
    })
    .when(zoomable, |root| {
        let pinch_cat = state.clone();
        let touch = state.clone();
        root.cursor_crosshair()
            .child(
                canvas(
                    |_, _, _| {},
                    move |_, _, window, _| {
                        let touch = touch.clone();
                        window.on_mouse_event(move |ev: &TouchDragEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            let bounds = touch.read(cx).bounds;
                            let start = local(ev.start_position, bounds);
                            let at = local(ev.position, bounds);
                            if ev.phase == TouchPhase::Started {
                                if window.default_prevented()
                                    || !rect.contains(start)
                                    || (at.0 - start.0).abs() <= (at.1 - start.1).abs()
                                {
                                    return;
                                }
                                touch.update(cx, |state, cx| {
                                    state.brush =
                                        Some((start.0, at.0.clamp(rect.x, rect.x + rect.w)));
                                    state.hover = None;
                                    cx.notify();
                                });
                                window.prevent_default();
                                cx.stop_propagation();
                            } else if ev.phase == TouchPhase::Cancelled {
                                touch.update(cx, |state, cx| {
                                    state.brush = None;
                                    cx.notify();
                                });
                            } else if touch.read(cx).brush.is_some() {
                                touch.update(cx, |state, cx| {
                                    if let Some(brush) = state.brush.as_mut() {
                                        brush.1 = at.0.clamp(rect.x, rect.x + rect.w);
                                    }
                                    cx.notify();
                                });
                                if ev.phase == TouchPhase::Ended {
                                    finish_brush(&touch, pick, rect, span, cx);
                                }
                                cx.stop_propagation();
                            }
                        });
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let place = local(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |state, cx| {
                    if event.click_count == 2 {
                        state.span = None;
                        log::info!("chart: zoom reset");
                    } else if rect.contains(place) {
                        state.brush = Some((place.0, place.0));
                    }
                    cx.notify();
                })
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                let scale = ev.delta.exp().clamp(0.25, 4.0);
                if (scale - 1.0).abs() <= 1e-4 {
                    return;
                }
                pinch_cat.update(cx, |state, cx| {
                    state.brush = None;
                    let total = span.1 + 1;
                    let (cur_a, cur_b) = state.span.unwrap_or(span);
                    let cur_len = (cur_b.saturating_sub(cur_a) + 1).max(2) as f32;
                    let next_len = (cur_len / scale).round() as usize;
                    if next_len >= total {
                        state.span = None;
                    } else {
                        let next_len = next_len.max(2);
                        let mid = (cur_a + cur_b) / 2;
                        let half = next_len / 2;
                        let from = mid.saturating_sub(half).min(total.saturating_sub(next_len));
                        let to = (from + next_len - 1).min(total.saturating_sub(1));
                        if to > from {
                            state.span = Some((from, to));
                        }
                    }
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .on_mouse_up(MouseButton::Left, end_brush.clone())
            .on_mouse_up_out(MouseButton::Left, end_brush)
    })
}

fn finish_brush(state: &Entity<State>, pick: Pick, rect: Rect, span: (usize, usize), cx: &mut App) {
    state.update(cx, |state, cx| {
        let Some((a, b)) = state.brush.take() else {
            return;
        };
        let end = |x: f32| {
            pick_at(
                pick,
                (x.clamp(rect.x, rect.x + rect.w), rect.y + rect.h / 2.0),
                rect,
                false,
                span,
                &[],
                0.0,
            )
        };
        if let (Some(from), Some(to)) = (end(a.min(b)), end(a.max(b)))
            && to > from
        {
            state.span = Some((from, to));
            log::info!("chart: zoomed to {from}..={to}");
        }
        cx.notify();
    });
}

#[cfg(test)]
mod tests;
