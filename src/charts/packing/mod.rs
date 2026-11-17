use std::f32::consts::TAU;

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels,
    Refineable, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, canvas, div,
};

use super::{
    axes::anchored,
    paint::{finish, measure, tint},
    parts::ChartTooltip,
    pie::toward,
    scale::compact,
    sunburst::{Slice, subtree},
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::format,
};

#[derive(Default)]
struct PackingState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    zoom_path: Vec<usize>,
}

#[derive(Clone, Debug)]
struct PackedCircle {
    cx: f32,
    cy: f32,
    r: f32,
    depth: usize,
    branch: usize,
    path: Vec<SharedString>,
    indices: Vec<usize>,
    is_branch: bool,
    weight: f64,
}

/// Hierarchical nested packed circles sharing [`Slice`] nodes with [`Sunburst`](super::Sunburst).
#[derive(IntoElement)]
pub struct CirclePacking {
    base: Div,
    id: ElementId,
    slices: Vec<Slice>,
}

impl CirclePacking {
    pub fn new(id: impl Into<ElementId>, slices: impl IntoIterator<Item = Slice>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            slices: slices.into_iter().collect(),
        }
    }
}

/// Pack sibling circles inside `(0, 0)` with bounding radius `outer` without overlap.
fn pack_siblings(radii: &[f32]) -> (Vec<(f32, f32, f32)>, f32) {
    if radii.is_empty() {
        return (Vec::new(), 1.0);
    }
    let mut placed: Vec<(f32, f32, f32)> = Vec::with_capacity(radii.len());
    let mut order: Vec<usize> = (0..radii.len()).collect();
    order.sort_by(|a, b| radii[*b].total_cmp(&radii[*a]));
    let mut coords = vec![(0.0_f32, 0.0_f32, 0.0_f32); radii.len()];

    for (step, &orig_ix) in order.iter().enumerate() {
        let r = radii[orig_ix].max(0.001);
        let (x, y) = if step == 0 {
            (0.0, 0.0)
        } else if step == 1 {
            (placed[0].2 + r, 0.0)
        } else {
            let mut best: Option<(f32, f32, f32)> = None;
            let candidates = 64usize;
            let ring_dist_max = placed
                .iter()
                .map(|c| c.0.hypot(c.1) + c.2)
                .fold(0.0_f32, f32::max)
                + r;
            for angle_ix in 0..candidates {
                let angle = TAU * angle_ix as f32 / candidates as f32;
                let (dx, dy) = (angle.cos(), angle.sin());
                let mut lo = 0.0_f32;
                let mut hi = ring_dist_max;
                for _ in 0..20 {
                    let mid = (lo + hi) * 0.5;
                    let (tx, ty) = (dx * mid, dy * mid);
                    let overlaps = placed.iter().any(|&(px, py, pr)| {
                        let d2 = (tx - px).powi(2) + (ty - py).powi(2);
                        d2 < (r + pr - 1e-4).powi(2)
                    });
                    if overlaps {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                let (tx, ty) = (dx * hi, dy * hi);
                let score = tx * tx + ty * ty;
                if best.is_none_or(|b| score < b.2) {
                    best = Some((tx, ty, score));
                }
            }
            best.map_or((0.0, 0.0), |(x, y, _)| (x, y))
        };
        placed.push((x, y, r));
        coords[orig_ix] = (x, y, r);
    }

    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for &(x, y, r) in &coords {
        min_x = min_x.min(x - r);
        max_x = max_x.max(x + r);
        min_y = min_y.min(y - r);
        max_y = max_y.max(y + r);
    }
    let (cx, cy) = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let mut bound = 0.001_f32;
    for c in &mut coords {
        c.0 -= cx;
        c.1 -= cy;
        bound = bound.max(c.0.hypot(c.1) + c.2);
    }
    (coords, bound)
}

fn pack_hierarchy(slices: &[Slice], center: (f32, f32), radius: f32) -> Vec<PackedCircle> {
    fn walk(
        slices: &[Slice],
        center: (f32, f32),
        radius: f32,
        (depth, branch): (usize, Option<usize>),
        (trail, index_trail): (&[SharedString], &[usize]),
        out: &mut Vec<PackedCircle>,
    ) {
        if slices.is_empty() || radius <= 0.5 {
            return;
        }
        let radii: Vec<f32> = slices
            .iter()
            .map(|s| (s.weight().max(1e-6) as f32).sqrt())
            .collect();
        let (local, bound) = pack_siblings(&radii);
        let scale = (radius * 0.94) / bound;
        for (ix, (slice, (lx, ly, lr))) in slices.iter().zip(local).enumerate() {
            let cx = center.0 + lx * scale;
            let cy = center.1 + ly * scale;
            let r = lr * scale;
            let branch = branch.unwrap_or(ix);
            let mut path = trail.to_vec();
            path.push(slice.name.clone());
            let mut indices = index_trail.to_vec();
            indices.push(ix);
            let is_branch = !slice.children.is_empty();
            out.push(PackedCircle {
                cx,
                cy,
                r,
                depth,
                branch,
                path: path.clone(),
                indices: indices.clone(),
                is_branch,
                weight: slice.weight(),
            });
            if is_branch {
                walk(
                    &slice.children,
                    (cx, cy),
                    r,
                    (depth + 1, Some(branch)),
                    (&path, &indices),
                    out,
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(slices, center, radius, (0, None), (&[], &[]), &mut out);
    out.sort_by_key(|c| c.depth);
    out
}

fn circle_under(circles: &[PackedCircle], pos: (f32, f32)) -> Option<usize> {
    circles
        .iter()
        .enumerate()
        .rev()
        .find(|(_, c)| (pos.0 - c.cx).hypot(pos.1 - c.cy) <= c.r)
        .map(|(ix, _)| ix)
}

fn circle_hit(circles: &[PackedCircle], pos: (f32, f32), touch_mode: bool) -> Option<usize> {
    circle_under(circles, pos).or_else(|| {
        touch_mode.then(|| {
            circles
                .iter()
                .enumerate()
                .map(|(ix, c)| (ix, (pos.0 - c.cx).hypot(pos.1 - c.cy)))
                .filter(|(_, dist)| *dist <= 24.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(ix, _)| ix)
        })?
    })
}

impl Styled for CirclePacking {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CirclePacking {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<PackingState> =
            window.use_keyed_state((self.id.clone(), "packing"), cx, |_, _| {
                PackingState::default()
            });
        let (active, crumbs) = subtree(&self.slices, &state.read(cx).zoom_path);
        let valid_depth = crumbs.len();
        let bounds = state.read(cx).bounds;
        let center = (
            f32::from(bounds.size.width) * 0.5,
            f32::from(bounds.size.height) * 0.5,
        );
        let outer = (center.0.min(center.1) * 0.92).max(0.0);
        let circles = pack_hierarchy(active, center, outer);
        let total: f64 = active.iter().map(Slice::weight).sum();
        let hover = state.read(cx).hover.filter(|ix| *ix < circles.len());
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let box_w = f32::from(bounds.size.width);

        let tooltip = hover.map(|ix| {
            let c = &circles[ix];
            let share = if total > 0.0 { c.weight / total } else { 0.0 };
            let card = ChartTooltip::new(c.path.join(" › "))
                .row(Some(tint(&colors, c.branch)), "Value", compact(c.weight))
                .row(None, "Share", format::percent(share, 1, false));
            let pos = (c.cx + c.r, c.cy);
            anchored(pos, pos.0 > box_w * 0.55, card)
        });

        let (circles_move, circles_down, circles_up) =
            (circles.clone(), circles.clone(), circles.clone());
        let (moved, pressed, released, left) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);
        let controls = (!crumbs.is_empty()).then(|| {
            let up_state = state.clone();
            let label = crumbs.join(" › ");
            let crumb_max = if box_w > 0.0 {
                (box_w - 84.0).clamp(48.0, 160.0)
            } else {
                160.0
            };
            let button = Button::new((self.id.clone(), "up"), "Up")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .debug_selector(|| "packing-up".into())
                .on_click(move |_, _, cx| {
                    up_state.update(cx, |state, cx| {
                        let depth = valid_depth.min(state.zoom_path.len());
                        state.zoom_path.truncate(depth.saturating_sub(1));
                        state.hover = None;
                        cx.notify();
                    })
                });
            div()
                .debug_selector(|| "packing-breadcrumb".into())
                .absolute()
                .top_1()
                .right_1()
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .flex()
                .items_center()
                .gap_1()
                .pl_2()
                .rounded(theme.radius(Radius::Sm))
                .bg(colors.bg.opacity(0.85))
                .child(
                    div()
                        .min_w_0()
                        .max_w(Pixels::from(crumb_max))
                        .truncate()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(label),
                )
                .child(button)
        });
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        root.child(
            div()
                .id((self.id.clone(), "packing"))
                .absolute()
                .inset_0()
                .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                    let offset = event.position - moved.read(cx).bounds.origin;
                    let next = circle_hit(
                        &circles_move,
                        (f32::from(offset.x), f32::from(offset.y)),
                        touch_mode,
                    );
                    moved.update(cx, |state, cx| {
                        if state.hover != next {
                            state.hover = next;
                            cx.notify();
                        }
                    })
                })
                .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                    if event.click_count >= 2 {
                        return;
                    }
                    let offset = event.position - pressed.read(cx).bounds.origin;
                    let next = circle_hit(
                        &circles_down,
                        (f32::from(offset.x), f32::from(offset.y)),
                        touch_mode,
                    );
                    pressed.update(cx, |state, cx| {
                        if state.hover != next {
                            state.hover = next;
                            cx.notify();
                        }
                    })
                })
                .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                    if event.click_count >= 2 {
                        released.update(cx, |state, cx| {
                            if !state.zoom_path.is_empty() {
                                state.zoom_path.clear();
                                state.hover = None;
                                cx.notify();
                            }
                        });
                        return;
                    }
                    let offset = event.position - released.read(cx).bounds.origin;
                    let hit = circle_hit(
                        &circles_up,
                        (f32::from(offset.x), f32::from(offset.y)),
                        touch_mode,
                    );
                    released.update(cx, |state, cx| match hit {
                        Some(ix) if circles_up[ix].is_branch => {
                            let depth = valid_depth.min(state.zoom_path.len());
                            state.zoom_path.truncate(depth);
                            state.zoom_path.extend_from_slice(&circles_up[ix].indices);
                            state.hover = None;
                            cx.notify();
                        }
                        None if valid_depth > 0 => {
                            let depth = valid_depth.min(state.zoom_path.len());
                            state.zoom_path.truncate(depth.saturating_sub(1));
                            state.hover = None;
                            cx.notify();
                        }
                        _ => {}
                    })
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
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let origin = bounds.origin;
                            for (ix, c) in circles.iter().enumerate() {
                                if c.r <= 0.5 {
                                    continue;
                                }
                                let center_pt =
                                    origin + gpui::point(Pixels::from(c.cx), Pixels::from(c.cy));
                                let segs = ((c.r * 1.4).ceil() as usize).clamp(16, 48);
                                let mut fill_path = PathBuilder::fill();
                                let mut edge_path = PathBuilder::stroke(if hover == Some(ix) {
                                    stroke
                                } else {
                                    hairline
                                });
                                fill_path.move_to(toward(center_pt, c.r, 0.0));
                                edge_path.move_to(toward(center_pt, c.r, 0.0));
                                for s in 1..=segs {
                                    let pt = toward(center_pt, c.r, TAU * s as f32 / segs as f32);
                                    fill_path.line_to(pt);
                                    edge_path.line_to(pt);
                                }
                                fill_path.close();
                                edge_path.close();
                                let ink = tint(&colors, c.branch);
                                let alpha = if hover == Some(ix) { 0.42 } else { 0.18 };
                                finish(fill_path, ink.opacity(alpha), window);
                                finish(
                                    edge_path,
                                    if hover == Some(ix) {
                                        colors.fg
                                    } else {
                                        ink.opacity(0.65)
                                    },
                                    window,
                                );
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
                .children(tooltip)
                .child(measure(state, |state| &mut state.bounds)),
        )
        .children(controls)
    }
}

#[cfg(test)]
mod tests;
