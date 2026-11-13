use std::{collections::HashMap, fmt::Write as _, rc::Rc};

use gpui::{
    App, Bounds, ContentMask, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchPhase, Window, canvas, div, fill,
    size,
};

use super::{
    axes::{anchored, below, beside},
    geometry::{Rect, frame},
    paint::{at, finish, measure, place, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::{Linear, compact, nice},
    series::drawable,
    tiles::key,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{format, tabular},
};

const SQRT_3: f32 = 1.732_050_8;

type Domain = ((f64, f64), (f64, f64));

#[derive(Default)]
struct HexState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    domain: Option<Domain>,
    drag_last: Option<(f32, f32)>,
    dragged: bool,
}

impl HexState {
    fn pointed(&self, count: usize) -> Option<usize> {
        self.hover.filter(|ix| *ix < count)
    }
}

/// A hexagonal bin holding aggregated screen and data centers and its point count.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HexBin {
    pub center: (f32, f32),
    pub data_center: (f64, f64),
    pub count: usize,
}

/// Dense 2D point clouds aggregated into regular hexagonal bins shaded by count.
#[derive(IntoElement)]
pub struct HexbinChart {
    base: Div,
    id: ElementId,
    points: Vec<(f64, f64)>,
    radius: f32,
    axes: (SharedString, SharedString),
    format: Format,
}

impl HexbinChart {
    /// Create a hexagonal-binning density chart.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            points: Vec::new(),
            radius: 12.0,
            axes: ("X".into(), "Y".into()),
            format: Rc::new(compact),
        }
    }

    /// Append a single `(x, y)` point.
    pub fn point(mut self, x: f64, y: f64) -> Self {
        assert!(
            drawable(x) && drawable(y),
            "a hexbin point needs finite values, within charts::LIMIT"
        );
        self.points.push((x, y));
        self
    }

    /// Append multiple `(x, y)` points.
    pub fn points(mut self, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        for (x, y) in points {
            self = self.point(x, y);
        }
        self
    }

    /// Set the hexagon circumradius in logical pixels (clamped to at least 4 px).
    pub fn radius(mut self, radius: f32) -> Self {
        assert!(
            radius.is_finite() && radius > 0.0,
            "a hexbin radius must be finite and positive"
        );
        self.radius = radius.max(4.0);
        self
    }

    /// Set the axis labels used in tooltips.
    pub fn axes(mut self, x: impl Into<SharedString>, y: impl Into<SharedString>) -> Self {
        self.axes = (x.into(), y.into());
        self
    }

    /// Format numeric coordinates in axes and tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export aggregated bins as CSV (`x,y,count`) at the given reference layout size.
    pub fn csv(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem) = (theme.chart(), theme.base_rem());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let layout = compute_layout(&self.points, rect, self.radius, None);
        let mut out = String::from("x,y,count");
        for bin in &layout.bins {
            write!(
                out,
                "\n{},{},{}",
                bin.data_center.0, bin.data_center.1, bin.count
            )
            .expect("csv write");
        }
        out
    }

    /// Export the chart as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let layout = compute_layout(&self.points, rect, self.radius, None);
        let ink = tint(colors, 0);
        let (bg_hex, _) = svg_hex(colors.bg);
        let (ink_hex, _) = svg_hex(ink);
        let (grid_hex, _) = svg_hex(colors.border);
        let (label_hex, _) = svg_hex(colors.fg_subtle);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for (y_px, _) in &layout.y_ticks {
            write!(
                out,
                r#"<line x1="{:.1}" y1="{y_px:.1}" x2="{:.1}" y2="{y_px:.1}" stroke="{grid_hex}" stroke-opacity="0.6" stroke-width="1"/>"#,
                rect.x,
                rect.x + rect.w
            )
            .expect("svg write");
        }
        let max_count = layout.max_count.max(1);
        for bin in &layout.bins {
            let alpha = strength(bin.count, max_count);
            let verts = hex_vertices(bin.center, layout.radius);
            let mut d = String::new();
            for (idx, (vx, vy)) in verts.iter().enumerate() {
                let cmd = if idx == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{vx:.1},{vy:.1} ").expect("svg write");
            }
            d.push('Z');
            write!(
                out,
                r#"<path d="{d}" fill="{ink_hex}" fill-opacity="{alpha:.2}" stroke="{ink_hex}" stroke-opacity="0.45" stroke-width="1"/>"#,
            )
            .expect("svg write");
        }
        for (x_px, val) in &layout.x_ticks {
            write!(
                out,
                r#"<text x="{x_px:.1}" y="{:.1}" text-anchor="middle" fill="{label_hex}" font-size="11">{}</text>"#,
                rect.y + rect.h + 16.0,
                (self.format)(*val)
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

struct HexLayout {
    bins: Vec<HexBin>,
    max_count: usize,
    radius: f32,
    x_ticks: Vec<(f32, f64)>,
    y_ticks: Vec<(f32, f64)>,
    base_domain: Domain,
}

fn svg_hex(color: Hsla) -> (String, f32) {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            byte(rgba.r),
            byte(rgba.g),
            byte(rgba.b)
        ),
        rgba.a,
    )
}

