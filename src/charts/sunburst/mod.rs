use std::f32::consts::{FRAC_PI_2, TAU};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Refineable,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas,
    div,
};

use super::{
    paint::{finish, measure, tint},
    pie::{bearing, wedge},
    scale::compact,
    series::drawable,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{format, tabular},
};

/// A node of a sunburst: its name, its own value, and the nodes inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    pub(crate) name: SharedString,
    pub(crate) value: f64,
    pub(crate) children: Vec<Slice>,
}

impl Slice {
    pub fn new(name: impl Into<SharedString>, value: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0,
            "a slice needs a value of zero or more, within charts::LIMIT"
        );
        Self {
            name: name.into(),
            value,
            children: Vec::new(),
        }
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Slice>) -> Self {
        self.children.extend(children);
        self
    }

    /// Its value, or its children's total when that is larger.
    pub(crate) fn weight(&self) -> f64 {
        self.value
            .max(self.children.iter().map(Slice::weight).sum())
    }
}

/// Walks zoom_path through branch slices, returning the active root slices and breadcrumb names.
pub(crate) fn subtree<'a>(
    slices: &'a [Slice],
    zoom_path: &[usize],
) -> (&'a [Slice], Vec<SharedString>) {
    let mut current = slices;
    let mut crumbs = Vec::new();
    for &ix in zoom_path {
        let Some(node) = current.get(ix) else {
            break;
        };
        if node.children.is_empty() {
            break;
        }
        crumbs.push(node.name.clone());
        current = &node.children;
    }
    (current, crumbs)
}

/// An arc of a sunburst: its ring, start and sweep, the top branch it belongs to, and its path of names.
#[derive(Clone, Debug, PartialEq)]
struct Arc {
    depth: usize,
    start: f32,
    sweep: f32,
    branch: usize,
    path: Vec<SharedString>,
    indices: Vec<usize>,
    is_branch: bool,
    weight: f64,
}

/// Arcs for a hierarchy, each child inside its parent's sweep.
fn rings(slices: &[Slice]) -> Vec<Arc> {
    fn walk(
        slices: &[Slice],
        (depth, start, sweep, whole): (usize, f32, f32, f64),
        branch: Option<usize>,
        (trail, index_trail): (&[SharedString], &[usize]),
        out: &mut Vec<Arc>,
    ) {
        let mut at = start;
        for (ix, slice) in slices.iter().enumerate() {
            let part = if whole > 0.0 {
                (slice.weight() / whole) as f32 * sweep
            } else {
                0.0
            };
            let (branch, mut path, mut indices) =
                (branch.unwrap_or(ix), trail.to_vec(), index_trail.to_vec());
            path.push(slice.name.clone());
            indices.push(ix);
            walk(
                &slice.children,
                (depth + 1, at, part, slice.weight()),
                Some(branch),
                (&path, &indices),
                out,
            );
            out.push(Arc {
                depth,
                start: at,
                sweep: part,
                branch,
                path,
                indices,
                is_branch: !slice.children.is_empty(),
                weight: slice.weight(),
            });
            at += part;
        }
    }
    let mut out = Vec::new();
    let whole = slices.iter().map(Slice::weight).sum();
    walk(
        slices,
        (0, -FRAC_PI_2, TAU, whole),
        None,
        (&[], &[]),
        &mut out,
    );
    out.sort_by_key(|arc| arc.depth);
    out
}

/// The arc under a pointer at `position` in a chart box `bounds`: the ring it lands on and the arc within it, if any.
fn arc_under(
    arcs: &[Arc],
    bounds: gpui::Bounds<Pixels>,
    position: gpui::Point<Pixels>,
    hole: f32,
    band: f32,
    outer: f32,
) -> Option<Option<usize>> {
    let center = bounds.center();
    let at = (f32::from(position.x), f32::from(position.y));
    let (angle, distance) = bearing((f32::from(center.x), f32::from(center.y)), at);
    let depth = ((distance - hole) / band).floor();
    (depth >= 0.0 && distance <= outer).then(|| {
        arcs.iter().position(|arc| {
            arc.depth == depth as usize && angle >= arc.start && angle < arc.start + arc.sweep
        })
    })
}

/// Whether a pointer lies inside the center hole (distance < hole).
fn in_hole(bounds: Bounds<Pixels>, position: gpui::Point<Pixels>, hole: f32) -> bool {
    let center = bounds.center();
    let dx = f32::from(position.x) - f32::from(center.x);
    let dy = f32::from(position.y) - f32::from(center.y);
    dx.hypot(dy) < hole
}

#[derive(Default)]
struct SunburstState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    zoom_path: Vec<usize>,
}

/// Parts of parts as rings around a center that holds the whole, each arc as wide as its share. Hover reads an arc's path and share.
#[derive(IntoElement)]
pub struct Sunburst {
    base: Div,
    id: ElementId,
    slices: Vec<Slice>,
}

impl Sunburst {
    pub fn new(id: impl Into<ElementId>, slices: impl IntoIterator<Item = Slice>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            slices: slices.into_iter().collect(),
        }
    }
}

