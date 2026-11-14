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
pub(super) struct DifferenceState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) pinned: Option<usize>,
    pub(super) span: Option<(usize, usize)>,
    pub(super) brush: Option<(f32, f32)>,
}

pub(super) fn local(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

pub(super) fn zoom_span(
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

pub(super) fn pan_span(
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
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) labels: Vec<SharedString>,
    pub(super) primary: Option<Series>,
    pub(super) baseline: Option<Series>,
    pub(super) format: Format,
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

pub(super) struct DiffLayout {
    pub(super) regions: Vec<(bool, Vec<(f32, f32)>)>,
    pub(super) line_a: Vec<(f32, f32)>,
    pub(super) line_b: Vec<(f32, f32)>,
    pub(super) xs: Vec<f32>,
    pub(super) ticks: Vec<(f32, f64)>,
}

/// Split segments at crossing points into closed polygons tagged `(primary >= baseline, polygon)`.
pub(super) fn split_difference_regions(
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

pub(super) fn compute_diff(
    labels: &[SharedString],
    a_vals: &[f64],
    b_vals: &[f64],
    rect: Rect,
) -> DiffLayout {
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

mod render;
#[cfg(test)]
mod tests;