fn strength(count: usize, max_count: usize) -> f32 {
    if max_count <= 1 {
        0.65
    } else {
        0.14 + 0.86 * (count as f32 / max_count as f32)
    }
}

fn steps(ink: Hsla) -> [Hsla; 5] {
    [0.14, 0.35, 0.56, 0.78, 1.0].map(|s| ink.opacity(s))
}

/// Compute pointy-top hexagon vertices around `center` with circumradius `radius`.
pub(crate) fn hex_vertices(center: (f32, f32), radius: f32) -> [(f32, f32); 6] {
    std::array::from_fn(|ix| {
        let angle = std::f32::consts::FRAC_PI_3 * ix as f32 - std::f32::consts::FRAC_PI_6;
        (
            center.0 + radius * angle.cos(),
            center.1 + radius * angle.sin(),
        )
    })
}

/// Snap a 2D pixel coordinate `(px, py)` to the nearest pointy-top hexagonal lattice key `(q, r)` and center.
fn snap_hex(px: f32, py: f32, radius: f32) -> ((i32, i32), (f32, f32)) {
    let dx = radius * SQRT_3;
    let dy = radius * 1.5;
    let py_norm = py / dy;
    let pj_base = py_norm.round() as i32;
    let px_norm = px / dx;
    let mut best_key = (0, 0);
    let mut best_center = (0.0, 0.0);
    let mut best_dist = f32::INFINITY;

    for dj in -1..=1 {
        let r = pj_base + dj;
        let offset = if r.rem_euclid(2) != 0 { 0.5 } else { 0.0 };
        let q_center = (px_norm - offset).round() as i32;
        for dq in -1..=1 {
            let q = q_center + dq;
            let cx = (q as f32 + offset) * dx;
            let cy = r as f32 * dy;
            let dist = (px - cx).powi(2) + (py - cy).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best_key = (q, r);
                best_center = (cx, cy);
            }
        }
    }
    (best_key, best_center)
}

