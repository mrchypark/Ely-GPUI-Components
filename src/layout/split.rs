use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Axis, Bounds, CursorStyle, DispatchPhase, DragMoveEvent, ElementId, EmptyView,
    EntityId, HitboxBehavior, Hsla, InteractiveElement, IntoElement, Length, ParentElement, Pixels,
    Point, RenderOnce, Stateful, StatefulInteractiveElement, Styled, TouchDragEvent, TouchPhase,
    Window, canvas, div, prelude::*,
};

use super::seeded::use_seeded;
use crate::theme::ActiveTheme;

/// Thin divider with a wide grab area. `axis` is the split's direction.
pub fn resize_handle(id: impl Into<ElementId>, axis: Axis, cx: &App) -> Stateful<gpui::Div> {
    handle(id, axis, cx.theme().colors.border, "split-handle", cx)
}

/// A resize handle whose line shows only while hovered, on an edge whose box draws its own border.
pub fn resize_edge(id: impl Into<ElementId>, axis: Axis, cx: &App) -> Stateful<gpui::Div> {
    let selector = match axis {
        Axis::Horizontal => "resize-edge-horizontal",
        Axis::Vertical => "resize-edge-vertical",
    };
    handle(id, axis, gpui::transparent_black(), selector, cx)
}

fn handle(
    id: impl Into<ElementId>,
    axis: Axis,
    line: Hsla,
    selector: &'static str,
    cx: &App,
) -> Stateful<gpui::Div> {
    let theme = cx.theme();
    let (lit, hit) = (theme.colors.focus, theme.handle_hit());
    // A touch has no hover, so the line is lit and the grab area is a thumb.
    let hit: Length = if theme.touch_mode {
        theme.touch_minimum(Pixels::ZERO).into()
    } else {
        hit.into()
    };
    let border = if theme.touch_mode { lit } else { line };
    let base = div()
        .id(id)
        .group("resize")
        .debug_selector(|| selector.into())
        .flex_none()
        .relative()
        .flex()
        .justify_center()
        .items_center();
    match axis {
        Axis::Horizontal => base
            .w(hit)
            .h_full()
            .cursor(CursorStyle::ResizeLeftRight)
            .child(
                div()
                    .h_full()
                    .debug_selector(|| "split-handle-line".into())
                    .border_l_1()
                    .border_color(border)
                    .when(!theme.touch_mode, |line| {
                        line.group_hover("resize", |s| s.border_color(lit))
                    }),
            ),
        Axis::Vertical => base
            .h(hit)
            .w_full()
            .cursor(CursorStyle::ResizeUpDown)
            .child(
                div()
                    .w_full()
                    .debug_selector(|| "split-handle-line".into())
                    .border_t_1()
                    .border_color(border)
                    .when(!theme.touch_mode, |line| {
                        line.group_hover("resize", |s| s.border_color(lit))
                    }),
            ),
    }
}

struct DividerDrag {
    owner: EntityId,
    ix: usize,
    start: Vec<f32>,
    anchor: Rc<Cell<Option<Pixels>>>,
}

#[derive(Clone)]
struct DividerTouch {
    start: Point<Pixels>,
    sizes: Vec<f32>,
}

/// Moves divider `ix` by `delta` of the whole, keeping each pane at `floor` or more.
fn shift(start: &[f32], ix: usize, delta: f32, floor: f32) -> Vec<f32> {
    let pair = start[ix] + start[ix + 1];
    let floor = floor.min(pair / 2.0);
    let left = (start[ix] + delta).clamp(floor, pair - floor);
    let mut next = start.to_vec();
    next[ix] = left;
    next[ix + 1] = pair - left;
    next
}

/// Weights are finite and non-negative, with a finite, positive sum.
pub(crate) fn check_weights(weights: &[f32]) -> anyhow::Result<f32> {
    anyhow::ensure!(
        weights
            .iter()
            .all(|weight| weight.is_finite() && *weight >= 0.0),
        "split weights {weights:?} must be finite and non-negative"
    );
    let total: f32 = weights.iter().sum();
    anyhow::ensure!(
        total.is_finite() && total > 0.0,
        "split weights {weights:?} need a finite, positive sum"
    );
    Ok(total)
}

