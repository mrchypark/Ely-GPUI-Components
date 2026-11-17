use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, DispatchPhase, DragMoveEvent, ElementId, EmptyView, EntityId,
    HitboxBehavior, InteractiveElement, IntoElement, MouseButton, Orientation, ParentElement,
    Pixels, Point, RenderOnce, Role, StatefulInteractiveElement, Styled, TouchDragEvent,
    TouchPhase, Window, canvas, div, prelude::*, relative,
};

use super::options::OnNumber;
use crate::{
    i18n,
    primitives::tab_stop,
    theme::{ActiveTheme, Elevation},
};

/// `value` as a fraction of `min..=max`.
#[path = "slider_math.rs"]
mod math;

pub(crate) use math::{along, fraction, keyed, value_at};

/// A drag owned by this track; the press selects its thumb.
struct Thumb {
    owner: EntityId,
}

/// The value at the start of a touch drag and the last stepped value it emitted.
#[derive(Clone)]
struct TouchDrag {
    start: Point<Pixels>,
    origin: f64,
    value: f64,
}

pub(super) type Commit = Rc<dyn Fn(usize, f64, bool, &mut Window, &mut App)>;
pub(super) type OnSpan = Rc<dyn Fn((f64, f64), &mut Window, &mut App)>;

/// A track with one or two thumbs. `commit` gets the thumb and its new value.
pub(super) struct Track {
    id: ElementId,
    values: Vec<f64>,
    range: (f64, f64, f64),
    vertical: bool,
    disabled: bool,
    commit: Commit,
}

