use std::{fmt::Write as _, rc::Rc};

use gpui::{
    App, Bounds, DispatchPhase, Div, ElementId, Entity, HoverListenerMode, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchDragEvent, TouchPhase, Window,
    canvas, div, fill, prelude::FluentBuilder as _, size,
};

use super::{
    axes::{anchored, below, beside},
    geometry::{Rect, frame, point_at},
    paint::{at, finish, measure, place, ring},
    parts::ChartTooltip,
    plot::Format,
    scale::{Band, Linear, compact, nice},
    series::Series,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::tabular,
};

#[derive(Default)]
struct DifferenceState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    pinned: Option<usize>,
    span: Option<(usize, usize)>,
    brush: Option<(f32, f32)>,
}

fn local(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

fn zoom_span(
    current: Option<(usize, usize)>,
    total: usize,
    factor: f32,
    anchor_t: f32,
) -> Option<(usize, usize)> {
    if total < 3 {
        return None;
    }
    let (lo, hi) = current
        .filter(|(a, b)| *a < *b && *b < total)
        .unwrap_or((0, total - 1));
    let cur_len = hi - lo;
    let mut target_len = ((cur_len as f32 * factor).round() as usize).clamp(1, total - 1);
    if target_len == cur_len {
        if factor < 0.99 && cur_len > 1 {
            target_len = cur_len - 1;
        } else if factor > 1.01 && cur_len < total - 1 {
            target_len = cur_len + 1;
        }
    }
    let pivot = lo as f32 + anchor_t.clamp(0.0, 1.0) * cur_len as f32;
    let max_lo = (total - 1 - target_len) as f32;
    let new_lo = (pivot - anchor_t.clamp(0.0, 1.0) * target_len as f32)
        .round()
        .clamp(0.0, max_lo) as usize;
    let new_hi = new_lo + target_len;
    (new_lo > 0 || new_hi + 1 < total).then_some((new_lo, new_hi))
}

fn pan_span(
    current: Option<(usize, usize)>,
    total: usize,
    dx: f32,
    width: f32,
) -> Option<(usize, usize)> {
    let (lo, hi) = current.filter(|(a, b)| *a < *b && *b < total)?;
    if width <= 0.0 {
        return Some((lo, hi));
    }
    let vis_len = hi - lo;
    let step_px = width / vis_len.max(1) as f32;
    let shift = (-dx / step_px).round() as isize;
    let max_lo = (total - 1 - vis_len) as isize;
    let new_lo = (lo as isize + shift).clamp(0, max_lo) as usize;
    Some((new_lo, new_lo + vis_len))
}

/// Compares two series over shared categories, shading positive and negative differences separately.
#[derive(IntoElement)]
pub struct DifferenceChart {
    base: Div,
    id: ElementId,
    labels: Vec<SharedString>,
    primary: Option<Series>,
    baseline: Option<Series>,
    format: Format,
}

impl DifferenceChart {
    pub fn new(
        id: impl Into<ElementId>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            primary: None,
            baseline: None,
            format: Rc::new(compact),
        }
    }

    /// Primary series (shaded with `success` when above baseline).
    pub fn primary(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.labels.len(),
            "difference primary series needs a value per label"
        );
        self.primary = Some(series);
        self
    }

    /// Baseline series (shaded with `danger` when primary falls below it).
    pub fn baseline(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.labels.len(),
            "difference baseline series needs a value per label"
        );
        self.baseline = Some(series);
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    pub fn csv(&self) -> String {
        let mut out = String::from("label,primary,baseline,delta");
        if let (Some(a), Some(b)) = (&self.primary, &self.baseline) {
            for (ix, label) in self.labels.iter().enumerate() {
                let va = a.values[ix];
                let vb = b.values[ix];
                write!(out, "\n{label},{va},{vb},{}", va - vb).expect("csv write");
            }
        }
        out
    }

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
        let (a_vals, b_vals) = self.series_slices();
        let layout = compute_diff(&self.labels, a_vals, b_vals, rect);
        let hex = |c: gpui::Hsla| {
            let r: gpui::Rgba = c.into();
            format!(
                "#{:02x}{:02x}{:02x}",
                (r.r * 255.0).round() as u8,
                (r.g * 255.0).round() as u8,
                (r.b * 255.0).round() as u8
            )
        };
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        for (above, poly) in &layout.regions {
            let fill_hex = if *above {
                hex(colors.success)
            } else {
                hex(colors.danger)
            };
            let mut d = String::new();
            for (ix, &(px, py)) in poly.iter().enumerate() {
                let cmd = if ix == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg");
            }
            d.push('Z');
            write!(
                out,
                r#"<path d="{d}" fill="{fill_hex}" fill-opacity="0.24"/>"#
            )
            .expect("svg");
        }
        for (pts, color) in [
            (&layout.line_a, colors.fg),
            (&layout.line_b, colors.fg_subtle),
        ] {
            if let Some(first) = pts.first() {
                let mut d = format!("M{:.1},{:.1}", first.0, first.1);
                for &(px, py) in &pts[1..] {
                    write!(d, " L{px:.1},{py:.1}").expect("svg");
                }
                write!(
                    out,
                    r#"<path d="{d}" fill="none" stroke="{}" stroke-width="1.5"/>"#,
                    hex(color)
                )
                .expect("svg");
            }
        }
        out.push_str("</svg>");
        out
    }

    fn series_slices(&self) -> (&[f64], &[f64]) {
        let a = self.primary.as_ref().map_or(&[][..], |s| &s.values);
        let b = self.baseline.as_ref().map_or(&[][..], |s| &s.values);
        (a, b)
    }
}

