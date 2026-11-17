use std::rc::Rc;

use gpui::{
    App, Bounds, DragMoveEvent, ElementId, EmptyView, Entity, EntityId, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, ScrollDelta, ScrollSource,
    StatefulInteractiveElement, Styled, TouchPhase, Window, canvas, div, linear_color_stop,
    linear_gradient, prelude::*, relative,
};

use super::{ColorPicker, swatch::checker};
use crate::{
    buttons::{ButtonVariant, IconButton},
    layout::seeded::{Seeded, use_seeded},
    primitives::{IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, Radius},
};

/// A color at a place along a gradient, from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub at: f32,
    pub color: Hsla,
}

/// The color at `t` along stops sorted by place.
#[path = "gradient_stops.rs"]
mod stops;

#[cfg(test)]
pub(crate) use stops::color_at;
pub(crate) use stops::{added, moved, removed};

/// A stop handle being dragged; the grabbed stop is the chosen one.
struct Handle {
    owner: EntityId,
}

#[derive(Clone)]
struct TouchDrag {
    x: Pixels,
    stops: Vec<GradientStop>,
    index: usize,
}

#[derive(Clone)]
enum TouchState {
    Pending { start_x: Pixels },
    Drag(TouchDrag),
    Momentum,
}

type OnStops = Rc<dyn Fn(&[GradientStop], &mut Window, &mut App)>;
type SetStops = Rc<dyn Fn(Vec<GradientStop>, usize, &mut Window, &mut App)>;
type Live = Entity<Seeded<Vec<GradientStop>>>;

/// The stops and the chosen index as they stand when a handler runs.
fn now(live: &Live, chosen: &Entity<usize>, cx: &App) -> (Vec<GradientStop>, usize) {
    let stops = live.read(cx).value.clone();
    let at = (*chosen.read(cx)).min(stops.len() - 1);
    (stops, at)
}

fn grabbed(
    stops: &[GradientStop],
    bounds: Bounds<Pixels>,
    x: Pixels,
    selected: usize,
    hit: Pixels,
) -> Option<usize> {
    let mut nearest = None;
    for (index, stop) in stops.iter().enumerate() {
        let center = bounds.left() + bounds.size.width * stop.at;
        let distance = (x - center).abs();
        if distance <= hit * 0.5 && nearest.is_none_or(|(best, _)| distance <= best) {
            nearest = Some((distance, index));
        }
    }
    let (distance, topmost) = nearest?;
    let selected_distance = {
        let center = bounds.left() + bounds.size.width * stops[selected].at;
        (x - center).abs()
    };
    Some(if selected_distance == distance {
        selected
    } else {
        topmost
    })
}

/// A gradient bar with stops beneath. Drag a stop by pointer or touch, tap the bar to add one, Delete or the touch button removes it.
#[derive(IntoElement)]
pub struct GradientEditor {
    id: ElementId,
    stops: Vec<GradientStop>,
    on_change: Option<OnStops>,
}