impl Track {
    fn render(self, window: &mut Window, cx: &mut App) -> AnyElement {
        let (min, max, step) = self.range;
        let vertical = self.vertical;
        let owner = window
            .use_keyed_state((self.id.clone(), "drag"), cx, |_, _| ())
            .entity_id();
        let grabbed = window.use_keyed_state((self.id.clone(), "grab"), cx, |_, _| 0usize);
        let tied = window.use_keyed_state((self.id.clone(), "tie"), cx, |_, _| None::<f64>);
        let touch =
            window.use_keyed_state((self.id.clone(), "touch"), cx, |_, _| None::<TouchDrag>);
        let measured = window.use_keyed_state((self.id.clone(), "bounds"), cx, |_, _| {
            Bounds::<Pixels>::default()
        });
        let handles: Vec<_> = (0..self.values.len())
            .map(|k| {
                tab_stop(
                    (self.id.clone(), ["low", "high"][k]).into(),
                    !self.disabled,
                    window,
                    cx,
                )
            })
            .collect();
        let unavailable = self
            .disabled
            .then(|| i18n::text(cx, "state.unavailable", &[]));
        let orientation = match vertical {
            true => Orientation::Vertical,
            false => Orientation::Horizontal,
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (thumb, line) = (theme.slider_thumb(), theme.slider_track());
        let hit = theme.touch_minimum(thumb.to_pixels(window.rem_size()));
        let fractions: Vec<f32> = self
            .values
            .iter()
            .map(|value| fraction(*value, min, max))
            .collect();
        let (start, end) = if fractions.len() == 2 {
            (fractions[0], fractions[1])
        } else {
            (0.0, fractions[0])
        };
        let rail = div()
            .absolute()
            .rounded_full()
            .bg(colors.border)
            .map(|rail| {
                if vertical {
                    rail.top_0()
                        .bottom_0()
                        .left(relative(0.5))
                        .ml(line * -0.5)
                        .w(line)
                } else {
                    rail.left_0()
                        .right_0()
                        .top(relative(0.5))
                        .mt(line * -0.5)
                        .h(line)
                }
            });
        let fill = div()
            .absolute()
            .rounded_full()
            .bg(colors.accent)
            .map(|fill| {
                if vertical {
                    fill.bottom(relative(start))
                        .h(relative(end - start))
                        .left(relative(0.5))
                        .ml(line * -0.5)
                        .w(line)
                } else {
                    fill.left(relative(start))
                        .w(relative(end - start))
                        .top(relative(0.5))
                        .mt(line * -0.5)
                        .h(line)
                }
            });
        let thumbs: Vec<_> = fractions
            .iter()
            .enumerate()
            .map(|(k, at)| {
                let focused = handles[k].is_focused(window);
                let (commit, value) = (self.commit.clone(), self.values[k]);
                let (floor, ceiling) = match (self.values.as_slice(), k) {
                    ([_, high], 0) => (min, *high),
                    ([low, _], _) => (*low, max),
                    _ => (min, max),
                };
                div()
                    .id(("thumb", k))
                    .debug_selector(move || format!("slider-thumb-{k}"))
                    .role(Role::Slider)
                    .aria_numeric_value(value)
                    .aria_min_numeric_value(floor)
                    .aria_max_numeric_value(ceiling)
                    .aria_numeric_value_step(step)
                    .aria_orientation(orientation)
                    .when_some(unavailable.clone(), |thumb, text| {
                        thumb.aria_description(text)
                    })
                    .track_focus(&handles[k])
                    .absolute()
                    .size(hit)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex_none()
                            .size(thumb)
                            .rounded_full()
                            .bg(colors.surface)
                            .border_1()
                            .border_color(if focused {
                                colors.focus
                            } else {
                                colors.border_strong
                            })
                            .shadow(theme.elevation(Elevation::Raised)),
                    )
                    .map(|thumb_box| {
                        if vertical {
                            thumb_box.left_0().bottom(relative(*at)).mb(hit * -0.5)
                        } else {
                            thumb_box.top_0().left(relative(*at)).ml(hit * -0.5)
                        }
                    })
                    .when(!self.disabled, |thumb_box| {
                        thumb_box
                            .cursor_pointer()
                            .on_drag(Thumb { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                            .on_key_down(move |event, window, cx| {
                                if let Some(next) =
                                    keyed(&event.keystroke.key, value, min, max, step)
                                {
                                    cx.stop_propagation();
                                    commit(k, next, false, window, cx);
                                }
                            })
                    })
            })
            .collect();
        let (drag, jump) = (self.commit.clone(), self.commit);
        let (values, focus) = (self.values, handles);
        let touch_layer = (theme.touch_mode && !self.disabled).then(|| {
            let (values, focus, commit) = (values.clone(), focus.clone(), jump.clone());
            let (measured, grabbed, tied) = (measured.clone(), grabbed.clone(), tied.clone());
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |_, hitbox, window, _| {
                    window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        if event.phase == TouchPhase::Started {
                            touch.update(cx, |state, _| *state = None);
                            if window.default_prevented()
                                || !hitbox.is_hovered_at(event.start_position, window)
                            {
                                return;
                            }
                            let start = value_at(
                                along(*measured.read(cx), event.start_position, vertical),
                                min,
                                max,
                                step,
                            );
                            let index = match values.as_slice() {
                                [low, high] if low == high && start > *high => 1,
                                [low, high] if (start - high).abs() < (start - low).abs() => 1,
                                _ => 0,
                            };
                            let tie = match values.as_slice() {
                                [low, high] if low == high && start == *low => Some(*low),
                                _ => None,
                            };
                            grabbed.update(cx, |state, _| *state = index);
                            tied.update(cx, |state, _| *state = tie);
                            touch.update(cx, |state, _| {
                                *state = Some(TouchDrag {
                                    start: event.start_position,
                                    origin: values[index],
                                    value: values[index],
                                })
                            });
                            window.focus(&focus[index], cx);
                            window.prevent_default();
                        }
                        let Some(drag) = touch.read(cx).clone() else {
                            return;
                        };
                        if drag.start != event.start_position {
                            return;
                        }
                        cx.stop_propagation();
                        if event.phase == TouchPhase::Cancelled {
                            let index = *grabbed.read(cx);
                            if drag.value != drag.origin {
                                commit(index, drag.origin, true, window, cx);
                            }
                            touch.update(cx, |state, _| *state = None);
                            tied.update(cx, |state, _| *state = None);
                            return;
                        }
                        let ended = event.phase == TouchPhase::Ended;
                        if ended {
                            touch.update(cx, |state, _| *state = None);
                        }
                        let next = value_at(
                            along(*measured.read(cx), event.position, vertical),
                            min,
                            max,
                            step,
                        );
                        if let Some(value) = *tied.read(cx) {
                            if next == value {
                                if ended {
                                    tied.update(cx, |state, _| *state = None);
                                }
                                return;
                            }
                            let index = usize::from(next > value);
                            grabbed.update(cx, |state, _| *state = index);
                            tied.update(cx, |state, _| *state = None);
                            window.focus(&focus[index], cx);
                        }
                        let index = *grabbed.read(cx);
                        let value = match (values.as_slice(), index) {
                            ([_, high], 0) => next.min(*high),
                            ([low, _], 1) => next.max(*low),
                            _ => next,
                        };
                        if value == drag.value {
                            return;
                        }
                        commit(index, value, true, window, cx);
                        if !ended {
                            touch.update(cx, |state, _| {
                                if let Some(drag) = state {
                                    drag.value = value;
                                }
                            });
                        } else {
                            tied.update(cx, |state, _| *state = None);
                        }
                    });
                },
            )
            .absolute()
            .inset_0()
        });
        let bounds_state = measured.clone();
        let grab = grabbed.clone();
        let press_tie = tied.clone();
        let inner = div()
            .id((self.id.clone(), "inner"))
            .debug_selector(|| "slider-track".into())
            .relative()
            .size_full()
            .child(rail)
            .child(fill)
            .children(thumbs)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if *bounds_state.read(cx) != bounds {
                            bounds_state.update(cx, |state, _| *state = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(!self.disabled, |inner| {
                let drag_focus = focus.clone();
                inner
                    .on_drag(Thumb { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                    .on_drag_move(move |event: &DragMoveEvent<Thumb>, window, cx| {
                        let thumb = event.drag(cx);
                        if thumb.owner != owner {
                            return;
                        }
                        let next = value_at(
                            along(event.bounds, event.event.position, vertical),
                            min,
                            max,
                            step,
                        );
                        if let Some(value) = *tied.read(cx) {
                            if next == value {
                                return;
                            }
                            let index = usize::from(next > value);
                            grabbed.update(cx, |grab, _| *grab = index);
                            tied.update(cx, |tie, _| *tie = None);
                            window.focus(&drag_focus[index], cx);
                        }
                        let index = *grabbed.read(cx);
                        drag(index, next, false, window, cx);
                    })
            });
        div()
            .debug_selector(|| "slider-root".into())
            .id(self.id)
            .relative()
            .flex_none()
            .map(|outer| {
                if vertical {
                    outer.h_full().w(hit).py(hit * 0.5)
                } else {
                    outer.h(hit).px(hit * 0.5)
                }
            })
            .when(!self.disabled, |outer| {
                outer.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    let bounds = *measured.read(cx);
                    let next = value_at(along(bounds, event.position, vertical), min, max, step);
                    let index = match values.as_slice() {
                        [low, high] if low == high && next > *high => 1,
                        [low, high] if (next - high).abs() < (next - low).abs() => 1,
                        _ => 0,
                    };
                    let tie = match values.as_slice() {
                        [low, high] if low == high && next == *low => Some(*low),
                        _ => None,
                    };
                    press_tie.update(cx, |pending, _| *pending = tie);
                    grab.update(cx, |grab, _| *grab = index);
                    window.focus(&focus[index], cx);
                    jump(index, next, false, window, cx);
                })
            })
            .when(self.disabled, |outer| outer.opacity(0.5))
            .children(touch_layer)
            .child(inner)
            .into_any_element()
    }
}

/// A value along a track. Drag the thumb, click the track, or use the arrow keys.
#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f64,
    range: (f64, f64, f64),
    vertical: bool,
    disabled: bool,
    on_change: Option<OnNumber>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            range: (0.0, 100.0, 1.0),
            vertical: false,
            disabled: false,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(min < max, "slider range {min}..{max} is empty");
        self.range.0 = min;
        self.range.1 = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        assert!(step > 0.0, "slider step {step} must be positive");
        self.range.2 = step;
        self
    }

    /// Runs bottom to top; the parent gives it a height.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, value, on_change) = (self.id.clone(), self.value, self.on_change);
        let commit: Commit = Rc::new(move |_, next, force, window, cx| {
            if !force && next == value {
                return;
            }
            log::debug!("slider {id:?}: {next}");
            if let Some(on_change) = &on_change {
                on_change(next, window, cx);
            }
        });
        Track {
            id: self.id,
            values: vec![value],
            range: self.range,
            vertical: self.vertical,
            disabled: self.disabled,
            commit,
        }
        .render(window, cx)
    }
}

#[path = "slider_range.rs"]
mod range;

pub use range::RangeSlider;

#[cfg(test)]
#[path = "slider_tests.rs"]
mod tests;
