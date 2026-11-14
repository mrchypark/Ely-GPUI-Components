use std::{
    collections::HashSet,
    f32::consts::{FRAC_PI_2, TAU},
    fmt::Write as _,
    rc::Rc,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchPhase, Window, canvas, div, fill,
    prelude::FluentBuilder as _, size,
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

fn svg_hex(color: Hsla) -> String {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgba.r),
        byte(rgba.g),
        byte(rgba.b)
    )
}

fn svg_escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn band_alpha(band: usize, bands: usize) -> f32 {
    0.22 + 0.66 * ((band + 1) as f32 / bands.max(1) as f32)
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

#[derive(Default)]
struct HorizonState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    pinned: Option<(usize, usize)>,
    hidden: HashSet<SharedString>,
    span: Option<(usize, usize)>,
    brush: Option<(f32, f32)>,
}

#[derive(Default)]
struct RadialState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    pinned: Option<usize>,
    hidden: HashSet<SharedString>,
    rotation: f32,
    zoom: f32,
    drag: Option<(f32, (f32, f32), bool)>,
}

#[derive(Clone, Debug, PartialEq)]
struct BandPatch {
    band: usize,
    positive: bool,
    points: Vec<(f32, f32)>,
}

struct HorizonRow {
    rect: Rect,
    patches: Vec<BandPatch>,
}

struct HorizonLayout {
    rows: Vec<HorizonRow>,
    xs: Vec<f32>,
    band_step: f64,
    max_dev: f64,
}

