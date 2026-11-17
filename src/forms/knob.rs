use std::{cell::Cell, f32::consts::PI, rc::Rc};

use gpui::{
    App, DispatchPhase, DragMoveEvent, ElementId, EmptyView, EntityId, HitboxBehavior,
    InteractiveElement, IntoElement, ParentElement, Path, PathBuilder, Pixels, Point, RenderOnce,
    StatefulInteractiveElement, Styled, TouchDragEvent, TouchPhase, Window, canvas, div, point,
    prelude::*,
};

use super::{
    options::OnNumber,
    slider::{fraction, keyed, value_at},
};
use crate::{
    primitives::tab_stop,
    theme::{ActiveTheme, Elevation},
};

/// Where the dial starts, at lower left, and how far it turns.
const START: f32 = 0.75 * PI;
const SWEEP: f32 = 1.5 * PI;

fn at_angle(center: Point<Pixels>, radius: Pixels, angle: f32) -> Point<Pixels> {
    point(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

#[cfg(test)]
mod tests {
    use super::Knob;
    use crate::theme::Theme;
    use gpui::{
        Context, IntoElement, ParentElement, Render, Styled, TestAppContext, TouchDragEvent,
        TouchPhase, Window, div, point, px,
    };

    struct Probe {
        value: f64,
        disabled: bool,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity();
            div().size_full().child(
                Knob::new("knob", self.value)
                    .disabled(self.disabled)
                    .on_change(move |value, _, cx| {
                        view.update(cx, |view, cx| {
                            view.value = value;
                            cx.notify();
                        });
                    }),
            )
        }
    }

    #[gpui::test]
    fn touch_drag_turns_knob_and_disabled_knob_does_not_claim(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, _| Probe {
            value: 65.,
            disabled: false,
        });
        cx.run_until_parked();
        let start = cx.debug_bounds("knob-root").unwrap().center();
        let up = start - point(px(0.), px(40.));
        for (phase, position) in [
            (TouchPhase::Started, up),
            (TouchPhase::Moved, up),
            (TouchPhase::Ended, up),
        ] {
            cx.simulate_event(TouchDragEvent {
                phase,
                start_position: start,
                position,
            });
            cx.run_until_parked();
        }
        let changed = view.read_with(cx, |view, _| view.value);
        assert!(changed > 65., "drag up left value at {changed}");
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Moved,
            start_position: start,
            position: start,
        });
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |view, _| view.value), changed);
        view.update(cx, |view, cx| {
            view.disabled = true;
            cx.notify();
        });
        cx.run_until_parked();
        for phase in [TouchPhase::Started, TouchPhase::Moved, TouchPhase::Ended] {
            cx.simulate_event(TouchDragEvent {
                phase,
                start_position: start,
                position: up,
            });
            cx.run_until_parked();
        }
        assert_eq!(view.read_with(cx, |view, _| view.value), changed);
    }
}

/// A stroked arc clockwise from `from` to `to`, in radians.
fn arc(center: Point<Pixels>, radius: Pixels, from: f32, to: f32, width: Pixels) -> Path<Pixels> {
    let mut path = PathBuilder::stroke(width);
    path.move_to(at_angle(center, radius, from));
    path.arc_to(
        point(radius, radius),
        Pixels::ZERO,
        to - from > PI,
        true,
        at_angle(center, radius, to),
    );
    path.build().expect("an arc is a simple open path")
}

struct Turn {
    owner: EntityId,
    start: Rc<Cell<Option<(Pixels, f64)>>>,
}

/// A dial for audio-style values. Drag up or down, or use the arrow keys.
#[derive(IntoElement)]
pub struct Knob {
    id: ElementId,
    value: f64,
    range: (f64, f64, f64),
    disabled: bool,
    on_change: Option<OnNumber>,
}