type OnResize = Rc<dyn Fn(&[f32], &mut Window, &mut App)>;

/// Panes side by side or stacked, split by draggable handles.
#[derive(IntoElement)]
pub struct SplitPane {
    id: ElementId,
    axis: Axis,
    panes: Vec<AnyElement>,
    sizes: Option<Vec<f32>>,
    min: Pixels,
    on_resize: Option<OnResize>,
}

impl SplitPane {
    /// `min` is the smallest a pane may shrink to.
    pub fn new(id: impl Into<ElementId>, axis: Axis, min: Pixels) -> Self {
        Self {
            id: id.into(),
            axis,
            panes: Vec::new(),
            sizes: None,
            min,
            on_resize: None,
        }
    }

    pub fn pane(mut self, pane: impl IntoElement) -> Self {
        self.panes.push(pane.into_any_element());
        self
    }

    /// Shares, one per pane, normalized. A new value replaces dragged ones.
    pub fn sizes(mut self, sizes: &[f32]) -> Self {
        self.sizes = Some(sizes.to_vec());
        self
    }

    /// Reports shares after each drag step.
    pub fn on_resize(mut self, handler: impl Fn(&[f32], &mut Window, &mut App) + 'static) -> Self {
        self.on_resize = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SplitPane {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.panes.len();
        assert!(count >= 2, "split pane {:?} needs two panes", self.id);
        let even = vec![1.0 / count as f32; count];
        let normalized = match self.sizes.clone() {
            None => even,
            Some(weights) if weights.len() != count => {
                log::error!(
                    "split pane {:?}: {} sizes for {count} panes; even shares",
                    self.id,
                    weights.len()
                );
                even
            }
            Some(weights) => match check_weights(&weights) {
                Ok(total) => weights.iter().map(|weight| weight / total).collect(),
                Err(error) => {
                    log::error!("split pane {:?}: {error:#}; even shares", self.id);
                    even
                }
            },
        };
        let seed = (self.sizes, normalized);
        let fractions = use_seeded(self.id.clone(), seed, window, cx);
        let owner = fractions.entity_id();
        let shares = fractions.read(cx).value.1.clone();
        let (axis, min) = (self.axis, self.min);
        let measured = window.use_keyed_state((self.id.clone(), "bounds"), cx, |_, _| {
            Bounds::<Pixels>::default()
        });
        type Resize = Rc<dyn Fn(&[f32], usize, Pixels, Pixels, &mut Window, &mut App)>;
        let resize: Resize = Rc::new({
            let fractions = fractions.clone();
            let on_resize = self.on_resize;
            move |start, ix, delta, extent, window, cx| {
                let next = shift(start, ix, delta / extent, min / extent);
                if next.as_slice() == start {
                    return;
                }
                if let Some(report) = &on_resize {
                    report(&next, window, cx);
                }
                fractions.update(cx, |shares, cx| {
                    shares.value.1 = next;
                    cx.notify();
                });
            }
        });
        let split_id = self.id.clone();
        let drag_resize = resize.clone();
        let mut root = div()
            .id(self.id)
            .relative()
            .size_full()
            .flex()
            .on_drag_move(move |event: &DragMoveEvent<DividerDrag>, window, cx| {
                let (start, ix, anchor_state) = {
                    let drag = event.drag(cx);
                    if drag.owner != owner {
                        return;
                    }
                    if drag.start.len() != count {
                        log::error!(
                            "split pane: a drag over {} panes ends at {count}",
                            drag.start.len()
                        );
                        return;
                    }
                    (drag.start.clone(), drag.ix, drag.anchor.clone())
                };
                let (along, extent) = match axis {
                    Axis::Horizontal => (event.event.position.x, event.bounds.size.width),
                    Axis::Vertical => (event.event.position.y, event.bounds.size.height),
                };
                let anchor = match anchor_state.get() {
                    Some(anchor) => anchor,
                    None => {
                        anchor_state.set(Some(along));
                        along
                    }
                };
                drag_resize(&start, ix, along - anchor, extent, window, cx);
            });
        let measure = measured.clone();
        root = root.child(
            canvas(
                move |next_bounds, _, cx| {
                    if *measure.read(cx) != next_bounds {
                        measure.update(cx, |bounds, _| *bounds = next_bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        );
        root = match axis {
            Axis::Horizontal => root.flex_row(),
            Axis::Vertical => root.flex_col(),
        };
        let mut children = Vec::with_capacity(count * 2);
        for (ix, pane) in self.panes.into_iter().enumerate() {
            if ix > 0 {
                let drag = DividerDrag {
                    owner,
                    ix: ix - 1,
                    start: shares.clone(),
                    anchor: Rc::new(Cell::new(None)),
                };
                let touch = window.use_keyed_state(
                    (split_id.clone(), format!("touch-{ix}")),
                    cx,
                    |_, _| None::<DividerTouch>,
                );
                let (touch_sizes, touch_measured, touch_resize) =
                    (shares.clone(), measured.clone(), resize.clone());
                let touch_layer = canvas(
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
                                touch.update(cx, |state, _| {
                                    *state = Some(DividerTouch {
                                        start: event.start_position,
                                        sizes: touch_sizes.clone(),
                                    });
                                });
                                window.prevent_default();
                                cx.stop_propagation();
                                return;
                            }
                            let Some(drag) = touch.read(cx).clone() else {
                                return;
                            };
                            if drag.start != event.start_position {
                                return;
                            }
                            cx.stop_propagation();
                            if event.phase == TouchPhase::Cancelled {
                                touch.update(cx, |state, _| *state = None);
                                return;
                            }
                            let ended = event.phase == TouchPhase::Ended;
                            if ended {
                                touch.update(cx, |state, _| *state = None);
                            }
                            let bounds = *touch_measured.read(cx);
                            let (along, start, extent) = match axis {
                                Axis::Horizontal => {
                                    (event.position.x, drag.start.x, bounds.size.width)
                                }
                                Axis::Vertical => {
                                    (event.position.y, drag.start.y, bounds.size.height)
                                }
                            };
                            touch_resize(&drag.sizes, ix - 1, along - start, extent, window, cx);
                        });
                    },
                )
                .absolute()
                .inset_0();
                children.push(
                    resize_handle(("divider", ix), axis, cx)
                        .on_drag(drag, |_, _, _, cx| cx.new(|_| EmptyView))
                        .child(touch_layer)
                        .into_any_element(),
                );
            }
            let mut cell = div()
                .relative()
                .overflow_hidden()
                .flex_basis(gpui::relative(0.0));
            cell.style().flex_grow = Some(shares[ix]);
            children.push(cell.child(pane).into_any_element());
        }
        root.children(children)
    }
}

#[cfg(test)]
mod tests {
    use super::{check_weights, shift};