struct DiffLayout {
    regions: Vec<(bool, Vec<(f32, f32)>)>,
    line_a: Vec<(f32, f32)>,
    line_b: Vec<(f32, f32)>,
    xs: Vec<f32>,
    ticks: Vec<(f32, f64)>,
}

/// Split segments at crossing points into closed polygons tagged `(primary >= baseline, polygon)`.
fn split_difference_regions(
    xs: &[f32],
    a_vals: &[f64],
    b_vals: &[f64],
    y_scale: &Linear,
) -> Vec<(bool, Vec<(f32, f32)>)> {
    let n = xs.len().min(a_vals.len()).min(b_vals.len());
    if n < 2 {
        return Vec::new();
    }
    let mut regions = Vec::new();
    for ix in 0..n - 1 {
        let (x0, x1) = (xs[ix], xs[ix + 1]);
        let (a0, a1) = (a_vals[ix], a_vals[ix + 1]);
        let (b0, b1) = (b_vals[ix], b_vals[ix + 1]);
        let (d0, d1) = (a0 - b0, a1 - b1);
        let (ya0, ya1) = (y_scale.at(a0), y_scale.at(a1));
        let (yb0, yb1) = (y_scale.at(b0), y_scale.at(b1));
        if (d0 >= 0.0 && d1 >= 0.0) || (d0 <= 0.0 && d1 <= 0.0) {
            let above = d0 + d1 >= 0.0;
            regions.push((above, vec![(x0, ya0), (x1, ya1), (x1, yb1), (x0, yb0)]));
        } else {
            let t = (d0 / (d0 - d1)).clamp(0.0, 1.0) as f32;
            let xm = x0 + (x1 - x0) * t;
            let ym = ya0 + (ya1 - ya0) * t;
            regions.push((d0 >= 0.0, vec![(x0, ya0), (xm, ym), (x0, yb0)]));
            regions.push((d1 >= 0.0, vec![(xm, ym), (x1, ya1), (x1, yb1)]));
        }
    }
    regions
}

fn compute_diff(labels: &[SharedString], a_vals: &[f64], b_vals: &[f64], rect: Rect) -> DiffLayout {
    let n = labels.len().min(a_vals.len()).min(b_vals.len());
    if n == 0 || rect.w <= 0.0 || rect.h <= 0.0 {
        return DiffLayout {
            regions: Vec::new(),
            line_a: Vec::new(),
            line_b: Vec::new(),
            xs: Vec::new(),
            ticks: Vec::new(),
        };
    }
    let (lo, hi) = a_vals[..n]
        .iter()
        .chain(&b_vals[..n])
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let ((y0, y1), tick_vals) = nice(lo, hi, 5);
    let y_scale = Linear::new((y0, y1), (rect.y + rect.h, rect.y));
    let band = Band {
        count: n,
        range: (rect.x, rect.x + rect.w),
        padding: 0.0,
    };
    let xs: Vec<f32> = (0..n).map(|ix| point_at(&band, ix, n)).collect();
    let line_a: Vec<(f32, f32)> = (0..n).map(|ix| (xs[ix], y_scale.at(a_vals[ix]))).collect();
    let line_b: Vec<(f32, f32)> = (0..n).map(|ix| (xs[ix], y_scale.at(b_vals[ix]))).collect();
    let regions = split_difference_regions(&xs, &a_vals[..n], &b_vals[..n], &y_scale);
    let ticks = tick_vals.into_iter().map(|v| (y_scale.at(v), v)).collect();
    DiffLayout {
        regions,
        line_a,
        line_b,
        xs,
        ticks,
    }
}