impl Knob {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            range: (0.0, 100.0, 1.0),
            disabled: false,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(min < max, "knob range {min}..{max} is empty");
        self.range.0 = min;
        self.range.1 = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        assert!(step > 0.0, "knob step {step} must be positive");
        self.range.2 = step;
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

impl RenderOnce for Knob {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ((min, max, step), value) = (self.range, self.value);
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            !self.disabled,
            window,
            cx,
        );
        let focused = focus.is_focused(window);
        let owner = window
            .use_keyed_state((self.id.clone(), "drag"), cx, |_, _| ())
            .entity_id();
        let touch = window.use_keyed_state((self.id.clone(), "touch"), cx, |_, _| {
            None::<(Point<Pixels>, f64)>
        });
        let theme = cx.theme();
        let colors = &theme.colors;
        let (diameter, width) = theme.knob();
        let (travel, stroke) = (
            diameter.to_pixels(window.rem_size()) * 4.0,
            width.to_pixels(window.rem_size()),
        );
        let turned = fraction(value, min, max);
        let (rail, ink, tick) = (colors.border, colors.accent, colors.fg);
        let commit = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next: f64, window: &mut Window, cx: &mut App| {
                if next == value {
                    return;
                }
                log::debug!("knob {id:?}: {next}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let (drag, keys, touch_commit) = (commit.clone(), commit.clone(), commit);
        let touch_enabled = theme.touch_mode && !self.disabled;
        let touch_focus = focus.clone();
        div()
            .id(self.id)
            .debug_selector(|| "knob-root".into())
            .track_focus(&focus)
            .relative()
            .flex_none()
            .size(diameter)
            .rounded_full()
            .bg(colors.surface)
            .border_1()
            .border_color(if focused { colors.focus } else { colors.border })
            .shadow(theme.elevation(Elevation::Raised))
            .when(self.disabled, |knob| knob.opacity(0.5))
            .child(
                canvas(
                    move |bounds, window, _| {
                        touch_enabled.then(|| window.insert_hitbox(bounds, HitboxBehavior::Normal))
                    },
                    move |bounds, hitbox, window, _| {
                        if let Some(hitbox) = hitbox {
                            window.on_mouse_event(
                                move |event: &TouchDragEvent, phase, window, cx| {
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
                                        touch.update(cx, |state, _| {
                                            *state = Some((event.start_position, value))
                                        });
                                        window.focus(&touch_focus, cx);
                                        window.prevent_default();
                                    }
                                    let Some((start, from)) = *touch.read(cx) else {
                                        return;
                                    };
                                    if start != event.start_position {
                                        return;
                                    }
                                    cx.stop_propagation();
                                    if matches!(
                                        event.phase,
                                        TouchPhase::Ended | TouchPhase::Cancelled
                                    ) {
                                        touch.update(cx, |state, _| *state = None);
                                        return;
                                    }
                                    let moved = (start.y - event.position.y) / travel;
                                    touch_commit(
                                        value_at(fraction(from, min, max) + moved, min, max, step),
                                        window,
                                        cx,
                                    );
                                },
                            );
                        }
                        let center = bounds.center();
                        let radius = bounds.size.width / 2.0 - stroke * 1.5;
                        window.paint_path(arc(center, radius, START, START + SWEEP, stroke), rail);
                        if turned > 0.0 {
                            let to = START + SWEEP * turned;
                            window.paint_path(arc(center, radius, START, to, stroke), ink);
                        }
                        let angle = START + SWEEP * turned;
                        let mut line = PathBuilder::stroke(stroke);
                        line.move_to(at_angle(center, radius * 0.3, angle));
                        line.line_to(at_angle(center, radius * 0.62, angle));
                        window.paint_path(line.build().expect("a tick is a straight line"), tick);
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(!self.disabled, |knob| {
                knob.cursor_ns_resize()
                    .on_drag(
                        Turn {
                            owner,
                            start: Rc::new(Cell::new(None)),
                        },
                        |turn, _, _, cx| {
                            turn.start.set(None);
                            cx.new(|_| EmptyView)
                        },
                    )
                    .on_drag_move(move |event: &DragMoveEvent<Turn>, window, cx| {
                        let turn = event.drag(cx);
                        if turn.owner != owner {
                            return;
                        }
                        let (start, y) = (turn.start.clone(), event.event.position.y);
                        let (anchor, from) = start.get().unwrap_or_else(|| {
                            start.set(Some((y, value)));
                            (y, value)
                        });
                        let moved = (anchor - y) / travel;
                        drag(
                            value_at(fraction(from, min, max) + moved, min, max, step),
                            window,
                            cx,
                        );
                    })
                    .on_key_down(move |event, window, cx| {
                        if let Some(next) = keyed(&event.keystroke.key, value, min, max, step) {
                            cx.stop_propagation();
                            keys(next, window, cx);
                        }
                    })
            })
    }
}
