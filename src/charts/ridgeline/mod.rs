use std::{collections::HashSet, fmt::Write as _, rc::Rc};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, fill, size,
};

use super::{
    axes::{PAD, anchored, below, beside},
    geometry::{Rect, frame},
    paint::{at, finish, measure, tint, trace},
    parts::{ChartLegend, ChartTooltip},
    plot::Format,
    scale::{Linear, compact, nice},
    series::drawable,
    stats::{density, five},
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{Ellipsis, tabular, text_width},
};

#[derive(Default)]
pub(super) struct RidgeState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) pinned: Option<usize>,
    pub(super) hidden: HashSet<SharedString>,
    pub(super) x_domain: Option<(f64, f64)>,
    pub(super) drag_start: Option<(f32, f32)>,
    pub(super) drag_last: Option<(f32, f32)>,
    pub(super) dragged: bool,
}

/// Stacked overlapping kernel-density distribution curves (joyplot).
#[derive(IntoElement)]
pub struct RidgelineChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) groups: Vec<(SharedString, Vec<f64>)>,
    pub(super) overlap: f32,
    pub(super) samples: usize,
    pub(super) format: Format,
}

impl RidgelineChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            groups: Vec::new(),
            overlap: 1.5,
            samples: 48,
            format: Rc::new(compact),
        }
    }

    /// Add a named distribution group (at least 2 finite values).
    pub fn group(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert!(
            values.len() >= 2 && values.iter().all(|v| drawable(*v)),
            "a ridgeline group needs two values, within charts::LIMIT"
        );
        self.groups.push((label.into(), values));
        self
    }

    /// Vertical peak overlap multiplier relative to row spacing (default `1.5`).
    pub fn overlap(mut self, overlap: f32) -> Self {
        assert!(
            overlap.is_finite() && overlap > 0.0,
            "overlap must be positive"
        );
        self.overlap = overlap.clamp(0.5, 3.0);
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    pub fn csv(&self) -> String {
        let mut out = String::from("group,low,median,high,count");
        for (name, vals) in &self.groups {
            let stats = five(vals);
            write!(
                out,
                "\n{},{},{},{},{}",
                name,
                stats.low,
                stats.median,
                stats.high,
                vals.len()
            )
            .expect("csv write");
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
        let layout = compute_ridges(&self.groups, rect, self.samples, self.overlap, None);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        for (ix, ridge) in layout.ridges.iter().enumerate() {
            let rgba: gpui::Rgba = tint(colors, ix).into();
            let hex = format!(
                "#{:02x}{:02x}{:02x}",
                (rgba.r * 255.0).round() as u8,
                (rgba.g * 255.0).round() as u8,
                (rgba.b * 255.0).round() as u8
            );
            let mut d = String::new();
            if let Some(first) = ridge.points.first() {
                write!(d, "M{:.1},{:.1}", first.0, ridge.baseline).expect("svg");
                for &(x, y) in &ridge.points {
                    write!(d, " L{x:.1},{y:.1}").expect("svg");
                }
                if let Some(last) = ridge.points.last() {
                    write!(d, " L{:.1},{:.1} Z", last.0, ridge.baseline).expect("svg");
                }
                write!(
                    out,
                    r#"<path d="{d}" fill="{hex}" fill-opacity="0.35" stroke="{hex}" stroke-width="1.5"/>"#,
                )
                .expect("svg");
            }
        }
        out.push_str("</svg>");
        out
    }
}

pub(super) struct Ridge {
    pub(super) baseline: f32,
    pub(super) points: Vec<(f32, f32)>,
    pub(super) median: f64,
    pub(super) q1: f64,
    pub(super) q3: f64,
}

pub(super) struct RidgeLayout {
    pub(super) ridges: Vec<Ridge>,
    pub(super) x_ticks: Vec<(f32, f64)>,
    pub(super) base_domain: (f64, f64),
    pub(super) active_domain: (f64, f64),
}

pub(super) fn compute_ridges(
    groups: &[(SharedString, Vec<f64>)],
    rect: Rect,
    samples: usize,
    overlap: f32,
    custom_x: Option<(f64, f64)>,
) -> RidgeLayout {
    if groups.is_empty() || rect.w <= 0.0 || rect.h <= 0.0 {
        return RidgeLayout {
            ridges: Vec::new(),
            x_ticks: Vec::new(),
            base_domain: (0.0, 1.0),
            active_domain: (0.0, 1.0),
        };
    }
    let (lo, hi) = groups
        .iter()
        .flat_map(|(_, vals)| vals.iter().copied())
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(v), b.max(v))
        });
    let ((base_x0, base_x1), base_ticks) = nice(lo, hi, 5);
    let (act_domain, x_tick_vals) = if let Some((cx0, cx1)) = custom_x {
        let (_, ticks) = nice(cx0, cx1, 5);
        let filtered = ticks
            .into_iter()
            .filter(|&t| t >= cx0.min(cx1) && t <= cx0.max(cx1))
            .collect();
        ((cx0, cx1), filtered)
    } else {
        ((base_x0, base_x1), base_ticks)
    };
    let x_scale = Linear::new(act_domain, (rect.x, rect.x + rect.w));
    let densities: Vec<Vec<(f64, f64)>> = groups
        .iter()
        .map(|(_, vals)| density(vals, samples))
        .collect();
    let max_d = densities
        .iter()
        .flat_map(|curve| curve.iter().map(|&(_, d)| d))
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    let step = rect.h / groups.len() as f32;
    let peak_h = step * overlap;
    let ridges = groups
        .iter()
        .zip(densities)
        .enumerate()
        .map(|(ix, ((_, vals), curve))| {
            let baseline = rect.y + step * (ix as f32 + 0.85);
            let points = curve
                .into_iter()
                .map(|(vx, vd)| {
                    let px = x_scale.at(vx).clamp(rect.x, rect.x + rect.w);
                    let py = baseline - (vd / max_d) as f32 * peak_h;
                    (px, py.max(rect.y))
                })
                .collect();
            let stats = five(vals);
            Ridge {
                baseline,
                points,
                median: stats.median,
                q1: stats.q1,
                q3: stats.q3,
            }
        })
        .collect();
    let x_ticks = x_tick_vals
        .into_iter()
        .map(|v| (x_scale.at(v), v))
        .collect();
    RidgeLayout {
        ridges,
        x_ticks,
        base_domain: (base_x0, base_x1),
        active_domain: act_domain,
    }
}

mod render;
#[cfg(test)]
mod tests;