impl GradientEditor {
    pub fn new(id: impl Into<ElementId>, stops: impl IntoIterator<Item = GradientStop>) -> Self {
        let stops: Vec<GradientStop> = stops.into_iter().collect();
        assert!(stops.len() >= 2, "a gradient needs two stops");
        assert!(
            stops.windows(2).all(|pair| pair[0].at <= pair[1].at)
                && stops.iter().all(|stop| (0.0..=1.0).contains(&stop.at)),
            "gradient stops must be sorted within 0 to 1"
        );
        Self {
            id: id.into(),
            stops,
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[GradientStop], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GradientEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let live = use_seeded((self.id.clone(), "stops"), self.stops, window, cx);
        let chosen = window.use_keyed_state((self.id.clone(), "chosen"), cx, |_, _| 0usize);
        let touch =
            window.use_keyed_state((self.id.clone(), "touch"), cx, |_, _| None::<TouchState>);
        let (stops, at) = now(&live, &chosen, cx);
        let count = stops.len();
        let bar = window.use_keyed_state((self.id.clone(), "bar"), cx, |_, _| {
            Bounds::<Pixels>::default()
        });
        let owner = window
            .use_keyed_state((self.id.clone(), "drag"), cx, |_, _| ())
            .entity_id();
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let commit: SetStops = {
            let (id, live, chosen, on_change) = (
                self.id.clone(),
                live.clone(),
                chosen.clone(),
                self.on_change,
            );
            Rc::new(move |next, ix, window, cx| {
                log::info!(
                    "gradient editor {id:?}: {} stops, stop {ix} chosen",
                    next.len()
                );
                chosen.update(cx, |chosen, cx| {
                    *chosen = ix;
                    cx.notify();
                });
                live.update(cx, |live, cx| {
                    live.value = next.clone();
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (knob, small) = (theme.slider_thumb(), theme.radius(Radius::Sm));
        let knob_px = knob.to_pixels(window.rem_size());
        let touch_hit = theme.touch_minimum(knob_px);
        let side = if theme.touch_mode {
            touch_hit * 0.5
        } else {
            knob_px * 0.5
        };
        let rail_height = if theme.touch_mode { touch_hit } else { knob_px };
        let rail_top = (rail_height - knob_px) * 0.5;
        let mut spans: Vec<(f32, f32, Hsla, Hsla)> = Vec::new();
        let (first, last) = (stops[0], stops[count - 1]);
        if first.at > 0.0 {
            spans.push((0.0, first.at, first.color, first.color));
        }
        spans.extend(
            stops
                .windows(2)
                .map(|pair| (pair[0].at, pair[1].at, pair[0].color, pair[1].color)),
        );
        if last.at < 1.0 {
            spans.push((last.at, 1.0, last.color, last.color));
        }
        let edge = spans.len() - 1;
        let bands = spans.into_iter().enumerate().map(|(ix, (from, to, a, b))| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(from))
                .w(relative(to - from))
                .bg(linear_gradient(
                    90.0,
                    linear_color_stop(a, 0.0),
                    linear_color_stop(b, 1.0),
                ))
                .when(ix == 0, |band| band.rounded_l(small))
                .when(ix == edge, |band| band.rounded_r(small))
        });
        let (measure, press) = (bar.clone(), bar.clone());
        let (add_live, add_chosen, add) = (live.clone(), chosen.clone(), commit.clone());
        let (bar_debug, rail_debug) = (self.id.clone(), self.id.clone());
        let strip = div()
            .id((self.id.clone(), "bar"))
            .debug_selector(|| format!("{bar_debug}-bar"))
            .relative()
            .w_full()
            .h(theme.control_height(ControlSize::Md))
            .map(|bar| theme.touch_target(bar))
            .cursor_crosshair()
            .child(checker(small, cx))
            .children(bands)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if *measure.read(cx) != bounds {
                            measure.update(cx, |bar, _| *bar = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                cx.stop_propagation();
                let bounds = *press.read(cx);
                let t = (event.position.x - bounds.left()) / bounds.size.width;
                let (stops, _) = now(&add_live, &add_chosen, cx);
                let (next, ix) = added(&stops, t);
                add(next, ix, window, cx);
            });
        let handles = stops.iter().enumerate().map(|(ix, stop)| {
            let pick = chosen.clone();
            div()
                .id(("stop", ix))
                .absolute()
                .top_0()
                .left(relative(stop.at))
                .ml(knob * -0.5)
                .size(knob)
                .rounded(small)
                .border_2()
                .border_color(if ix == at {
                    if focused { colors.focus } else { colors.accent }
                } else {
                    colors.border_strong
                })
                .bg(stop.color)
                .cursor_grab()
                .on_drag(Handle { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    pick.update(cx, |chosen, cx| {
                        *chosen = ix;
                        cx.notify();
                    })
                })
        });
        let (drag_live, drag_chosen, drag) = (live.clone(), chosen.clone(), commit.clone());
        let (touch_live, touch_chosen, touch_commit) =
            (live.clone(), chosen.clone(), commit.clone());
        let touch_focus = focus.clone();
        let touch_bar = bar.clone();
        let touch_debug = self.id.clone();
        let (key_live, key_chosen, keys) = (live.clone(), chosen.clone(), commit.clone());
        let rail = div()
            .id((self.id.clone(), "stops"))
            .debug_selector(|| format!("{rail_debug}-rail"))
            .track_focus(&focus)
            .relative()
            .h(knob)
            .on_drag_move(move |event: &DragMoveEvent<Handle>, window, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let bounds = event.bounds;
                let t = (event.event.position.x - bounds.left()) / bounds.size.width;
                let (stops, at) = now(&drag_live, &drag_chosen, cx);
                let (next, ix) = moved(&stops, at, t);
                drag(next, ix, window, cx);
            })
            .on_key_down(move |event, window, cx| {
                let (stops, at) = now(&key_live, &key_chosen, cx);
                let (next, ix) = match event.keystroke.key.as_str() {
                    "left" => moved(&stops, at, stops[at].at - 0.01),
                    "right" => moved(&stops, at, stops[at].at + 0.01),
                    "delete" | "backspace" if stops.len() > 2 => removed(&stops, at),
                    _ => return,
                };
                cx.stop_propagation();
                keys(next, ix, window, cx);
            })
            .children(handles);
        let (remove_live, remove_chosen, remove_commit) =
            (live.clone(), chosen.clone(), commit.clone());
        let remove_debug = self.id.clone();
        let remove = (theme.touch_mode && count > 2).then(|| {
            IconButton::new((self.id.clone(), "remove"), IconName::Trash2)
                .debug_selector(move || format!("{remove_debug}-remove"))
                .variant(ButtonVariant::Ghost)
                .tooltip("Remove stop")
                .on_click(move |_, window, cx| {
                    let (stops, at) = now(&remove_live, &remove_chosen, cx);
                    let (next, at) = removed(&stops, at);
                    remove_commit(next, at, window, cx);
                })
                .into_any_element()
        });
        let rail_area = div()
            .id((self.id.clone(), "touch"))
            .debug_selector(|| format!("{touch_debug}-hit"))
            .relative()
            .w_full()
            .h(rail_height)
            .child(rail.absolute().left(side).right(side).top(rail_top).h(knob))
            .when(theme.touch_mode, |area| {
                area.on_scroll(move |event, window, cx| {
                    if event.source != ScrollSource::Touch {
                        return;
                    }
                    let ScrollDelta::Pixels(delta) = event.delta else {
                        return;
                    };
                    if event.touch_phase == TouchPhase::Started {
                        let pending_start = match touch.read(cx).clone() {
                            Some(TouchState::Pending { start_x }) => Some(start_x),
                            _ => None,
                        };
                        touch.update(cx, |touch, _| *touch = None);
                        let pending = delta.x == Pixels::ZERO && delta.y == Pixels::ZERO;
                        if !pending && delta.x.abs() <= delta.y.abs() {
                            return;
                        }
                        let bounds = *touch_bar.read(cx);
                        let (stops, selected) = now(&touch_live, &touch_chosen, cx);
                        let origin_x = pending_start.unwrap_or(event.position.x - delta.x);
                        let Some(index) = grabbed(&stops, bounds, origin_x, selected, touch_hit)
                        else {
                            return;
                        };
                        if pending {
                            touch.update(cx, |touch, _| {
                                *touch = Some(TouchState::Pending {
                                    start_x: event.position.x,
                                })
                            });
                            return;
                        }
                        touch_chosen.update(cx, |chosen, cx| {
                            *chosen = index;
                            cx.notify();
                        });
                        window.focus(&touch_focus, cx);
                        touch.update(cx, |touch, _| {
                            *touch = Some(TouchState::Drag(TouchDrag {
                                x: event.position.x,
                                stops,
                                index,
                            }))
                        });
                    }
                    let Some(state) = touch.read(cx).clone() else {
                        return;
                    };
                    if matches!(state, TouchState::Momentum) {
                        cx.stop_propagation();
                        if matches!(event.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            touch.update(cx, |touch, _| *touch = None);
                        }
                        return;
                    }
                    let state = match state {
                        TouchState::Pending { start_x } => {
                            if event.touch_phase != TouchPhase::Moved {
                                touch.update(cx, |touch, _| *touch = None);
                                return;
                            }
                            if delta.x.abs() <= delta.y.abs() {
                                touch.update(cx, |touch, _| *touch = None);
                                return;
                            }
                            let bounds = *touch_bar.read(cx);
                            let (stops, selected) = now(&touch_live, &touch_chosen, cx);
                            let Some(index) = grabbed(&stops, bounds, start_x, selected, touch_hit)
                            else {
                                touch.update(cx, |touch, _| *touch = None);
                                return;
                            };
                            touch_chosen.update(cx, |chosen, cx| {
                                *chosen = index;
                                cx.notify();
                            });
                            window.focus(&touch_focus, cx);
                            let drag = TouchDrag {
                                x: start_x,
                                stops,
                                index,
                            };
                            touch.update(cx, |touch, _| {
                                *touch = Some(TouchState::Drag(drag.clone()))
                            });
                            TouchState::Drag(drag)
                        }
                        state => state,
                    };
                    let TouchState::Drag(mut drag) = state else {
                        unreachable!("pending and momentum handled above")
                    };
                    cx.stop_propagation();
                    match event.touch_phase {
                        TouchPhase::Moved => {
                            drag.x = event.position.x;
                            touch.update(cx, |touch, _| {
                                *touch = Some(TouchState::Drag(drag.clone()))
                            });
                        }
                        TouchPhase::Ended => {
                            drag.x = event.position.x;
                            touch.update(cx, |touch, _| *touch = Some(TouchState::Momentum));
                        }
                        TouchPhase::Cancelled => {
                            touch.update(cx, |touch, _| *touch = None);
                            let (stops, _) = now(&touch_live, &touch_chosen, cx);
                            if stops != drag.stops {
                                touch_commit(drag.stops, drag.index, window, cx);
                            }
                            return;
                        }
                        TouchPhase::Started => {}
                    }
                    let bounds = *touch_bar.read(cx);
                    let t = (drag.x - bounds.left()) / bounds.size.width;
                    let (stops, index) = now(&touch_live, &touch_chosen, cx);
                    let (next, index) = moved(&stops, index, t);
                    if next != stops {
                        touch_commit(next, index, window, cx);
                    }
                })
            });
        let (edit_live, edit_chosen, edit) = (live, chosen, commit);
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().px(side).child(strip))
                    .child(rail_area)
                    .children(
                        remove.map(|remove| div().px(side).flex().justify_end().child(remove)),
                    ),
            )
            .child(
                ColorPicker::new((self.id, "color"), stops[at].color).on_change(
                    move |color, window, cx| {
                        let (mut next, at) = now(&edit_live, &edit_chosen, cx);
                        next[at].color = color;
                        edit(next, at, window, cx);
                    },
                ),
            )
    }
}

#[cfg(test)]
#[path = "gradient_touch_tests.rs"]
mod touch_tests;

#[cfg(test)]
#[path = "gradient_tests.rs"]
mod tests;