    #[test]
    fn weights_must_be_finite_non_negative_and_sum_positive() {
        assert_eq!(check_weights(&[1.0, 3.0]).unwrap(), 4.0);
        assert_eq!(check_weights(&[0.0, 1.0]).unwrap(), 1.0);
        assert!(check_weights(&[0.0, 0.0]).is_err());
        assert!(check_weights(&[-1.0, 2.0]).is_err());
        assert!(check_weights(&[f32::NAN, 1.0]).is_err());
        assert!(check_weights(&[f32::MAX, f32::MAX]).is_err());
    }

    #[test]
    fn shift_moves_one_boundary_and_respects_the_floor() {
        let moved = shift(&[0.5, 0.5], 0, 0.1, 0.1);
        assert!((moved[0] - 0.6).abs() < 1e-6 && (moved[1] - 0.4).abs() < 1e-6);
        let floored = shift(&[0.5, 0.5], 0, -0.9, 0.2);
        assert!((floored[0] - 0.2).abs() < 1e-6);
        let three = shift(&[0.2, 0.3, 0.5], 1, 0.1, 0.05);
        assert!((three[1] - 0.4).abs() < 1e-6 && (three[2] - 0.4).abs() < 1e-6);
        assert!((three[0] - 0.2).abs() < 1e-6);
    }
}