impl Styled for Sunburst {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Sunburst {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ring: Entity<SunburstState> =
            window.use_keyed_state((self.id.clone(), "ring"), cx, |_, _| {
                SunburstState::default()
            });
        let (active, crumbs) = subtree(&self.slices, &ring.read(cx).zoom_path);
        let valid_depth = crumbs.len();
        let arcs = rings(active);
        let depths = arcs.iter().map(|arc| arc.depth + 1).max().unwrap_or(1);
        let total: f64 = active.iter().map(Slice::weight).sum();
        let hover = ring.read(cx).hover.filter(|ix| *ix < arcs.len());
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let bounds = ring.read(cx).bounds;
        let outer =
            (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 2.0).max(0.0);
        let hole = outer * 0.34;
        let band = (outer - hole) / depths as f32;
        let gap = f32::from(sizes.stroke.to_pixels(rem));
        let color = |arc: &Arc, lit: bool| {
            let fade = if lit {
                1.0
            } else {
                (1.0 - 0.22 * arc.depth as f32).max(0.4)
            };
            tint(&colors, arc.branch).opacity(fade)
        };
        let fills: Vec<Hsla> = arcs
            .iter()
            .enumerate()
            .map(|(ix, arc)| color(arc, hover == Some(ix)))
            .collect();
        let (big, small) = match hover.map(|ix| &arcs[ix]) {
            Some(arc) => (
                format::percent(if total > 0.0 { arc.weight / total } else { 0.0 }, 1, false),
                arc.path.join(" › "),
            ),
            None => (
                compact(total),
                crumbs
                    .last()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| "Total".to_string()),
            ),
        };
        let path = div()
            .max_w(Pixels::from(hole * 1.6))
            .truncate()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_muted)
            .child(small);
        let path = if hover.is_some() {
            path.debug_selector(|| "sunburst-tooltip".into())
        } else {
            path
        };
        let middle = div()
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .debug_selector(|| "chart-center".into())
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Lg))
                    .text_color(colors.fg)
                    .child(big),
            )
            .child(path);
        let (moved, left, measured, hit) = (ring.clone(), ring.clone(), ring.clone(), arcs.clone());
        let (pressed, released) = (ring.clone(), ring.clone());
        let (press_hit, up_hit) = (arcs.clone(), arcs.clone());
        let painted = arcs.clone();
        let controls = (!crumbs.is_empty()).then(|| {
            let up_state = ring.clone();
            let label = crumbs.join(" › ");
            let box_w = f32::from(bounds.size.width);
            let crumb_max = if box_w > 0.0 {
                (box_w - 84.0).clamp(48.0, 160.0)
            } else {
                160.0
            };
            let button = Button::new((self.id.clone(), "up"), "Up")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .debug_selector(|| "sunburst-up".into())
                .on_click(move |_, _, cx| {
                    up_state.update(cx, |state, cx| {
                        let depth = valid_depth.min(state.zoom_path.len());
                        state.zoom_path.truncate(depth.saturating_sub(1));
                        state.hover = None;
                        cx.notify();
                    })
                });
            div()
                .debug_selector(|| "sunburst-breadcrumb".into())
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
        let mut frame = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        frame.style().refine(self.base.style());
        frame
            .child(
                div()
                    .id((self.id.clone(), "ring"))
                    .absolute()
                    .inset_0()
                    .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                        let next = arc_under(
                            &hit,
                            moved.read(cx).bounds,
                            event.position,
                            hole,
                            band,
                            outer,
                        );
                        moved.update(cx, |ring, cx| {
                            let next = next.flatten();
                            if ring.hover != next {
                                ring.hover = next;
                                cx.notify();
                            }
                        })
                    })
                    .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                        if event.click_count >= 2 {
                            return;
                        }
                        let next = arc_under(
                            &press_hit,
                            pressed.read(cx).bounds,
                            event.position,
                            hole,
                            band,
                            outer,
                        );
                        pressed.update(cx, |ring, cx| {
                            let next = next.flatten();
                            if ring.hover != next {
                                ring.hover = next;
                                cx.notify();
                            }
                        })
                    })
                    .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                        let bounds = released.read(cx).bounds;
                        if event.click_count >= 2 {
                            released.update(cx, |ring, cx| {
                                if !ring.zoom_path.is_empty() {
                                    ring.zoom_path.clear();
                                    ring.hover = None;
                                    cx.notify();
                                }
                            });
                            return;
                        }
                        let picked =
                            arc_under(&up_hit, bounds, event.position, hole, band, outer).flatten();
                        released.update(cx, |ring, cx| match picked {
                            Some(ix) if up_hit[ix].is_branch => {
                                let depth = valid_depth.min(ring.zoom_path.len());
                                ring.zoom_path.truncate(depth);
                                ring.zoom_path.extend_from_slice(&up_hit[ix].indices);
                                ring.hover = None;
                                cx.notify();
                            }
                            None if valid_depth > 0 && in_hole(bounds, event.position, hole) => {
                                let depth = valid_depth.min(ring.zoom_path.len());
                                ring.zoom_path.truncate(depth.saturating_sub(1));
                                ring.hover = None;
                                cx.notify();
                            }
                            _ => {}
                        })
                    })
                    .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
                    .on_hover(move |inside, _, cx| {
                        if !*inside {
                            left.update(cx, |ring, cx| {
                                ring.hover = None;
                                cx.notify();
                            })
                        }
                    })
                    .child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| {
                                let center = bounds.center();
                                for (ix, arc) in painted.iter().enumerate() {
                                    let from = hole + band * arc.depth as f32;
                                    if let Some(path) = wedge(
                                        center,
                                        (from, from + band - gap),
                                        (arc.start, arc.sweep),
                                        gap,
                                    ) {
                                        finish(path, fills[ix], window);
                                    }
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(middle)
                    .child(measure(measured, |ring| &mut ring.bounds)),
            )
            .children(controls)
    }
}

#[cfg(test)]
mod tests;
