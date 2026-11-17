use std::{
    collections::HashSet,
    f32::consts::{FRAC_PI_2, TAU},
    fmt::Write as _,
    rc::Rc,
};

use gpui::{
    App, Bounds, DispatchPhase, Div, ElementId, Entity, HoverListenerMode, Hsla,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, PathBuilder, PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent,
    ScrollSource, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    TouchDragEvent, TouchPhase, Window, canvas, div, fill, prelude::FluentBuilder as _, size,
};

use super::{
    axes::{PAD, anchored, below, beside},
    geometry::{Rect, frame},
    paint::{at, finish, measure, place, ring, tint},
    parts::{ChartLegend, ChartTooltip},
    pie::{bearing, toward},
    plot::Format,
    scale::{compact, nice},
    series::{Series, csv as series_csv, drawable},
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{Ellipsis, tabular, text_width},
};

pub(super) fn svg_hex(color: Hsla) -> String {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgba.r),
        byte(rgba.g),
        byte(rgba.b)
    )
}

pub(super) fn svg_escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn band_alpha(band: usize, bands: usize) -> f32 {
    0.22 + 0.66 * ((band + 1) as f32 / bands.max(1) as f32)
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

#[derive(Default)]
pub(super) struct HorizonState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) pinned: Option<(usize, usize)>,
    pub(super) hidden: HashSet<SharedString>,
    pub(super) span: Option<(usize, usize)>,
    pub(super) brush: Option<(f32, f32)>,
}

#[derive(Default)]
pub(super) struct RadialState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) pinned: Option<usize>,
    pub(super) hidden: HashSet<SharedString>,
    pub(super) rotation: f32,
    pub(super) zoom: f32,
    pub(super) drag: Option<(f32, (f32, f32), bool)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BandPatch {
    pub(super) band: usize,
    pub(super) positive: bool,
    pub(super) points: Vec<(f32, f32)>,
}

pub(super) struct HorizonRow {
    pub(super) rect: Rect,
    pub(super) patches: Vec<BandPatch>,
}

pub(super) struct HorizonLayout {
    pub(super) rows: Vec<HorizonRow>,
    pub(super) xs: Vec<f32>,
    pub(super) band_step: f64,
    pub(super) max_dev: f64,
}

pub(super) fn split_zero_crossings(xs: &[f32], deltas: &[f64]) -> Vec<(f32, f64)> {
    let mut out = Vec::with_capacity(xs.len() * 2);
    for ix in 0..xs.len() {
        let (x0, d0) = (xs[ix], deltas[ix]);
        out.push((x0, d0));
        if ix + 1 < xs.len() {
            let (x1, d1) = (xs[ix + 1], deltas[ix + 1]);
            if (d0 > 0.0 && d1 < 0.0) || (d0 < 0.0 && d1 > 0.0) {
                let t = (d0.abs() / (d0.abs() + d1.abs())) as f32;
                out.push((x0 + (x1 - x0) * t, 0.0));
            }
        }
    }
    out
}