fn split_zero_crossings(xs: &[f32], deltas: &[f64]) -> Vec<(f32, f64)> {
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

fn build_band_patches(
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

fn compute_horizon(
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

/// Compact multi-band horizon chart stacking positive and negative deviations across rows.
#[derive(IntoElement)]
pub struct HorizonChart {
    base: Div,
    id: ElementId,
    labels: Vec<SharedString>,
    series: Vec<Series>,
    bands: usize,
    baseline: f64,
    mirror: bool,
    format: Format,
}

impl HorizonChart {
    pub fn new(
        id: impl Into<ElementId>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            bands: 3,
            baseline: 0.0,
            mirror: true,
            format: Rc::new(compact),
        }
    }

    /// Append a series row (must provide one value per label).
    pub fn series(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.labels.len(),
            "horizon series {} needs a value per label",
            series.name
        );
        self.series.push(series);
        self
    }

    /// Number of folded amplitude bands per row, clamped to 1..=6 (default 3).
    pub fn bands(mut self, bands: usize) -> Self {
        self.bands = bands.clamp(1, 6);
        self
    }

    /// Reference baseline value (default 0.0).
    pub fn baseline(mut self, baseline: f64) -> Self {
        assert!(
            drawable(baseline),
            "a horizon baseline needs a finite value, within charts::LIMIT"
        );
        self.baseline = baseline;
        self
    }

    /// Mirror negative deviations upward from the row baseline when true (default), or hang downward when false.
    pub fn mode_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    /// Format numeric values in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export series rows as CSV.
    pub fn csv(&self) -> String {
        series_csv(&self.labels, &self.series)
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
        let refs: Vec<&Series> = self.series.iter().collect();
        let layout = compute_horizon(
            &refs,
            (0, self.labels.len().saturating_sub(1)),
            rect,
            self.bands,
            self.baseline,
            self.mirror,
        );
        let bg_hex = svg_hex(colors.bg);
        let pos_hex = svg_hex(colors.chart[0]);
        let neg_hex = svg_hex(colors.danger);
        let grid_hex = svg_hex(colors.border);
        let label_hex = svg_hex(colors.fg_subtle);
        let text_px = pixels(theme.text_size(TextSize::Xs));
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-size="{text_px}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for (ix, row) in layout.rows.iter().enumerate() {
            for patch in &row.patches {
                if patch.points.len() < 3 {
                    continue;
                }
                let mut d = String::new();
                for (p_ix, &(px, py)) in patch.points.iter().enumerate() {
                    let cmd = if p_ix == 0 { 'M' } else { 'L' };
                    write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
                }
                d.push('Z');
                let fill_hex = if patch.positive { &pos_hex } else { &neg_hex };
                let alpha = band_alpha(patch.band, self.bands);
                write!(
                    out,
                    r#"<path d="{d}" fill="{fill_hex}" fill-opacity="{alpha:.2}"/>"#
                )
                .expect("svg write");
            }
            let y_line = row.rect.y + row.rect.h;
            write!(
                out,
                r#"<line x1="{:.1}" y1="{y_line:.1}" x2="{:.1}" y2="{y_line:.1}" stroke="{grid_hex}" stroke-width="1"/>"#,
                row.rect.x,
                row.rect.x + row.rect.w
            )
            .expect("svg write");
            write!(
                out,
                r#"<text x="{:.1}" y="{:.1}" text-anchor="end" fill="{label_hex}">{}</text>"#,
                rect.x - 8.0,
                row.rect.y + row.rect.h * 0.5 + text_px * 0.35,
                svg_escaped(&self.series[ix].name)
            )
            .expect("svg write");
        }
        let every = ((self.labels.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        for (ix, &x_px) in layout.xs.iter().enumerate() {
            if ix % every == 0 {
                write!(
                    out,
                    r#"<text x="{x_px:.1}" y="{:.1}" text-anchor="middle" fill="{label_hex}">{}</text>"#,
                    rect.y + rect.h + text_px * 1.6,
                    svg_escaped(&self.labels[ix])
                )
                .expect("svg write");
            }
        }
        out.push_str("</svg>");
        out
    }
}

impl Styled for HorizonChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for HorizonChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<HorizonState> =
            window.use_keyed_state((self.id.clone(), "horizon"), cx, |_, _| {
                HorizonState::default()
            });
        let (bounds, state_hover, state_pinned, hidden, raw_span, brush) = {
            let st = state.read(cx);
            (
                st.bounds,
                st.hover,
                st.pinned,
                st.hidden.clone(),
                st.span,
                st.brush,
            )
        };
        let total_cols = self.labels.len();
        let shown: Vec<&Series> = self
            .series
            .iter()
            .filter(|s| !hidden.contains(&s.name))
            .collect();
        let zoomed = raw_span.filter(|(a, b)| *a < *b && *b < total_cols);
        let effective_span = zoomed.unwrap_or((0, total_cols.saturating_sub(1)));
        let (start_col, end_col) = effective_span;
        let n_rows = shown.len();
        let n_cols = if total_cols == 0 {
            0
        } else {
            end_col - start_col + 1
        };
        let hover = state_hover.filter(|ix| n_rows > 0 && n_cols > 0 && *ix < n_rows * n_cols);
        let pinned = state_pinned
            .filter(|&(r, c)| n_rows > 0 && r < n_rows && c >= start_col && c <= end_col);
        let active = pinned.map(|(r, c)| r * n_cols + (c - start_col)).or(hover);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let text_size = theme.text_size(TextSize::Xs).to_pixels(rem);
        let widest = shown
            .iter()
            .map(|series| f32::from(text_width(&series.name, text_size, window)))
            .fold(0.0, f32::max);
        let gutter = (widest + pixels(PAD) + 4.0)
            .max(pixels(sizes.gutter))
            .min(box_size.0 / 2.0);
        let rect = frame(box_size, gutter, pixels(sizes.foot), pixels(sizes.inset));
        let layout = compute_horizon(
            &shown,
            effective_span,
            rect,
            self.bands,
            self.baseline,
            self.mirror,
        );
        let format = self.format.clone();
        let label_color = colors.fg_subtle;
        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(label_color)
                .child(content)
        };
        let every =
            ((n_cols as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil() as usize).max(1);
        let x_labels: Vec<_> = layout
            .xs
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every == 0)
            .map(|(ix, &x_px)| below(x_px, rect, text(self.labels[start_col + ix].clone())))
            .collect();
        let y_labels: Vec<_> = layout
            .rows
            .iter()
            .zip(&shown)
            .map(|(row, series)| {
                let name = div()
                    .debug_selector(|| "horizon-row-label".into())
                    .min_w_0()
                    .text_right()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(label_color)
                    .child(Ellipsis::new(series.name.clone()));
                beside(row.rect.y + row.rect.h * 0.5, Pixels::from(gutter), name)
            })
            .collect();

        let tooltip = active.and_then(|hit_ix| {
            if n_cols == 0 || n_rows == 0 {
                return None;
            }
            let row_ix = (hit_ix / n_cols).min(n_rows - 1);
            let local_col = hit_ix % n_cols;
            let col_ix = start_col + local_col;
            let x_px = *layout.xs.get(local_col)?;
            let row = layout.rows.get(row_ix)?;
            let val = shown[row_ix].values[col_ix];
            let delta = val - self.baseline;
            let mag = delta.abs();
            let band_num = if mag <= 0.0 || layout.max_dev <= 0.0 {
                0
            } else {
                ((mag / layout.band_step).ceil() as usize).clamp(1, self.bands)
            };
            let sign = if delta >= 0.0 { "+" } else { "-" };
            let band_color = if delta >= 0.0 {
                colors.chart[0]
            } else {
                colors.danger
            };
            let mut card = ChartTooltip::new(self.labels[col_ix].clone()).row(
                Some(band_color.opacity(band_alpha(band_num.saturating_sub(1), self.bands))),
                format!("{} band", shown[row_ix].name),
                format!("{band_num}/{} ({sign})", self.bands),
            );
            for series in &shown {
                let v = series.values[col_ix];
                let dot = if v >= self.baseline {
                    colors.chart[0]
                } else {
                    colors.danger
                };
                card = card.row(Some(dot), series.name.clone(), format(v));
            }
            Some(anchored(
                (x_px, row.rect.y + row.rect.h * 0.5),
                x_px > box_size.0 * 0.55,
                card,
            ))
        });

        let legend = (self.series.len() > 1).then(|| {
            let toggled = state.clone();
            let legend = self.series.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "legend")),
                |legend, (ix, series)| {
                    legend.entry(
                        tint(&colors, ix),
                        series.name.clone(),
                        hidden.contains(&series.name),
                    )
                },
            );
            legend.on_toggle(move |name, _, cx| {
                toggled.update(cx, |st, cx| {
                    if !st.hidden.remove(name) {
                        st.hidden.insert(name.clone());
                    }
                    cx.notify();
                })
            })
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

        let rows = layout.rows;
        let xs = layout.xs;
        let bands = self.bands;
        let hovered_rc = active.and_then(|hit_ix| {
            (n_cols > 0 && n_rows > 0)
                .then_some(((hit_ix / n_cols).min(n_rows - 1), hit_ix % n_cols))
        });
        let is_pinned = pinned.is_some();
        let pick_rc = move |pos: (f32, f32)| -> Option<usize> {
            if !rect.contains(pos) || n_rows == 0 || n_cols == 0 || rect.w <= 0.0 || rect.h <= 0.0 {
                return None;
            }
            let row = (((pos.1 - rect.y) / rect.h) * n_rows as f32).floor() as usize;
            let row = row.min(n_rows - 1);
            let col = if n_cols == 1 {
                0
            } else {
                let t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                ((t * (n_cols - 1) as f32).round() as usize).min(n_cols - 1)
            };
            Some(row * n_cols + col)
        };
        let col_at = move |x: f32| -> Option<usize> {
            if n_cols < 2 || rect.w <= 0.0 {
                return None;
            }
            let t = ((x.clamp(rect.x, rect.x + rect.w) - rect.x) / rect.w).clamp(0.0, 1.0);
            Some(start_col + ((t * (n_cols - 1) as f32).round() as usize).min(n_cols - 1))
        };
        let (moved, pressed, released, up_bounds, left, wheeled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let finish_brush = move |pos: Option<(f32, f32)>, cx: &mut App| {
            released.update(cx, |st, cx| {
                let Some((a, b)) = st.brush.take() else {
                    return;
                };
                if (b - a).abs() > 4.0
                    && total_cols >= 3
                    && let (Some(from), Some(to)) = (col_at(a.min(b)), col_at(a.max(b)))
                    && to > from
                {
                    st.span = (from > 0 || to + 1 < total_cols).then_some((from, to));
                } else if let Some(tap) = pos
                    && let Some(hit_ix) = pick_rc(tap)
                    && n_cols > 0
                    && n_rows > 0
                {
                    let row_ix = (hit_ix / n_cols).min(n_rows - 1);
                    let col_ix = start_col + (hit_ix % n_cols);
                    let rc = (row_ix, col_ix);
                    st.pinned = if st.pinned == Some(rc) {
                        None
                    } else {
                        Some(rc)
                    };
                }
                cx.notify();
            })
        };
        let (up_in, up_out) = (finish_brush.clone(), finish_brush);
        let hairline = sizes.hairline;
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let plot = div()
            .id((self.id.clone(), "horizon"))
            .debug_selector(|| "horizon-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .when(total_cols >= 3, |plot| plot.cursor_crosshair())
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(brush) = st.brush.as_mut()
                    {
                        if total_cols >= 3 {
                            brush.1 = pos.0.clamp(rect.x, rect.x + rect.w);
                        }
                        cx.notify();
                        return;
                    }
                    let next = pick_rc(pos);
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
                    st.hover = pick_rc(pos);
                    if rect.contains(pos) && n_rows > 0 && n_cols > 0 {
                        st.brush = Some((pos.0, pos.0));
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let pos = local(event.position, up_bounds.read(cx).bounds);
                up_in(Some(pos), cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| up_out(None, cx))
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
                if total_cols < 3 || rect.w <= 0.0 {
                    return;
                }
                let pos = local(ev.position, wheeled.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                if ev.source == ScrollSource::Touch {
                    if zoomed.is_some() && dx.abs() > dy.abs() && dx.abs() >= 0.01 {
                        wheeled.update(cx, |st, cx| {
                            st.span = pan_span(st.span, total_cols, dx, rect.w);
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
                    st.span = zoom_span(st.span, total_cols, factor, anchor_t);
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
                if (scale - 1.0).abs() < 0.001 || total_cols < 3 || rect.w <= 0.0 {
                    pinched.update(cx, |st, _| st.brush = None);
                    return;
                }
                let pos = local(ev.position, pinched.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                pinched.update(cx, |st, cx| {
                    st.brush = None;
                    st.span = zoom_span(st.span, total_cols, 1.0 / scale, anchor_t);
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let pos_color = colors.chart[0];
                        let neg_color = colors.danger;
                        let grid = colors.border.opacity(0.6);

                        for (r_ix, row) in rows.iter().enumerate() {
                            let row_hovered = hovered_rc.is_some_and(|(hr, _)| hr == r_ix);
                            for patch in &row.patches {
                                if patch.points.len() < 3 {
                                    continue;
                                }
                                let mut path = PathBuilder::fill();
                                path.move_to(at(origin, patch.points[0]));
                                for &pt in &patch.points[1..] {
                                    path.line_to(at(origin, pt));
                                }
                                path.close();
                                let base_color = if patch.positive { pos_color } else { neg_color };
                                let alpha = if row_hovered {
                                    (band_alpha(patch.band, bands) + 0.08).min(0.95)
                                } else {
                                    band_alpha(patch.band, bands)
                                };
                                finish(path, base_color.opacity(alpha), window);
                            }
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (row.rect.x, row.rect.y + row.rect.h)),
                                    size(Pixels::from(row.rect.w), hairline),
                                ),
                                if row_hovered {
                                    colors.border_strong
                                } else {
                                    grid
                                },
                            ));
                        }
                        if let Some((_, col_ix)) = hovered_rc
                            && let Some(&x_px) = xs.get(col_ix)
                        {
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
                        if let Some((a, b)) = brush
                            && total_cols >= 3
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
            .child(measure(state, |st| &mut st.bounds));
        root.flex().flex_col().gap_2().children(legend).child(plot)
    }
}

/// Polar coordinate cyclic line and area chart with optional min/max envelope band.
#[derive(IntoElement)]
pub struct RadialLineChart {
    base: Div,
    id: ElementId,
    labels: Vec<SharedString>,
    series: Vec<Series>,
    range_band: Option<(Vec<f64>, Vec<f64>)>,
    filled: bool,
    format: Format,
}

impl RadialLineChart {
    pub fn new(
        id: impl Into<ElementId>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            range_band: None,
            filled: false,
            format: Rc::new(compact),
        }
    }

    /// Append a cyclic data series (must provide one value per label).
    pub fn series(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.labels.len(),
            "radial series {} needs a value per label",
            series.name
        );
        self.series.push(series);
        self
    }

    /// Optional min/max envelope band around the cycle.
    pub fn range_band(
        mut self,
        low_values: impl IntoIterator<Item = f64>,
        high_values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let low: Vec<f64> = low_values.into_iter().collect();
        let high: Vec<f64> = high_values.into_iter().collect();
        assert_eq!(
            low.len(),
            self.labels.len(),
            "radial range_band low values need one entry per label"
        );
        assert_eq!(
            high.len(),
            self.labels.len(),
            "radial range_band high values need one entry per label"
        );
        assert!(
            low.iter().chain(&high).all(|v| drawable(*v)),
            "radial range_band values must be finite, within charts::LIMIT"
        );
        self.range_band = Some((low, high));
        self
    }

    /// Fill each series loop down to the inner ring when true.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Format numeric values on rings and in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export series (and range band, if present) as CSV.
    pub fn csv(&self) -> String {
        if let Some((low, high)) = &self.range_band {
            let mut all = self.series.clone();
            all.push(Series::new("Low", low.iter().copied()));
            all.push(Series::new("High", high.iter().copied()));
            series_csv(&self.labels, &all)
        } else {
            series_csv(&self.labels, &self.series)
        }
    }

    /// Export the chart as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let room = pixels(sizes.foot);
        let center = (width * 0.5, height * 0.5);
        let outer = (width.min(height) * 0.5 - room).max(1.0);
        let refs: Vec<&Series> = self.series.iter().collect();
        let layout = compute_radial(
            self.labels.len(),
            &refs,
            self.range_band.as_ref(),
            center,
            outer,
            0.0,
        );
        let bg_hex = svg_hex(colors.bg);
        let grid_hex = svg_hex(colors.border);
        let label_hex = svg_hex(colors.fg_subtle);
        let text_px = pixels(theme.text_size(TextSize::Xs));
        let stroke_px = pixels(sizes.stroke);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-size="{text_px}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for &(r, _) in &layout.rings {
            write!(
                out,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{r:.1}" fill="none" stroke="{grid_hex}" stroke-opacity="0.6" stroke-width="1"/>"#,
                center.0, center.1
            )
            .expect("svg write");
        }
        let n = self.labels.len();
        let every = if n == 0 {
            1
        } else {
            ((n as f32 * pixels(sizes.label) / (TAU * outer).max(1.0)).ceil() as usize).max(1)
        };
        for ix in (0..n).step_by(every) {
            let a = radial_angle(ix, n, 0.0);
            let p0 = (
                center.0 + layout.inner * a.cos(),
                center.1 + layout.inner * a.sin(),
            );
            let p1 = (center.0 + outer * a.cos(), center.1 + outer * a.sin());
            write!(
                out,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{grid_hex}" stroke-opacity="0.5" stroke-width="1"/>"#,
                p0.0, p0.1, p1.0, p1.1
            )
            .expect("svg write");
            let lx = center.0 + (outer + 10.0) * a.cos();
            let ly = center.1 + (outer + 10.0) * a.sin() + text_px * 0.35;
            let anchor = if a.cos() > 0.3 {
                "start"
            } else if a.cos() < -0.3 {
                "end"
            } else {
                "middle"
            };
            write!(
                out,
                r#"<text x="{lx:.1}" y="{ly:.1}" text-anchor="{anchor}" fill="{label_hex}">{}</text>"#,
                svg_escaped(&self.labels[ix])
            )
            .expect("svg write");
        }
        if let Some((ref low_pts, ref high_pts)) = layout.band
            && low_pts.len() >= 2
        {
            let band_hex = svg_hex(tint(colors, 0));
            let mut d = String::new();
            for (ix, &(px, py)) in high_pts.iter().enumerate() {
                let cmd = if ix == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
            }
            write!(d, "L{:.1},{:.1} ", high_pts[0].0, high_pts[0].1).expect("svg write");
            write!(d, "L{:.1},{:.1} ", low_pts[0].0, low_pts[0].1).expect("svg write");
            for &(px, py) in low_pts.iter().rev() {
                write!(d, "L{px:.1},{py:.1} ").expect("svg write");
            }
            d.push('Z');
            write!(
                out,
                r#"<path d="{d}" fill="{band_hex}" fill-opacity="0.18"/>"#
            )
            .expect("svg write");
        }
        for (s_ix, pts) in layout.series_pts.iter().enumerate() {
            if pts.len() < 2 {
                continue;
            }
            let hex = svg_hex(tint(colors, s_ix));
            let mut d = String::new();
            for (ix, &(px, py)) in pts.iter().enumerate() {
                let cmd = if ix == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
            }
            d.push('Z');
            let fill_attr = if self.filled {
                format!(r#"fill="{hex}" fill-opacity="0.18""#)
            } else {
                r#"fill="none""#.to_string()
            };
            write!(
                out,
                r#"<path d="{d}" {fill_attr} stroke="{hex}" stroke-width="{stroke_px:.1}" stroke-linejoin="round"/>"#
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

type PointRing = Vec<(f32, f32)>;

struct RadialLayout {
    inner: f32,
    rings: Vec<(f32, f64)>,
    band: Option<(PointRing, PointRing)>,
    series_pts: Vec<PointRing>,
}

fn radial_angle(ix: usize, count: usize, rotation: f32) -> f32 {
    -FRAC_PI_2 + rotation + TAU * ix as f32 / count.max(1) as f32
}

fn radial_spoke_at(count: usize, center: (f32, f32), at_pos: (f32, f32), rotation: f32) -> usize {
    let (angle, _) = bearing(center, at_pos);
    let norm = (angle + FRAC_PI_2 - rotation).rem_euclid(TAU);
    ((norm / (TAU / count.max(1) as f32)).round() as usize) % count.max(1)
}

fn compute_radial(
    n: usize,
    series: &[&Series],
    range_band: Option<&(Vec<f64>, Vec<f64>)>,
    center: (f32, f32),
    outer: f32,
    rotation: f32,
) -> RadialLayout {
    let inner = (outer * 0.16).max(0.0);
    if n == 0 || outer <= 0.0 {
        return RadialLayout {
            inner,
            rings: Vec::new(),
            band: None,
            series_pts: Vec::new(),
        };
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for s in series {
        for &v in &s.values {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if let Some((low, high)) = range_band {
        for &v in low.iter().chain(high) {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        lo = 0.0;
        hi = 1.0;
    }
    let ((dom_min, dom_max), ticks) = nice(lo, hi, 4);
    let radius_of = |v: f64| -> f32 {
        let share = if dom_max > dom_min {
            ((v - dom_min) / (dom_max - dom_min)).clamp(0.0, 1.0) as f32
        } else {
            0.5
        };
        inner + (outer - inner) * share
    };
    let pt_at = |ix: usize, v: f64| -> (f32, f32) {
        let r = radius_of(v);
        let a = radial_angle(ix, n, rotation);
        (center.0 + r * a.cos(), center.1 + r * a.sin())
    };
    let rings = ticks.into_iter().map(|t| (radius_of(t), t)).collect();
    let band = range_band.map(|(low, high)| {
        let low_pts = (0..n).map(|ix| pt_at(ix, low[ix].min(high[ix]))).collect();
        let high_pts = (0..n).map(|ix| pt_at(ix, low[ix].max(high[ix]))).collect();
        (low_pts, high_pts)
    });
    let series_pts = series
        .iter()
        .map(|s| (0..n).map(|ix| pt_at(ix, s.values[ix])).collect())
        .collect();
    RadialLayout {
        inner,
        rings,
        band,
        series_pts,
    }
}

fn paint_closed_loop(path: &mut PathBuilder, pts: &[(f32, f32)], origin: Point<Pixels>) {
    if let Some(&first) = pts.first() {
        path.move_to(at(origin, first));
        for &pt in &pts[1..] {
            path.line_to(at(origin, pt));
        }
        path.close();
    }
}

impl Styled for RadialLineChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for RadialLineChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<RadialState> =
            window.use_keyed_state((self.id.clone(), "radial-line"), cx, |_, _| {
                RadialState::default()
            });
        let n = self.labels.len();
        let (bounds, state_hover, state_pinned, hidden, rotation, zoom) = {
            let st = state.read(cx);
            (
                st.bounds,
                st.hover,
                st.pinned,
                st.hidden.clone(),
                st.rotation,
                if st.zoom <= 0.0 { 1.0 } else { st.zoom },
            )
        };
        let hover = state_hover.filter(|ix| *ix < n);
        let pinned = state_pinned.filter(|ix| *ix < n);
        let active = pinned.or(hover);
        let shown: Vec<(usize, &Series)> = self
            .series
            .iter()
            .enumerate()
            .filter(|(_, s)| !hidden.contains(&s.name))
            .collect();
        let shown_series: Vec<&Series> = shown.iter().map(|(_, s)| *s).collect();
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let room = pixels(sizes.foot);
        let gap = pixels(sizes.inset);
        let center = (width * 0.5, height * 0.5);
        let base_outer = (width.min(height) * 0.5 - room).max(0.0);
        let outer = base_outer * zoom;
        let layout = compute_radial(
            n,
            &shown_series,
            self.range_band.as_ref(),
            center,
            outer,
            rotation,
        );
        let format = self.format.clone();
        let (fg_color, muted_color, subtle_color) = (colors.fg, colors.fg_muted, colors.fg_subtle);
        let text = |content: String, strong: bool| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if strong { fg_color } else { muted_color })
                .child(content)
        };

        let every = if n == 0 || outer <= 0.0 {
            1
        } else {
            ((n as f32 * pixels(sizes.label) / (TAU * outer).max(1.0)).ceil() as usize).max(1)
        };
        let spoke_labels: Vec<_> = (0..n)
            .filter(|ix| ix % every == 0)
            .map(|ix| {
                let a = radial_angle(ix, n, rotation);
                let (dx, dy) = (a.cos(), a.sin());
                let (lx, ly) = (center.0 + (outer + gap) * dx, center.1 + (outer + gap) * dy);
                let box_div = div()
                    .absolute()
                    .left(Pixels::from(lx))
                    .top(Pixels::from(ly))
                    .w_0()
                    .h_0()
                    .flex();
                let box_div = if dx > 0.3 {
                    box_div.justify_start()
                } else if dx < -0.3 {
                    box_div.justify_end()
                } else {
                    box_div.justify_center()
                };
                let box_div = if dy > 0.3 {
                    box_div.items_start()
                } else if dy < -0.3 {
                    box_div.items_end()
                } else {
                    box_div.items_center()
                };
                box_div.child(text(self.labels[ix].to_string(), active == Some(ix)))
            })
            .collect();

        let ring_labels: Vec<_> = layout
            .rings
            .iter()
            .skip(1)
            .map(|&(r, val)| {
                div()
                    .absolute()
                    .left(Pixels::from(center.0))
                    .top(Pixels::from(center.1 - r))
                    .h_0()
                    .flex()
                    .items_center()
                    .pl_1p5()
                    .child(tabular(text(format(val), false)).text_color(subtle_color))
            })
            .collect();

        let tooltip = active.map(|ix| {
            let mut card = ChartTooltip::new(self.labels[ix].clone());
            for &(orig_ix, s) in &shown {
                card = card.row(
                    Some(tint(&colors, orig_ix)),
                    s.name.clone(),
                    format(s.values[ix]),
                );
            }
            if let Some((ref low, ref high)) = self.range_band {
                card = card.row(
                    None,
                    "Range",
                    format!("{} – {}", format(low[ix]), format(high[ix])),
                );
            }
            let a = radial_angle(ix, n, rotation);
            let reach = outer.min(base_outer) * 0.78;
            let pos = (center.0 + reach * a.cos(), center.1 + reach * a.sin());
            anchored(pos, pos.0 > width * 0.55 || a.cos() > 0.0, card)
        });

        let legend = (self.series.len() > 1).then(|| {
            let toggled = state.clone();
            let legend = self.series.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "legend")),
                |legend, (ix, series)| {
                    legend.entry(
                        tint(&colors, ix),
                        series.name.clone(),
                        hidden.contains(&series.name),
                    )
                },
            );
            legend.on_toggle(move |name, _, cx| {
                toggled.update(cx, |st, cx| {
                    if !st.hidden.remove(name) {
                        st.hidden.insert(name.clone());
                    }
                    cx.notify();
                })
            })
        });
        let modified = rotation.abs() > 1e-4 || (zoom - 1.0).abs() > 1e-3 || pinned.is_some();
        let reset_btn = modified.then(|| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "radial-reset"), "Reset")
                .debug_selector(|| "radial-reset".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.rotation = 0.0;
                        st.zoom = 1.0;
                        st.pinned = None;
                        st.hover = None;
                        st.drag = None;
                        cx.notify();
                    })
                }))
        });

        let rings = layout.rings;
        let band = layout.band;
        let painted_series: Vec<(usize, PointRing)> = shown
            .iter()
            .zip(layout.series_pts)
            .map(|((orig_ix, _), pts)| (*orig_ix, pts))
            .collect();
        let inner = layout.inner;
        let filled = self.filled;
        let (hairline, stroke) = (sizes.hairline, sizes.stroke.to_pixels(rem));
        let hit_outer = if touch_mode && outer > 0.0 {
            (outer + room * 0.5).max(24.0)
        } else {
            outer + room * 0.5
        };
        let pick_spoke = move |pos: (f32, f32)| -> Option<usize> {
            if n == 0 || outer <= 0.0 {
                return None;
            }
            let (_, dist) = bearing(center, pos);
            (dist <= hit_outer).then(|| radial_spoke_at(n, center, pos, rotation))
        };
        let (moved, pressed, released, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let released_end = released.clone();
        let end_drag = move |pos: Option<(f32, f32)>, cx: &mut App| {
            released_end.update(cx, |st, cx| {
                let Some((_, down_pos, dragged)) = st.drag.take() else {
                    return;
                };
                if !dragged
                    && let Some(place) = pos
                    && n > 0
                    && outer > 0.0
                {
                    let (_, dist) = bearing(center, place);
                    if dist <= hit_outer {
                        let ix = radial_spoke_at(n, center, down_pos, st.rotation);
                        st.pinned = if st.pinned == Some(ix) {
                            None
                        } else {
                            Some(ix)
                        };
                    }
                }
                cx.notify();
            })
        };
        let (up_in, up_out) = (end_drag.clone(), end_drag);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let plot = div()
            .id((self.id.clone(), "radial-line"))
            .debug_selector(|| "radial-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .cursor_grab()
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some((prev_angle, down_pos, dragged)) = st.drag.as_mut()
                    {
                        let dist = (pos.0 - down_pos.0).hypot(pos.1 - down_pos.1);
                        if *dragged || dist > 4.0 {
                            *dragged = true;
                            let (angle, _) = bearing(center, pos);
                            let mut da = angle - *prev_angle;
                            if da > std::f32::consts::PI {
                                da -= TAU;
                            }
                            if da < -std::f32::consts::PI {
                                da += TAU;
                            }
                            st.rotation = (st.rotation + da).rem_euclid(TAU);
                            *prev_angle = angle;
                            cx.notify();
                        }
                        return;
                    }
                    let next = pick_spoke(pos);
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
                        st.rotation = 0.0;
                        st.zoom = 1.0;
                        st.pinned = None;
                        st.drag = None;
                        cx.notify();
                        return;
                    }
                    st.hover = pick_spoke(pos);
                    if n > 0 && outer > 0.0 {
                        let (angle, dist) = bearing(center, pos);
                        if dist <= hit_outer {
                            st.drag = Some((angle, pos, false));
                        }
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let bounds = released.read(cx).bounds;
                up_in(Some(local(event.position, bounds)), cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                up_out(None, cx);
            })
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
                if ev.source == ScrollSource::Touch {
                    if dx.abs() > dy.abs() && dx.abs() >= 0.2 {
                        scrolled.update(cx, |st, cx| {
                            st.rotation = (st.rotation + dx * 0.008).rem_euclid(TAU);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                    return;
                }
                if dy.abs() >= 0.05 && base_outer > 0.0 {
                    scrolled.update(cx, |st, cx| {
                        let z = if st.zoom <= 0.0 { 1.0 } else { st.zoom };
                        st.zoom = (z * (1.0 + dy * 0.002)).clamp(0.5, 3.0);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                if !ev.delta.is_finite() || ev.phase == TouchPhase::Cancelled {
                    pinched.update(cx, |st, _| st.drag = None);
                    return;
                }
                let scale = ev.delta.exp().clamp(0.25, 4.0);
                pinched.update(cx, |st, cx| {
                    st.drag = None;
                    if (scale - 1.0).abs() > 1e-4 && base_outer > 0.0 {
                        let z = if st.zoom <= 0.0 { 1.0 } else { st.zoom };
                        st.zoom = (z * scale).clamp(0.5, 3.0);
                        cx.notify();
                    }
                });
                if (scale - 1.0).abs() > 1e-4 && base_outer > 0.0 {
                    cx.stop_propagation();
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let center_pt = bounds.center();
                        let segs = 56;
                        for &(ring_r, _) in &rings {
                            if ring_r <= 0.5 {
                                continue;
                            }
                            let mut ring_path = PathBuilder::stroke(hairline);
                            ring_path.move_to(toward(center_pt, ring_r, 0.0));
                            for s in 1..=segs {
                                ring_path.line_to(toward(
                                    center_pt,
                                    ring_r,
                                    TAU * s as f32 / segs as f32,
                                ));
                            }
                            ring_path.close();
                            finish(ring_path, colors.border.opacity(0.55), window);
                        }
                        for ix in 0..n {
                            let is_active = active == Some(ix);
                            if ix % every != 0 && !is_active && n > 24 {
                                continue;
                            }
                            let a = radial_angle(ix, n, rotation);
                            let mut spoke = PathBuilder::stroke(if pinned == Some(ix) {
                                stroke
                            } else {
                                hairline
                            });
                            spoke.move_to(toward(center_pt, inner, a));
                            spoke.line_to(toward(center_pt, outer, a));
                            finish(
                                spoke,
                                if pinned == Some(ix) {
                                    colors.fg
                                } else if is_active {
                                    colors.border_strong
                                } else {
                                    colors.border.opacity(0.45)
                                },
                                window,
                            );
                        }
                        if let Some((ref low_pts, ref high_pts)) = band
                            && low_pts.len() >= 2
                        {
                            let mut area = PathBuilder::fill();
                            area.move_to(at(origin, high_pts[0]));
                            for &pt in &high_pts[1..] {
                                area.line_to(at(origin, pt));
                            }
                            area.line_to(at(origin, high_pts[0]));
                            area.line_to(at(origin, low_pts[0]));
                            for &pt in low_pts.iter().rev() {
                                area.line_to(at(origin, pt));
                            }
                            area.close();
                            finish(area, tint(&colors, 0).opacity(0.16), window);
                        }
                        for (orig_ix, pts) in &painted_series {
                            let ink = tint(&colors, *orig_ix);
                            if pts.len() >= 2 {
                                if filled {
                                    let mut area = PathBuilder::fill();
                                    if inner > 0.5 && pts.len() >= 3 {
                                        area.move_to(at(origin, pts[0]));
                                        for &pt in &pts[1..] {
                                            area.line_to(at(origin, pt));
                                        }
                                        area.line_to(at(origin, pts[0]));
                                        let first_inner =
                                            toward(center_pt, inner, radial_angle(0, n, rotation));
                                        area.line_to(first_inner);
                                        for ix in (0..n).rev() {
                                            area.line_to(toward(
                                                center_pt,
                                                inner,
                                                radial_angle(ix, n, rotation),
                                            ));
                                        }
                                        area.close();
                                    } else {
                                        paint_closed_loop(&mut area, pts, origin);
                                    }
                                    finish(area, ink.opacity(0.18), window);
                                }
                                let mut edge = PathBuilder::stroke(stroke);
                                paint_closed_loop(&mut edge, pts, origin);
                                finish(edge, ink, window);
                            } else if let Some(&pt) = pts.first() {
                                ring(
                                    at(origin, pt),
                                    stroke * 2.0,
                                    (colors.bg, ink),
                                    stroke,
                                    window,
                                );
                            }
                        }
                        if let Some(ix) = active {
                            for (orig_ix, pts) in &painted_series {
                                if let Some(&pt) = pts.get(ix) {
                                    ring(
                                        at(origin, pt),
                                        stroke * 2.0,
                                        (colors.bg, tint(&colors, *orig_ix)),
                                        stroke,
                                        window,
                                    );
                                }
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(ring_labels)
            .children(spoke_labels)
            .children(tooltip)
            .children(reset_btn)
            .child(measure(state, |st| &mut st.bounds));
        root.flex().flex_col().gap_2().children(legend).child(plot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizon_splits_bands_and_crossings_cleanly() {
        let row = Rect {
            x: 0.0,
            y: 10.0,
            w: 100.0,
            h: 40.0,
        };
        let patches = build_band_patches(&[0.0, 100.0], &[-10.0, 10.0], row, 2, 5.0, true);
        assert!(
            patches.iter().any(|p| p.positive && p.band == 0)
                && patches.iter().any(|p| !p.positive && p.band == 0),
            "crossing zero produces both positive and negative band patches"
        );
        for patch in &patches {
            for &(x, y) in &patch.points {
                assert!(x >= 0.0 && x <= 100.0, "x stays in row: {x}");
                assert!(
                    y >= row.y - 1e-3 && y <= row.y + row.h + 1e-3,
                    "y stays in row: {y}"
                );
            }
        }

        let offset_patches = build_band_patches(&[0.0, 100.0], &[-5.0, -5.0], row, 2, 5.0, false);
        assert_eq!(offset_patches.len(), 1);
        assert!(!offset_patches[0].positive);
        assert!(
            offset_patches[0]
                .points
                .iter()
                .any(|&(_, y)| (y - row.y).abs() < 1e-3),
            "offset mode anchors negative band to row top"
        );
    }

    #[test]
    fn radial_layout_maps_rings_and_envelope_within_outer_radius() {
        let series = vec![Series::new("Temp", [10.0, 20.0, 15.0, 12.0])];
        let refs: Vec<&Series> = series.iter().collect();
        let band = (vec![5.0, 15.0, 10.0, 8.0], vec![14.0, 25.0, 19.0, 16.0]);
        let layout = compute_radial(4, &refs, Some(&band), (120.0, 120.0), 80.0, 0.0);
        assert_eq!(layout.series_pts[0].len(), 4);
        let (low_pts, high_pts) = layout.band.expect("range band");
        for (lp, hp) in low_pts.iter().zip(&high_pts) {
            let dl = (lp.0 - 120.0).hypot(lp.1 - 120.0);
            let dh = (hp.0 - 120.0).hypot(hp.1 - 120.0);
            assert!(dl <= dh + 1e-3 && dh <= 80.0 + 1e-3);
        }
        assert_eq!(radial_spoke_at(4, (100.0, 100.0), (100.0, 40.0), 0.0), 0);
        assert_eq!(radial_spoke_at(4, (100.0, 100.0), (160.0, 100.0), 0.0), 1);
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn horizon_and_radial_render_export_and_open_tooltips(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, Window, px};

        struct HorizonHarness;
        impl Render for HorizonHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).h(px(280.0)).child(
                    HorizonChart::new("hz", ["T1", "T2", "T3", "T4"])
                        .series(Series::new("Cluster A", [12.0, -8.0, 25.0, -18.0]))
                        .series(Series::new("Cluster B", [4.0, 9.0, -6.0, 14.0]))
                        .bands(4)
                        .baseline(0.0)
                        .mode_mirror(true),
                )
            }
        }

        struct RadialHarness;
        impl Render for RadialHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).h(px(280.0)).child(
                    RadialLineChart::new("rad", ["Jan", "Apr", "Jul", "Oct"])
                        .series(Series::new("Mean", [4.0, 14.0, 24.0, 13.0]))
                        .range_band([0.0, 8.0, 18.0, 7.0], [8.0, 20.0, 30.0, 19.0])
                        .filled(true),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            let horizon = HorizonChart::new("hz-exp", ["A", "B", "C"])
                .series(Series::new("Load", [1.0, -2.0, 3.5]))
                .bands(3);
            assert_eq!(horizon.csv(), ",A,B,C\nLoad,1,-2,3.5");
            let h_svg = horizon.svg(320.0, 200.0, cx);
            assert!(h_svg.starts_with("<svg") && h_svg.contains("<path"));

            let radial = RadialLineChart::new("rad-exp", ["N", "E", "S", "W"])
                .series(Series::new("Temp", [10.0, 20.0, 15.0, 5.0]))
                .range_band([5.0, 12.0, 9.0, 2.0], [14.0, 25.0, 21.0, 9.0])
                .filled(true);
            assert!(radial.csv().contains("Temp,10,20,15,5"));
            assert!(radial.csv().contains("High,14,25,21,9"));
            let r_svg = radial.svg(280.0, 280.0, cx);
            assert!(
                r_svg.starts_with("<svg") && r_svg.contains("<circle") && r_svg.contains("<path")
            );
        });

        let (_, vc_h) = cx.add_window_view(|_, _| HorizonHarness);
        vc_h.simulate_resize(gpui::size(px(360.0), px(280.0)));
        settle(vc_h);
        let h_root = vc_h.debug_bounds("chart-root").expect("horizon root");
        let row_label = vc_h
            .debug_bounds("horizon-row-label")
            .expect("horizon row label");
        assert!(
            row_label.left() >= h_root.left(),
            "label clipped on left: {row_label:?} vs {h_root:?}"
        );
        vc_h.simulate_mouse_move(h_root.center(), None, Modifiers::none());
        settle(vc_h);
        assert!(vc_h.debug_bounds("chart-tooltip").is_some());

        let (_, vc_r) = cx.add_window_view(|_, _| RadialHarness);
        vc_r.simulate_resize(gpui::size(px(360.0), px(280.0)));
        settle(vc_r);
        let r_root = vc_r.debug_bounds("chart-root").expect("radial root");
        vc_r.simulate_mouse_move(r_root.center(), None, Modifiers::none());
        settle(vc_r);
        assert!(vc_r.debug_bounds("chart-tooltip").is_some());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn horizon_chart_supports_legend_brush_scroll_zoom_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

        struct HorizonInteractiveHarness;
        impl Render for HorizonInteractiveHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(400.0)).h(px(280.0)).child(
                    HorizonChart::new("hz-int", ["T1", "T2", "T3", "T4", "T5", "T6"])
                        .series(Series::new("CPU", [12.0, -8.0, 25.0, -18.0, 10.0, 4.0]))
                        .series(Series::new("Mem", [4.0, 9.0, -6.0, 14.0, -2.0, 8.0])),
                )
            }
        }

        cx.update(Theme::init);
        let (_, vc) = cx.add_window_view(|_, _| HorizonInteractiveHarness);
        vc.simulate_resize(gpui::size(px(400.0), px(280.0)));
        settle(vc);

        let legend_entry = vc
            .debug_bounds("chart-legend-entry-0")
            .expect("multi-series horizon legend renders");
        vc.simulate_mouse_down(legend_entry.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(legend_entry.center(), MouseButton::Left, Modifiers::none());
        settle(vc);

        let plot = vc
            .debug_bounds("horizon-plot")
            .expect("horizon plot bounds");
        assert!(vc.debug_bounds("chart-show-all").is_none());

        let y = plot.center().y;
        let from = point(plot.left() + plot.size.width * 0.25, y);
        let to = point(plot.left() + plot.size.width * 0.75, y);
        vc.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
        vc.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-show-all").is_some(),
            "horizontal drag brush zooms span and shows reset button"
        );

        let show_all = vc.debug_bounds("chart-show-all").expect("show-all button");
        vc.simulate_mouse_down(show_all.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(show_all.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-show-all").is_none(),
            "Show all button resets zoomed span"
        );

        vc.simulate_event(ScrollEvent {
            position: plot.center(),
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
            position: plot.center(),
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_mouse_up(plot.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-show-all").is_none(),
            "double-click resets zoomed span"
        );
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn radial_line_chart_supports_legend_pin_rotation_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, Window, point, px};

        struct RadialInteractiveHarness;
        impl Render for RadialInteractiveHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(360.0)).h(px(300.0)).child(
                    RadialLineChart::new("rad-int", ["Jan", "Apr", "Jul", "Oct"])
                        .series(Series::new("Mean", [4.0, 14.0, 24.0, 13.0]))
                        .series(Series::new("Peak", [8.0, 18.0, 28.0, 17.0])),
                )
            }
        }

        cx.update(Theme::init);
        let (_, vc) = cx.add_window_view(|_, _| RadialInteractiveHarness);
        vc.simulate_resize(gpui::size(px(360.0), px(300.0)));
        settle(vc);

        let legend_entry = vc
            .debug_bounds("chart-legend-entry-1")
            .expect("multi-series radial legend renders");
        vc.simulate_mouse_down(legend_entry.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(legend_entry.center(), MouseButton::Left, Modifiers::none());
        settle(vc);

        let plot = vc.debug_bounds("radial-plot").expect("radial plot bounds");
        let top_spoke = point(plot.center().x, plot.center().y - px(45.0));
        vc.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(top_spoke, MouseButton::Left, Modifiers::none());
        settle(vc);

        // Moving pointer outside keeps pinned tooltip visible.
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        let tip_before = vc
            .debug_bounds("chart-tooltip")
            .expect("pinned spoke keeps tooltip visible");

        // Drag from 12 o'clock to 3 o'clock to rotate dial clockwise ~90 deg.
        let right_spoke = point(plot.center().x + px(45.0), plot.center().y);
        vc.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(right_spoke, Some(MouseButton::Left), Modifiers::none());
        vc.simulate_mouse_up(right_spoke, MouseButton::Left, Modifiers::none());
        settle(vc);
        let tip_after = vc
            .debug_bounds("chart-tooltip")
            .expect("rotated pinned spoke keeps tooltip visible");
        assert!(
            tip_after.top() > tip_before.top() + px(20.0) && tip_after.origin != tip_before.origin,
            "rotating clockwise shifts pinned spoke tooltip down around the dial"
        );

        // Double-click resets rotation and pinned state.
        vc.simulate_event(MouseDownEvent {
            position: plot.center(),
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_mouse_up(plot.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_none(),
            "double-click clears pinned state"
        );
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn horizon_and_radial_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};

        struct NarrowHorizon;
        impl Render for NarrowHorizon {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(240.0)).h(px(240.0)).child(
                    HorizonChart::new("hz-touch", ["T1", "T2", "T3", "T4", "T5", "T6"])
                        .series(Series::new("CPU", [12.0, -8.0, 25.0, -18.0, 10.0, 4.0]))
                        .series(Series::new("Mem", [4.0, 9.0, -6.0, 14.0, -2.0, 8.0])),
                )
            }
        }

        struct NarrowRadial;
        impl Render for NarrowRadial {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(240.0)).h(px(240.0)).child(
                    RadialLineChart::new("rad-touch", ["Jan", "Apr", "Jul", "Oct"])
                        .series(Series::new("Mean", [4.0, 14.0, 24.0, 13.0]))
                        .series(Series::new("Peak", [8.0, 18.0, 28.0, 17.0])),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });

        // HorizonChart touch_mode: tap pin, right-side tooltip flip, pinch clears brush, touch swipe pan, reset.
        let (_, vc_h) = cx.add_window_view(|_, _| NarrowHorizon);
        vc_h.simulate_resize(gpui::size(px(240.0), px(240.0)));
        settle(vc_h);
        let h_plot = vc_h.debug_bounds("horizon-plot").expect("horizon plot");
        let right_tap = point(h_plot.left() + h_plot.size.width * 0.78, h_plot.center().y);
        vc_h.simulate_mouse_down(right_tap, MouseButton::Left, Modifiers::none());
        vc_h.simulate_mouse_up(right_tap, MouseButton::Left, Modifiers::none());
        settle(vc_h);
        let tip = vc_h
            .debug_bounds("chart-tooltip")
            .expect("tap pins tooltip");
        assert!(
            tip.right() <= h_plot.right() + px(1.0),
            "right-side tooltip flips inside narrow 240px card"
        );
        let h_reset = vc_h
            .debug_bounds("chart-show-all")
            .expect("pinned shows reset button");
        assert!(h_reset.size.width >= px(48.0) && h_reset.size.height >= px(48.0));
        vc_h.simulate_click(h_reset.center(), Modifiers::none());
        settle(vc_h);
        assert!(vc_h.debug_bounds("chart-show-all").is_none());

        // Start brush, pinch clears brush so trailing move does not brush-zoom.
        let h_from = point(h_plot.left() + h_plot.size.width * 0.3, h_plot.center().y);
        let h_to = point(h_plot.left() + h_plot.size.width * 0.9, h_plot.center().y);
        vc_h.simulate_mouse_down(h_from, MouseButton::Left, Modifiers::none());
        vc_h.simulate_event(PinchEvent {
            position: h_plot.center(),
            delta: 0.6,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc_h);
        vc_h.simulate_mouse_move(h_to, Some(MouseButton::Left), Modifiers::none());
        vc_h.simulate_mouse_up(h_to, MouseButton::Left, Modifiers::none());
        settle(vc_h);
        vc_h.simulate_event(ScrollEvent {
            position: h_plot.center(),
            delta: ScrollDelta::Pixels(point(px(28.0), px(0.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc_h);
        let h_show_all = vc_h
            .debug_bounds("chart-show-all")
            .expect("show-all after pinch/pan");
        vc_h.simulate_click(h_show_all.center(), Modifiers::none());
        settle(vc_h);
        assert!(vc_h.debug_bounds("chart-show-all").is_none());

        // RadialLineChart touch_mode: tap pin, pinch clears drag, touch swipe rotates, Reset button resets.
        let (_, vc_r) = cx.add_window_view(|_, _| NarrowRadial);
        vc_r.simulate_resize(gpui::size(px(240.0), px(240.0)));
        settle(vc_r);
        let r_plot = vc_r.debug_bounds("radial-plot").expect("radial plot");
        let top_spoke = point(r_plot.center().x, r_plot.center().y - px(32.0));
        let right_spoke = point(r_plot.center().x + px(32.0), r_plot.center().y);
        vc_r.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
        vc_r.simulate_mouse_up(top_spoke, MouseButton::Left, Modifiers::none());
        settle(vc_r);
        assert!(vc_r.debug_bounds("chart-tooltip").is_some());
        let r_reset = vc_r
            .debug_bounds("radial-reset")
            .expect("pinned spoke shows Reset button");
        assert!(r_reset.size.width >= px(48.0) && r_reset.size.height >= px(48.0));
        vc_r.simulate_click(r_reset.center(), Modifiers::none());
        settle(vc_r);
        assert!(vc_r.debug_bounds("radial-reset").is_none());

        // Press down, pinch clears drag so move does not rotate, then touch scroll rotates and Reset clears.
        vc_r.simulate_mouse_down(top_spoke, MouseButton::Left, Modifiers::none());
        vc_r.simulate_event(PinchEvent {
            position: r_plot.center(),
            delta: 0.4,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc_r);
        vc_r.simulate_mouse_move(right_spoke, Some(MouseButton::Left), Modifiers::none());
        vc_r.simulate_mouse_up(right_spoke, MouseButton::Left, Modifiers::none());
        settle(vc_r);
        assert!(
            vc_r.debug_bounds("radial-reset").is_some(),
            "pinch zoom shows radial-reset"
        );
        vc_r.simulate_event(ScrollEvent {
            position: r_plot.center(),
            delta: ScrollDelta::Pixels(point(px(36.0), px(0.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc_r);
        let r_reset = vc_r
            .debug_bounds("radial-reset")
            .expect("reset button visible");
        vc_r.simulate_click(r_reset.center(), Modifiers::none());
        settle(vc_r);
        assert!(vc_r.debug_bounds("radial-reset").is_none());
    }
}
