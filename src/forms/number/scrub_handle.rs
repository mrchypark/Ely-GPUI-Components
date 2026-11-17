use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, CursorStyle, DispatchPhase, DragMoveEvent, ElementId, EmptyView, Entity,
    HitboxBehavior, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, TouchDragEvent, TouchPhase, Window, canvas, div, prelude::*,
};

use super::{Numeric, Scrub, commit, settle};
use crate::theme::ActiveTheme;

/// Wraps `face` so dragging it sideways moves the number, one step per notch.
pub(super) fn scrub_handle(
    id: ElementId,
    face: AnyElement,
    state: Entity<Numeric>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let owner = window
        .use_keyed_state(id.clone(), cx, |_, _| ())
        .entity_id();
    let notch = cx.theme().handle_hit().to_pixels(window.rem_size());
    let touch = window.use_keyed_state((id.clone(), "touch"), cx, |_, _| None);
    let touch_state = state.clone();
    div()
        .id(id)
        .relative()
        .cursor(CursorStyle::ResizeLeftRight)
        .when(cx.theme().touch_mode, |handle| {
            handle.child(
                canvas(
                    |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                    move |_, hitbox, window, _| {
                        window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                            if phase != DispatchPhase::Bubble {
                                return;
                            }
                            if event.phase == TouchPhase::Started {
                                if window.default_prevented()
                                    || !hitbox.is_hovered_at(event.start_position, window)
                                {
                                    return;
                                }
                                let from = touch_state.read(cx).current(cx);
                                touch.update(cx, |touch, _| {
                                    *touch = Some((event.start_position, from));
                                });
                                window.prevent_default();
                            }
                            let Some((start, from)) = *touch.read(cx) else {
                                return;
                            };
                            if start != event.start_position {
                                return;
                            }
                            cx.stop_propagation();
                            if event.phase == TouchPhase::Cancelled {
                                touch.update(cx, |touch, _| *touch = None);
                                return;
                            }
                            let notches = f64::from(((event.position.x - start.x) / notch).trunc());
                            let numeric = touch_state.read(cx);
                            let (min, max, step, precision) = numeric.limits;
                            let next = settle(from + notches * step, min, max, precision);
                            if next != numeric.current(cx) {
                                commit(&touch_state, next, window, cx);
                            }
                            if event.phase == TouchPhase::Ended {
                                touch.update(cx, |touch, _| *touch = None);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
        })
        .on_drag(
            Scrub {
                owner,
                start: Rc::new(Cell::new(None)),
            },
            |scrub, _, _, cx| {
                scrub.start.set(None);
                cx.new(|_| EmptyView)
            },
        )
        .on_drag_move(move |event: &DragMoveEvent<Scrub>, window, cx| {
            let scrub = event.drag(cx);
            if scrub.owner != owner {
                return;
            }
            let (start, x) = (scrub.start.clone(), event.event.position.x);
            let (anchor, from) = start.get().unwrap_or_else(|| {
                let fresh = (x, state.read(cx).current(cx));
                start.set(Some(fresh));
                fresh
            });
            let notches = f64::from(((x - anchor) / notch).trunc());
            let step = state.read(cx).limits.2;
            commit(&state, from + notches * step, window, cx);
        })
        .child(face)
}