impl Styled for DifferenceChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DifferenceChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<DifferenceState> =
            window.use_keyed_state((self.id.clone(), "difference"), cx, |_, _| {
                DifferenceState::default()
            });
        let (bounds, state_hover, state_pinned, raw_span, brush) = {
            let st = state.read(cx);
            (st.bounds, st.hover, st.pinned, st.span, st.brush)
        };
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let (a_vals, b_vals) = self.series_slices();
        let n = self.labels.len().min(a_vals.len()).min(b_vals.len());
        let zoomed = raw_span.filter(|(a, b)| *a < *b && *b < n);
        let (start_ix, end_ix) = zoomed.unwrap_or((0, n.saturating_sub(1)));
        let vis_n = if n == 0 { 0 } else { end_ix - start_ix + 1 };
        let (vis_labels, vis_a, vis_b) = if n == 0 {
            (&[][..], &[][..], &[][..])
        } else {
            (
                &self.labels[start_ix..=end_ix],
                &a_vals[start_ix..=end_ix],
                &b_vals[start_ix..=end_ix],
            )
        };
        let layout = compute_diff(vis_labels, vis_a, vis_b, rect);
        let hover = state_hover.filter(|ix| *ix >= start_ix && *ix < start_ix + vis_n);
        let pinned = state_pinned.filter(|ix| *ix >= start_ix && *ix < start_ix + vis_n);
        let active = pinned.or(hover);
        let format = self.format.clone();
        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every_x =
            ((vis_n as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil() as usize).max(1);
        let x_labels: Vec<_> = layout
            .xs
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every_x == 0)
            .map(|(ix, &x_px)| below(x_px, rect, text(self.labels[start_ix + ix].clone())))
            .collect();
        let y_labels: Vec<_> = layout
            .ticks
            .iter()
            .map(|(y_px, val)| {
                beside(
                    *y_px,
                    sizes.gutter.to_pixels(rem),
                    tabular(text(format(*val).into())),
                )
            })
            .collect();

        let a_name = self
            .primary
            .as_ref()
            .map_or_else(|| "Primary".into(), |s| s.name.clone());
        let b_name = self
            .baseline
            .as_ref()
            .map_or_else(|| "Baseline".into(), |s| s.name.clone());
        let tooltip = active.and_then(|ix| {
            let local_ix = ix - start_ix;
            let px = *layout.xs.get(local_ix)?;
            let (va, vb) = (a_vals[ix], b_vals[ix]);
            let delta = va - vb;
            let delta_color = if delta >= 0.0 {
                colors.success
            } else {
                colors.danger
            };
            let card = ChartTooltip::new(self.labels[ix].clone())
                .row(Some(colors.fg), a_name, format(va))
                .row(Some(colors.fg_subtle), b_name, format(vb))
                .row(Some(delta_color), "Delta", format(delta));
            Some(anchored(
                (px, rect.y + rect.h * 0.4),
                px > box_size.0 * 0.55,
                card,
            ))
        });

        let show_all = (zoomed.is_some() || pinned.is_some()).then(|| {
            let reset = state.clone();
            let label = if zoomed.is_some() {
                "Show all"
            } else {
                "Reset"
            };
            let button = Button::new((self.id.clone(), "show-all"), label)
                .debug_selector(|| "chart-show-all".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.span = None;
                        st.pinned = None;
                        st.hover = None;
                        st.brush = None;
                        cx.notify();
                    })
                }))
        });

        let ticks = layout.ticks;
        let regions = layout.regions;
        let line_a = layout.line_a;
        let line_b = layout.line_b;
        let xs = layout.xs;
        let active_local = active.map(|ix| ix - start_ix);
        let is_pinned = pinned.is_some();
        let pick_x = move |pos: (f32, f32)| -> Option<usize> {
            if !rect.contains(pos) || vis_n == 0 || rect.w <= 0.0 {
                return None;
            }
            if vis_n == 1 {
                return Some(start_ix);
            }
            let t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
            let local_ix = ((t * (vis_n - 1) as f32).round() as usize).min(vis_n - 1);
            Some(start_ix + local_ix)
        };
        let col_at = move |x: f32| -> Option<usize> {
            if vis_n < 2 || rect.w <= 0.0 {
                return None;
            }
            let t = ((x.clamp(rect.x, rect.x + rect.w) - rect.x) / rect.w).clamp(0.0, 1.0);
            Some(start_ix + ((t * (vis_n - 1) as f32).round() as usize).min(vis_n - 1))
        };
        let (moved, pressed, released, left, wheeled, pinched, touch_drag, touch_finish) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let finish_brush = move |inside: bool, cx: &mut App| {
            released.update(cx, |st, cx| {
                let Some((a, b)) = st.brush.take() else {
                    return;
                };
                if (b - a).abs() > 4.0 {
                    if n >= 3
                        && let (Some(from), Some(to)) = (col_at(a.min(b)), col_at(a.max(b)))
                        && to > from
                    {
                        st.span = (from > 0 || to + 1 < n).then_some((from, to));
                    }
                } else if inside && let Some(ix) = pick_x((a, rect.y + rect.h * 0.5)) {
                    st.pinned = if st.pinned == Some(ix) {
                        None
                    } else {
                        Some(ix)
                    };
                }
                cx.notify();
            })
        };
        let (up_in, up_out) = (finish_brush.clone(), finish_brush);
        let (stroke, hairline) = (sizes.stroke.to_pixels(rem), sizes.hairline);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        root.id((self.id.clone(), "difference"))
            .when(n >= 3, |plot| plot.cursor_crosshair())
            .when(n >= 3 && zoomed.is_none(), |plot| {
                plot.child(
                    canvas(
                        |_, _, _| {},
                        move |_, _, window, _| {
                            let touch = touch_drag.clone();
                            let finish = touch_finish.clone();
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
                                    touch.update(cx, |st, cx| {
                                        st.brush =
                                            Some((start.0, at.0.clamp(rect.x, rect.x + rect.w)));
                                        st.hover = None;
                                        cx.notify();
                                    });
                                    window.prevent_default();
                                    cx.stop_propagation();
                                } else if ev.phase == TouchPhase::Cancelled {
                                    touch.update(cx, |st, cx| {
                                        st.brush = None;
                                        cx.notify();
                                    });
                                } else if touch.read(cx).brush.is_some() {
                                    touch.update(cx, |st, cx| {
                                        if let Some(brush) = st.brush.as_mut() {
                                            brush.1 = at.0.clamp(rect.x, rect.x + rect.w);
                                        }
                                        cx.notify();
                                    });
                                    if ev.phase == TouchPhase::Ended {
                                        finish.update(cx, |st, cx| {
                                            let Some((a, b)) = st.brush.take() else {
                                                return;
                                            };
                                            if (b - a).abs() > 4.0
                                                && let (Some(from), Some(to)) =
                                                    (col_at(a.min(b)), col_at(a.max(b)))
                                                && to > from
                                            {
                                                st.span =
                                                    (from > 0 || to + 1 < n).then_some((from, to));
                                            }
                                            cx.notify();
                                        });
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
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(brush) = st.brush.as_mut()
                    {
                        brush.1 = pos.0.clamp(rect.x, rect.x + rect.w);
                        cx.notify();
                        return;
                    }
                    let next = pick_x(pos);
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                })
            })
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |st, cx| {
                    if event.click_count == 2 {
                        st.span = None;
                        st.pinned = None;
                        st.brush = None;
                        cx.notify();
                        return;
                    }
                    st.hover = pick_x(pos);
                    if rect.contains(pos) && vis_n > 0 {
                        st.brush = Some((pos.0, pos.0));
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| up_in(true, cx))
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| up_out(false, cx))
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |st, cx| {
                        st.hover = None;
                        cx.notify();
                    })
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if n < 3 || rect.w <= 0.0 {
                    return;
                }
                let pos = local(ev.position, wheeled.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                if ev.source == ScrollSource::Touch {
                    if zoomed.is_some() && dx.abs() > dy.abs() && dx.abs() >= 0.01 {
                        wheeled.update(cx, |st, cx| {
                            st.span = pan_span(st.span, n, dx, rect.w);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                    return;
                }
                if dy.abs() < 0.01 {
                    return;
                }
                let factor = (-dy * 0.004).exp().clamp(0.25, 4.0);
                wheeled.update(cx, |st, cx| {
                    st.span = zoom_span(st.span, n, factor, anchor_t);
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                if !ev.delta.is_finite() || ev.phase == TouchPhase::Cancelled {
                    pinched.update(cx, |st, _| st.brush = None);
                    return;
                }
                let scale = ev.delta.exp().clamp(0.1, 10.0);
                if (scale - 1.0).abs() < 0.001 || n < 3 || rect.w <= 0.0 {
                    pinched.update(cx, |st, _| st.brush = None);
                    return;
                }
                let pos = local(ev.position, pinched.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                pinched.update(cx, |st, cx| {
                    st.brush = None;
                    st.span = zoom_span(st.span, n, 1.0 / scale, anchor_t);
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let grid = colors.border.opacity(0.6);
                        for &(y_px, _) in &ticks {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, y_px)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                grid,
                            ));
                        }
                        for (above, poly) in &regions {
                            if poly.len() < 3 {
                                continue;
                            }
                            let mut path = PathBuilder::fill();
                            path.move_to(at(origin, poly[0]));
                            for &pt in &poly[1..] {
                                path.line_to(at(origin, pt));
                            }
                            path.close();
                            let fill_color = if *above {
                                colors.success
                            } else {
                                colors.danger
                            }
                            .opacity(0.24);
                            finish(path, fill_color, window);
                        }
                        for (pts, color) in [(&line_b, colors.fg_subtle), (&line_a, colors.fg)] {
                            if pts.len() < 2 {
                                continue;
                            }
                            let mut path = PathBuilder::stroke(stroke);
                            path.move_to(at(origin, pts[0]));
                            for &pt in &pts[1..] {
                                path.line_to(at(origin, pt));
                            }
                            finish(path, color, window);
                        }
                        if let Some(ix) = active_local {
                            if let Some(&x_px) = xs.get(ix) {
                                window.paint_quad(fill(
                                    gpui::Bounds::new(
                                        at(origin, (x_px, rect.y)),
                                        size(hairline, Pixels::from(rect.h)),
                                    ),
                                    if is_pinned {
                                        colors.fg
                                    } else {
                                        colors.border_strong
                                    },
                                ));
                            }
                            if let Some(&pt) = line_a.get(ix) {
                                ring(
                                    at(origin, pt),
                                    stroke * 2.0,
                                    (colors.bg, colors.fg),
                                    stroke,
                                    window,
                                );
                            }
                        }
                        if let Some((a, b)) = brush
                            && (b - a).abs() > 4.0
                        {
                            let band = Rect {
                                x: a.min(b),
                                y: rect.y,
                                w: (a - b).abs(),
                                h: rect.h,
                            };
                            window.paint_quad(fill(place(origin, band), colors.selection));
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(x_labels)
            .children(y_labels)
            .children(tooltip)
            .children(show_all)
            .child(measure(state, |st| &mut st.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difference_chart_splits_crossing_segments() {
        let scale = Linear::new((0.0, 10.0), (100.0, 0.0));
        let regions = split_difference_regions(&[0.0, 100.0], &[8.0, 2.0], &[2.0, 8.0], &scale);
        assert_eq!(regions.len(), 2);
        assert!(regions[0].0, "first triangle has primary above baseline");
        assert!(!regions[1].0, "second triangle has primary below baseline");
        assert_eq!(regions[0].1[1].0, 50.0, "crossing at midpoint x=50");
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn difference_chart_supports_pin_brush_scroll_zoom_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

        struct DiffHarness;
        impl Render for DiffHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(400.0)).h(px(280.0)).child(
                    DifferenceChart::new("diff-int", ["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
                        .primary(Series::new("Actual", [10.0, 22.0, 14.0, 28.0, 19.0, 32.0]))
                        .baseline(Series::new("Plan", [12.0, 16.0, 18.0, 20.0, 24.0, 26.0])),
                )
            }
        }

        cx.update(Theme::init);
        let (_, vc) = cx.add_window_view(|_, _| DiffHarness);
        vc.simulate_resize(gpui::size(px(400.0), px(280.0)));
        settle(vc);

        let root = vc.debug_bounds("chart-root").expect("difference root");
        let click_at = root.center();
        vc.simulate_mouse_down(click_at, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(click_at, MouseButton::Left, Modifiers::none());
        settle(vc);

        // Moving outside keeps pinned crosshair and tooltip visible.
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "clicking without dragging pins tooltip"
        );

        // Clicking same spot unpins it.
        vc.simulate_mouse_down(click_at, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(click_at, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_none(),
            "clicking again unpins tooltip"
        );

        // Click to pin, then drag horizontally (> 4px) to brush-zoom.
        vc.simulate_mouse_down(click_at, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(click_at, MouseButton::Left, Modifiers::none());
        settle(vc);
        let from = point(root.left() + root.size.width * 0.25, click_at.y);
        let to = point(root.left() + root.size.width * 0.75, click_at.y);
        vc.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
        vc.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-show-all").is_some(),
            "horizontal drag brush zooms span"
        );

        // Show all resets both span and pinned state.
        let show_all = vc.debug_bounds("chart-show-all").expect("show-all button");
        vc.simulate_mouse_down(show_all.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(show_all.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-show-all").is_none());
        assert!(
            vc.debug_bounds("chart-tooltip").is_none(),
            "Show all resets pinned state"
        );

        // Wheel scroll zooms span; double-click resets span and pinned state.
        vc.simulate_event(ScrollEvent {
            position: root.center(),
            delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        });
        settle(vc);
        assert!(
            vc.debug_bounds("chart-show-all").is_some(),
            "wheel scroll zooms span"
        );

        vc.simulate_event(MouseDownEvent {
            position: root.center(),
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_mouse_up(root.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-show-all").is_none(),
            "double-click resets span"
        );
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn difference_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};

        struct NarrowDiff;
        impl Render for NarrowDiff {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(240.0)).h(px(240.0)).child(
                    DifferenceChart::new(
                        "diff-touch",
                        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],
                    )
                    .primary(Series::new(
                        "Actual",
                        [10.0, 22.0, 14.0, 28.0, 19.0, 32.0, 25.0, 34.0],
                    ))
                    .baseline(Series::new(
                        "Plan",
                        [12.0, 16.0, 18.0, 20.0, 24.0, 26.0, 22.0, 29.0],
                    )),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });
        let (_, vc) = cx.add_window_view(|_, _| NarrowDiff);
        vc.simulate_resize(gpui::size(px(240.0), px(240.0)));
        settle(vc);

        let root = vc.debug_bounds("chart-root").expect("difference root");
        // Tap on the right side (> 0.55 * width) pins and flips tooltip inside 240px card.
        let right_tap = point(root.left() + root.size.width * 0.78, root.center().y);
        vc.simulate_mouse_down(right_tap, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(right_tap, MouseButton::Left, Modifiers::none());
        settle(vc);
        let tip = vc.debug_bounds("chart-tooltip").expect("tap pins tooltip");
        assert!(
            tip.right() <= root.right() + px(1.0),
            "right-side tooltip flips inside narrow 240px card"
        );
        let reset_btn = vc
            .debug_bounds("chart-show-all")
            .expect("pinned selection shows reset button");
        assert!(reset_btn.size.width >= px(48.0) && reset_btn.size.height >= px(48.0));
        vc.simulate_click(reset_btn.center(), Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-show-all").is_none());
        assert!(vc.debug_bounds("chart-tooltip").is_none());

        // Single-finger down starts brush, then pinch clears brush so trailing move does not brush-zoom.
        let from = point(root.left() + root.size.width * 0.3, root.center().y);
        let to = point(root.left() + root.size.width * 0.9, root.center().y);
        vc.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        vc.simulate_event(PinchEvent {
            position: root.center(),
            delta: 0.6,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc);
        vc.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
        vc.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-show-all").is_some());

        // Horizontal touch swipe pans when zoomed, and Show all resets.
        vc.simulate_event(ScrollEvent {
            position: root.center(),
            delta: ScrollDelta::Pixels(point(px(30.0), px(0.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc);
        let show_all = vc
            .debug_bounds("chart-show-all")
            .expect("show-all after pan");
        vc.simulate_click(show_all.center(), Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-show-all").is_none());

        // Horizontal touch drag brushes and zooms without requiring an earlier mouse-down.
        vc.simulate_event(gpui::TouchDragEvent {
            phase: TouchPhase::Started,
            start_position: from,
            position: to,
        });
        vc.simulate_event(gpui::TouchDragEvent {
            phase: TouchPhase::Ended,
            start_position: from,
            position: to,
        });
        settle(vc);
        assert!(vc.debug_bounds("chart-show-all").is_some());
    }
}