fn local_pos(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

fn domain_ticks(low: f64, high: f64) -> Vec<f64> {
    let (_, ticks) = nice(low, high, 5);
    let tol = (high - low).abs() * 1e-9;
    ticks
        .into_iter()
        .filter(|v| *v >= low - tol && *v <= high + tol)
        .collect()
}

fn pan_domain(((x0, x1), (y0, y1)): Domain, rect: Rect, dx: f32, dy: f32) -> Domain {
    let shift_x = -(f64::from(dx) / f64::from(rect.w.max(1.0))) * (x1 - x0);
    let shift_y = (f64::from(dy) / f64::from(rect.h.max(1.0))) * (y1 - y0);
    ((x0 + shift_x, x1 + shift_x), (y0 + shift_y, y1 + shift_y))
}

fn zoom_axis((lo, hi): (f64, f64), base_span: f64, anchor: f64, factor: f64) -> (f64, f64) {
    let cur_span = (hi - lo).abs().max(1e-12);
    let min_span = (base_span * 1e-4).max(1e-9);
    let max_span = (base_span * 20.0).max(min_span);
    let new_span = (cur_span * factor).clamp(min_span, max_span);
    let actual = new_span / cur_span;
    (
        anchor + (lo - anchor) * actual,
        anchor + (hi - anchor) * actual,
    )
}

fn zoom_domain(current: Domain, base: Domain, anchor: (f64, f64), factor: f64) -> Domain {
    let bx = (base.0.1 - base.0.0).abs().max(1e-9);
    let by = (base.1.1 - base.1.0).abs().max(1e-9);
    (
        zoom_axis(current.0, bx, anchor.0, factor),
        zoom_axis(current.1, by, anchor.1, factor),
    )
}

fn cursor_data((x_dom, y_dom): Domain, rect: Rect, pos: (f32, f32)) -> (f64, f64) {
    let px = pos.0.clamp(rect.x, rect.x + rect.w);
    let py = pos.1.clamp(rect.y, rect.y + rect.h);
    let x_scale = Linear::new(x_dom, (rect.x, rect.x + rect.w));
    let y_scale = Linear::new(y_dom, (rect.y + rect.h, rect.y));
    (x_scale.value(px), y_scale.value(py))
}

fn compute_layout(
    points: &[(f64, f64)],
    rect: Rect,
    radius: f32,
    domain: Option<Domain>,
) -> HexLayout {
    if points.is_empty() || rect.w <= 0.0 || rect.h <= 0.0 {
        let base_domain = ((0.0, 1.0), (0.0, 1.0));
        return HexLayout {
            bins: Vec::new(),
            max_count: 0,
            radius,
            x_ticks: Vec::new(),
            y_ticks: Vec::new(),
            base_domain,
        };
    }
    let (mut x_min, mut x_max) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y_min, mut y_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(x, y) in points {
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }
    let (base_x, base_x_ticks) = nice(x_min, x_max, 5);
    let (base_y, base_y_ticks) = nice(y_min, y_max, 5);
    let base_domain = (base_x, base_y);
    let ((x0, x1), (y0, y1)) = domain.unwrap_or(base_domain);
    let (x_tick_vals, y_tick_vals) = if domain.is_some() {
        (domain_ticks(x0, x1), domain_ticks(y0, y1))
    } else {
        (base_x_ticks, base_y_ticks)
    };
    let x_scale = Linear::new((x0, x1), (rect.x, rect.x + rect.w));
    let y_scale = Linear::new((y0, y1), (rect.y + rect.h, rect.y));

    let mut map: HashMap<(i32, i32), ((f32, f32), usize)> = HashMap::new();
    for &(x, y) in points {
        if domain.is_some() && (x < x0 || x > x1 || y < y0 || y > y1) {
            continue;
        }
        let px = x_scale.at(x) - rect.x;
        let py = y_scale.at(y) - rect.y;
        let (key, local_center) = snap_hex(px, py, radius);
        let entry = map
            .entry(key)
            .or_insert(((rect.x + local_center.0, rect.y + local_center.1), 0));
        entry.1 += 1;
    }

    let mut bins: Vec<HexBin> = map
        .into_values()
        .map(|(center, count)| HexBin {
            center,
            data_center: (x_scale.value(center.0), y_scale.value(center.1)),
            count,
        })
        .collect();
    bins.sort_by(|a, b| {
        a.center
            .1
            .total_cmp(&b.center.1)
            .then_with(|| a.center.0.total_cmp(&b.center.0))
    });
    let max_count = bins.iter().map(|b| b.count).max().unwrap_or(0);
    let x_ticks = x_tick_vals
        .into_iter()
        .map(|v| (x_scale.at(v), v))
        .collect();
    let y_ticks = y_tick_vals
        .into_iter()
        .map(|v| (y_scale.at(v), v))
        .collect();
    HexLayout {
        bins,
        max_count,
        radius,
        x_ticks,
        y_ticks,
        base_domain,
    }
}

impl Styled for HexbinChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for HexbinChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<HexState> =
            window.use_keyed_state((self.id.clone(), "hexbin"), cx, |_, _| HexState::default());
        let (bounds, domain) = {
            let s = state.read(cx);
            (s.bounds, s.domain)
        };
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let layout = compute_layout(&self.points, rect, self.radius, domain);
        let hover = state.read(cx).pointed(layout.bins.len());
        let has_data = !self.points.is_empty();
        let total = self.points.len().max(1);
        let ink = tint(&colors, 0);
        let format = self.format.clone();
        let (x_name, y_name) = self.axes.clone();

        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every_x = ((layout.x_ticks.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        let x_labels: Vec<_> = layout
            .x_ticks
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every_x == 0)
            .map(|(_, (at_x, val))| below(*at_x, rect, tabular(text(format(*val).into()))))
            .collect();
        let y_labels: Vec<_> = layout
            .y_ticks
            .iter()
            .map(|(at_y, val)| {
                beside(
                    *at_y,
                    sizes.gutter.to_pixels(rem),
                    tabular(text(format(*val).into())),
                )
            })
            .collect();

        let tooltip = hover.map(|ix| {
            let bin = &layout.bins[ix];
            let share = bin.count as f64 / total as f64;
            let color = ink.opacity(strength(bin.count, layout.max_count));
            let card = ChartTooltip::new(format!("{} points", bin.count))
                .row(Some(color), "Share", format::percent(share, 1, false))
                .row(None, x_name.clone(), format(bin.data_center.0))
                .row(None, y_name.clone(), format(bin.data_center.1));
            let pos = (bin.center.0 + layout.radius, bin.center.1);
            anchored(pos, pos.0 > box_size.0 * 0.55, card)
        });

        let bins_for_hit = layout.bins.clone();
        let bins_for_paint = layout.bins;
        let y_ticks_for_paint = layout.y_ticks;
        let base_domain = layout.base_domain;
        let radius = layout.radius;
        let max_count = layout.max_count;
        let (hairline, stroke) = (sizes.hairline, sizes.stroke.to_pixels(rem));
        let (edge, grid_color) = (colors.fg, colors.border.opacity(0.6));
        let legend = key(steps(ink), ("Few", "Many"), cx);

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let find_hover = Rc::new(move |pos: (f32, f32)| {
            if !rect.contains(pos) {
                return None;
            }
            let hit_radius = if touch_mode { radius.max(24.0) } else { radius };
            let limit = hit_radius * hit_radius * 1.05;
            bins_for_hit
                .iter()
                .enumerate()
                .map(|(ix, bin)| {
                    let dx = pos.0 - bin.center.0;
                    let dy = pos.1 - bin.center.1;
                    (ix, dx * dx + dy * dy)
                })
                .filter(|(_, d2)| *d2 <= limit)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(ix, _)| ix)
        });
        let show_all = domain.map(|_| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "show-all"), "Show all")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .debug_selector(|| "show-all".into())
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |s, cx| {
                        s.domain = None;
                        s.drag_last = None;
                        s.dragged = false;
                        s.hover = None;
                        cx.notify();
                    })
                }))
        });
        let (pressed, moved, released, released_out, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (down_hit, move_hit) = (find_hover.clone(), find_hover);
        let end_drag = move |target: &Entity<HexState>, cx: &mut App| {
            target.update(cx, |s, cx| {
                s.drag_last = None;
                if s.dragged {
                    s.dragged = false;
                    cx.notify();
                }
            });
        };

        let chart = div()
            .id((self.id.clone(), "hexbin"))
            .debug_selector(|| "hexbin-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                if event.click_count == 2 {
                    pressed.update(cx, |s, cx| {
                        s.domain = None;
                        s.drag_last = None;
                        s.dragged = false;
                        s.hover = None;
                        cx.notify();
                    });
                    return;
                }
                let pos = local_pos(event.position, pressed.read(cx).bounds);
                let next = down_hit(pos);
                pressed.update(cx, |s, cx| {
                    s.drag_last = Some(pos);
                    s.dragged = false;
                    s.hover = next;
                    cx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, moved.read(cx).bounds);
                moved.update(cx, |s, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(last) = s.drag_last
                    {
                        let dx = pos.0 - last.0;
                        let dy = pos.1 - last.1;
                        if !s.dragged && dx * dx + dy * dy <= 9.0 {
                            return;
                        }
                        if !has_data || rect.w <= 0.0 || rect.h <= 0.0 {
                            return;
                        }
                        s.dragged = true;
                        s.drag_last = Some(pos);
                        s.hover = None;
                        let cur = s.domain.unwrap_or(base_domain);
                        s.domain = Some(pan_domain(cur, rect, dx, dy));
                        cx.notify();
                        return;
                    }
                    let was_dragging = s.drag_last.is_some() || s.dragged;
                    s.drag_last = None;
                    s.dragged = false;
                    let next = move_hit(pos);
                    if s.hover != next || was_dragging {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| end_drag(&released, cx))
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                end_drag(&released_out, cx)
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |s, cx| {
                        s.hover = None;
                        cx.notify();
                    });
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                if !has_data || rect.w <= 0.0 || rect.h <= 0.0 {
                    return;
                }
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                let pos = local_pos(ev.position, scrolled.read(cx).bounds);
                if ev.source == ScrollSource::Touch {
                    if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                        scrolled.update(cx, |s, cx| {
                            let cur = s.domain.unwrap_or(base_domain);
                            s.domain = Some(pan_domain(cur, rect, dx, dy));
                            s.hover = None;
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                } else {
                    let factor = (1.0 - f64::from(dy) * 0.002).clamp(0.1, 10.0);
                    if (factor - 1.0).abs() > 1e-6 {
                        scrolled.update(cx, |s, cx| {
                            let cur = s.domain.unwrap_or(base_domain);
                            let anchor = cursor_data(cur, rect, pos);
                            s.domain = Some(zoom_domain(cur, base_domain, anchor, factor));
                            s.hover = None;
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                if !ev.delta.is_finite() || ev.phase == TouchPhase::Cancelled {
                    pinched.update(cx, |s, _| {
                        s.drag_last = None;
                        s.dragged = false;
                    });
                    return;
                }
                let scale = ev.delta.exp().clamp(0.1, 10.0);
                let pos = local_pos(ev.position, pinched.read(cx).bounds);
                let factor = 1.0 / f64::from(scale);
                pinched.update(cx, |s, cx| {
                    s.drag_last = None;
                    s.dragged = false;
                    if !has_data || rect.w <= 0.0 || rect.h <= 0.0 || (scale - 1.0).abs() <= 1e-5 {
                        return;
                    }
                    let cur = s.domain.unwrap_or(base_domain);
                    let anchor = cursor_data(cur, rect, pos);
                    s.domain = Some(zoom_domain(cur, base_domain, anchor, factor));
                    s.hover = None;
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let mask = ContentMask {
                            bounds: place(origin, rect),
                        };
                        for (y_px, _) in &y_ticks_for_paint {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, *y_px)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                grid_color,
                            ));
                        }
                        window.with_content_mask(Some(mask), |window| {
                            for (ix, bin) in bins_for_paint.iter().enumerate() {
                                let verts = hex_vertices(bin.center, radius);
                                let mut fill_path = PathBuilder::fill();
                                let mut edge_path = PathBuilder::stroke(if hover == Some(ix) {
                                    stroke
                                } else {
                                    hairline
                                });
                                fill_path.move_to(at(origin, verts[0]));
                                edge_path.move_to(at(origin, verts[0]));
                                for v in &verts[1..] {
                                    fill_path.line_to(at(origin, *v));
                                    edge_path.line_to(at(origin, *v));
                                }
                                fill_path.close();
                                edge_path.close();
                                let fill_color = ink.opacity(strength(bin.count, max_count));
                                let border_color = if hover == Some(ix) {
                                    edge
                                } else {
                                    ink.opacity(0.45)
                                };
                                finish(fill_path, fill_color, window);
                                finish(edge_path, border_color, window);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(x_labels)
            .children(y_labels)
            .children(tooltip)
            .children(show_all)
            .child(measure(state, |state| &mut state.bounds));

        root.flex().flex_col().gap_2().child(chart).child(legend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexbin_snaps_nearby_points_and_handles_empty_or_degenerate_domains() {
        let rect = Rect {
            x: 20.0,
            y: 10.0,
            w: 200.0,
            h: 160.0,
        };
        let empty = compute_layout(&[], rect, 12.0, None);
        assert!(empty.bins.is_empty());
        assert_eq!(empty.max_count, 0);

        let same = compute_layout(&[(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)], rect, 12.0, None);
        assert_eq!(same.bins.len(), 1);
        assert_eq!(same.bins[0].count, 3);
        assert_eq!(same.max_count, 3);

        let cluster = compute_layout(&[(0.0, 0.0), (1.0, 1.0), (100.0, 100.0)], rect, 16.0, None);
        let total: usize = cluster.bins.iter().map(|b| b.count).sum();
        assert_eq!(total, 3);
        assert!(cluster.bins.len() >= 2);

        let base = ((0.0, 10.0), (0.0, 10.0));
        let tiny = zoom_domain(base, base, (5.0, 5.0), 1e-9);
        assert!((tiny.0.1 - tiny.0.0) >= 10.0 * 1e-4 - 1e-9);
        let wide = zoom_domain(base, base, (5.0, 5.0), 100.0);
        assert!((wide.0.1 - wide.0.0) <= 10.0 * 20.0 + 1e-6);
    }

    #[test]
    fn hex_vertices_form_regular_circumradius_hexagon() {
        let verts = hex_vertices((100.0, 50.0), 10.0);
        for (x, y) in verts {
            let dist = ((x - 100.0).powi(2) + (y - 50.0).powi(2)).sqrt();
            assert!((dist - 10.0).abs() < 1e-4);
        }
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn hexbin_chart_renders_exports_and_opens_tooltip_on_hover(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{
            Context, Modifiers, PinchEvent, Render, ScrollDelta, ScrollEvent, Window, point, px,
        };
        use std::cell::RefCell;

        struct HexHarness(Vec<(f64, f64)>, Rc<RefCell<Vec<f64>>>);
        impl Render for HexHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let seen = self.1.clone();
                div().w(px(360.0)).h(px(260.0)).child(
                    HexbinChart::new("hex-test")
                        .points(self.0.iter().copied())
                        .radius(14.0)
                        .axes("Duration", "Cost")
                        .format(move |v| {
                            seen.borrow_mut().push(v);
                            compact(v)
                        }),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            let chart = HexbinChart::new("export")
                .points([(10.0, 20.0), (11.0, 21.0), (50.0, 80.0)])
                .radius(12.0);
            let svg = chart.svg(360.0, 240.0, cx);
            assert!(svg.starts_with("<svg") && svg.contains("<path"));
            let csv = chart.csv(360.0, 240.0, cx);
            assert!(csv.starts_with("x,y,count\n"));
        });

        let (_, vc) =
            cx.add_window_view(|_, _| HexHarness(vec![], Rc::new(RefCell::new(Vec::new()))));
        settle(vc);
        assert!(vc.debug_bounds("hexbin-plot").is_some());

        let recorded = Rc::new(RefCell::new(Vec::new()));
        let (_, vc) = cx.add_window_view(|_, _| {
            HexHarness(
                vec![(0.0, 0.0), (50.0, 50.0), (50.5, 50.5), (100.0, 100.0)],
                recorded.clone(),
            )
        });
        settle(vc);
        let plot = vc.debug_bounds("hexbin-plot").expect("hexbin plot bounds");
        let rect_center = point(
            plot.left() + px(48.0) + (plot.size.width - px(56.0)) * 0.5,
            plot.top() + px(8.0) + (plot.size.height - px(36.0)) * 0.5,
        );
        vc.simulate_mouse_move(rect_center, None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "hover over bin opens tooltip"
        );
        assert!(vc.debug_bounds("show-all").is_none());

        // Pointer wheel scroll zooms in around cursor.
        recorded.borrow_mut().clear();
        vc.simulate_event(ScrollEvent {
            position: rect_center,
            delta: ScrollDelta::Pixels(point(px(0.0), px(200.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        });
        settle(vc);
        assert!(vc.debug_bounds("show-all").is_some());
        let zoomed_ticks = recorded.borrow().clone();
        assert!(!zoomed_ticks.is_empty());
        let zoomed_span = zoomed_ticks
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            - zoomed_ticks.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(zoomed_span < 100.0, "wheel zoom shrinks visible domain");

        // Left-button drag (> 3px) pans the domain.
        recorded.borrow_mut().clear();
        let start = rect_center;
        vc.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(
            start + point(px(90.0), px(-90.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            start + point(px(90.0), px(-90.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(vc);
        let panned_ticks = recorded.borrow().clone();
        assert!(!panned_ticks.is_empty());
        let panned_min = panned_ticks.iter().copied().fold(f64::INFINITY, f64::min);
        let zoomed_min = zoomed_ticks.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(panned_min < zoomed_min, "dragging shifts domain ticks");

        // Double-click resets domain = None.
        vc.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: rect_center,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_event(gpui::MouseUpEvent {
            button: MouseButton::Left,
            position: rect_center,
            modifiers: Modifiers::none(),
            click_count: 2,
        });
        settle(vc);
        assert!(
            vc.debug_bounds("show-all").is_none(),
            "double-click resets zoom"
        );

        // Pinch zooms in and Show all button resets.
        vc.simulate_event(PinchEvent {
            position: rect_center,
            delta: 0.6,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc);
        let show_all = vc
            .debug_bounds("show-all")
            .expect("pinch shows reset button");
        vc.simulate_click(show_all.center(), Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("show-all").is_none(),
            "Show all button resets zoom"
        );

        // Touch scroll pans and tap without drag preserves tooltip.
        vc.simulate_event(ScrollEvent {
            position: rect_center,
            delta: ScrollDelta::Pixels(point(px(30.0), px(0.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc);
        assert!(vc.debug_bounds("show-all").is_some());
        let reset = vc
            .debug_bounds("show-all")
            .expect("show-all after touch pan");
        vc.simulate_click(reset.center(), Modifiers::none());
        settle(vc);
        vc.simulate_mouse_down(rect_center, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(rect_center, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "tap without drag preserves tooltip"
        );
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn hexbin_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};
        use std::cell::RefCell;

        let pts = vec![(0.0, 0.0), (80.0, 50.0), (100.0, 100.0)];
        struct NarrowHex(Vec<(f64, f64)>, Rc<RefCell<Vec<f64>>>);
        impl Render for NarrowHex {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let seen = self.1.clone();
                div().w(px(240.0)).h(px(240.0)).child(
                    HexbinChart::new("hex-touch")
                        .points(self.0.iter().copied())
                        .radius(6.0)
                        .format(move |v| {
                            seen.borrow_mut().push(v);
                            compact(v)
                        }),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });
        let recorded = Rc::new(RefCell::new(Vec::new()));
        let (_, vc) = cx.add_window_view(|_, _| NarrowHex(pts.clone(), recorded.clone()));
        vc.simulate_resize(size(px(240.0), px(240.0)));
        settle(vc);

        let plot = vc.debug_bounds("hexbin-plot").expect("hexbin plot");
        let rect = frame(
            (f32::from(plot.size.width), f32::from(plot.size.height)),
            48.0,
            24.0,
            8.0,
        );
        let layout = compute_layout(&pts, rect, 6.0, None);
        let right_bin = layout
            .bins
            .iter()
            .find(|b| b.center.0 > rect.x + rect.w * 0.6 && b.center.0 < rect.x + rect.w * 0.95)
            .expect("mid-right bin");
        let near_tap = point(
            plot.left() + px(right_bin.center.0 - 18.0),
            plot.top() + px(right_bin.center.1),
        );
        vc.simulate_mouse_down(near_tap, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(near_tap, MouseButton::Left, Modifiers::none());
        settle(vc);
        let tip = vc
            .debug_bounds("chart-tooltip")
            .expect("touch_mode 24px target hits small hex bin");
        assert!(
            tip.right() <= plot.right() + px(1.0),
            "right-side tooltip flips inside narrow 240px card"
        );

        // Single-finger down followed by pinch clears drag_last so trailing move does not pan.
        let center = plot.center();
        vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        settle(vc);
        recorded.borrow_mut().clear();
        vc.simulate_event(PinchEvent {
            position: center,
            delta: 0.5,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc);
        let range = |vals: &[f64]| {
            (
                vals.iter().copied().fold(f64::INFINITY, f64::min),
                vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        let after_pinch_range = range(&recorded.borrow());
        recorded.borrow_mut().clear();
        vc.simulate_mouse_move(
            center + point(px(80.0), px(-80.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            center + point(px(80.0), px(-80.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(vc);
        vc.update(|window, _| window.refresh());
        settle(vc);
        assert_eq!(range(&recorded.borrow()), after_pinch_range);

        // Touch scroll pans and Show all button (>= 48px) resets.
        vc.simulate_event(ScrollEvent {
            position: center,
            delta: ScrollDelta::Pixels(point(px(24.0), px(0.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc);
        let show_all = vc
            .debug_bounds("show-all")
            .expect("show-all after pinch/pan");
        assert!(show_all.size.width >= px(48.0) && show_all.size.height >= px(48.0));
        vc.simulate_click(show_all.center(), Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("show-all").is_none());
    }
}