pub(super) fn build_band_patches(
    xs: &[f32],
    deltas: &[f64],
    row: Rect,
    bands: usize,
    band_step: f64,
    mirror: bool,
) -> Vec<BandPatch> {
    if xs.is_empty() || deltas.is_empty() || row.w <= 0.0 || row.h <= 0.0 || band_step <= 0.0 {
        return Vec::new();
    }
    let (sample_xs, sample_deltas): (Vec<f32>, Vec<f64>) = if xs.len() == 1 {
        (vec![row.x, row.x + row.w], vec![deltas[0], deltas[0]])
    } else {
        (xs.to_vec(), deltas.to_vec())
    };
    let signed = split_zero_crossings(&sample_xs, &sample_deltas);
    let mut patches = Vec::new();

    for positive in [true, false] {
        let mags: Vec<(f32, f64)> = signed
            .iter()
            .map(|&(x, d)| (x, if positive { d.max(0.0) } else { (-d).max(0.0) }))
            .collect();
        let grow_up = positive || mirror;
        let y_base = if grow_up { row.y + row.h } else { row.y };
        let y_of = |frac: f32| -> f32 {
            if grow_up {
                y_base - frac * row.h
            } else {
                y_base + frac * row.h
            }
        };

        for band in 0..bands {
            let lo = band as f64 * band_step;
            let hi = (band + 1) as f64 * band_step;
            let frac_at = |m: f64| ((m - lo) / band_step).clamp(0.0, 1.0) as f32;

            let mut dense: Vec<(f32, f32)> = Vec::with_capacity(mags.len() * 2);
            for ix in 0..mags.len() {
                let (x0, m0) = mags[ix];
                dense.push((x0, frac_at(m0)));
                if ix + 1 < mags.len() {
                    let (x1, m1) = mags[ix + 1];
                    let span = m1 - m0;
                    if span.abs() > 0.0 {
                        let mut ts = [0.0_f64; 2];
                        let mut count = 0;
                        for thresh in [lo, hi] {
                            let crosses =
                                (m0 < thresh && m1 > thresh) || (m0 > thresh && m1 < thresh);
                            if crosses {
                                ts[count] = (thresh - m0) / span;
                                count += 1;
                            }
                        }
                        if count == 2 && ts[0] > ts[1] {
                            ts.swap(0, 1);
                        }
                        for &t in &ts[..count] {
                            let tx = x0 + (x1 - x0) * t as f32;
                            let tm = m0 + span * t;
                            dense.push((tx, frac_at(tm)));
                        }
                    }
                }
            }

            let mut run: Vec<(f32, f32)> = Vec::new();
            for ix in 0..dense.len() {
                let (x, f) = dense[ix];
                if f > 0.0 {
                    if run.is_empty() && ix > 0 {
                        run.push(dense[ix - 1]);
                    }
                    run.push((x, f));
                } else if !run.is_empty() {
                    run.push((x, 0.0));
                    let mut poly: Vec<(f32, f32)> =
                        run.iter().map(|&(rx, rf)| (rx, y_of(rf))).collect();
                    let first_x = run[0].0;
                    let last_x = run[run.len() - 1].0;
                    poly.push((last_x, y_base));
                    poly.push((first_x, y_base));
                    patches.push(BandPatch {
                        band,
                        positive,
                        points: poly,
                    });
                    run.clear();
                }
            }
            if !run.is_empty() {
                let first_x = run[0].0;
                let last_x = run[run.len() - 1].0;
                let mut poly: Vec<(f32, f32)> =
                    run.iter().map(|&(rx, rf)| (rx, y_of(rf))).collect();
                poly.push((last_x, y_base));
                poly.push((first_x, y_base));
                patches.push(BandPatch {
                    band,
                    positive,
                    points: poly,
                });
            }
        }
    }
    patches
}

pub(super) fn compute_horizon(
    series: &[&Series],
    span: (usize, usize),
    rect: Rect,
    bands: usize,
    baseline: f64,
    mirror: bool,
) -> HorizonLayout {
    if series.is_empty()
        || rect.w <= 0.0
        || rect.h <= 0.0
        || span.0 > span.1
        || series.iter().any(|s| span.1 >= s.values.len())
    {
        return HorizonLayout {
            rows: Vec::new(),
            xs: Vec::new(),
            band_step: 1.0,
            max_dev: 0.0,
        };
    }
    let labels_len = span.1 - span.0 + 1;
    let xs: Vec<f32> = if labels_len == 1 {
        vec![rect.x + rect.w * 0.5]
    } else {
        (0..labels_len)
            .map(|ix| rect.x + rect.w * (ix as f32 / (labels_len - 1) as f32))
            .collect()
    };
    let max_dev = series
        .iter()
        .flat_map(|s| s.values[span.0..=span.1].iter())
        .map(|v| (*v - baseline).abs())
        .fold(0.0_f64, f64::max);
    let band_step = if max_dev > 0.0 {
        max_dev / bands as f64
    } else {
        1.0
    };
    let row_count = series.len();
    let slot_h = rect.h / row_count as f32;
    let gap = (slot_h * 0.12).min(4.0);
    let rows = series
        .iter()
        .enumerate()
        .map(|(ix, s)| {
            let row_rect = Rect {
                x: rect.x,
                y: rect.y + slot_h * ix as f32 + gap * 0.5,
                w: rect.w,
                h: (slot_h - gap).max(1.0),
            };
            let deltas: Vec<f64> = s.values[span.0..=span.1]
                .iter()
                .map(|v| *v - baseline)
                .collect();
            let patches = build_band_patches(&xs, &deltas, row_rect, bands, band_step, mirror);
            HorizonRow {
                rect: row_rect,
                patches,
            }
        })
        .collect();
    HorizonLayout {
        rows,
        xs,
        band_step,
        max_dev,
    }
}

mod chart;
mod paint;
mod radial;
mod radial_render;
mod render;
#[cfg(test)]
mod tests;

pub use chart::HorizonChart;
pub use radial::RadialLineChart;
use radial::*;
