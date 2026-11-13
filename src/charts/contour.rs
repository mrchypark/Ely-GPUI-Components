use std::{fmt::Write as _, rc::Rc};

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
    typography::tabular,
};

type Segment = ((f32, f32), (f32, f32));
type Domain = ((f64, f64), (f64, f64));

#[derive(Default)]
struct ContourState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    domain: Option<Domain>,
    drag_last: Option<(f32, f32)>,
    dragged: bool,
}

enum Source {
    Points(Vec<(f64, f64)>),
    Grid {
        cols: usize,
        rows: usize,
        values: Vec<f64>,
    },
}

/// 2D density and scalar-field contour bands and isolines extracted via Marching Squares.
#[derive(IntoElement)]
pub struct ContourChart {
    base: Div,
    id: ElementId,
    source: Source,
    grid_size: (usize, usize),
    bandwidth: Option<f64>,
    levels: usize,
    thresholds: Option<Vec<f64>>,
    axes: (SharedString, SharedString),
    format: Format,
}

impl ContourChart {
    /// Estimate 2D kernel density contours from scattered `(x, y)` points.
    pub fn from_points(
        id: impl Into<ElementId>,
        points: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        let pts: Vec<(f64, f64)> = points.into_iter().collect();
        assert!(
            pts.iter().all(|(x, y)| drawable(*x) && drawable(*y)),
            "contour points need finite values, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            source: Source::Points(pts),
            grid_size: (36, 36),
            bandwidth: None,
            levels: 6,
            thresholds: None,
            axes: ("X".into(), "Y".into()),
            format: Rc::new(compact),
        }
    }

    /// Build contours from an explicit row-major scalar field (`cols >= 2`, `rows >= 2`).
    pub fn from_grid(
        id: impl Into<ElementId>,
        cols: usize,
        rows: usize,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        assert!(
            cols >= 2 && rows >= 2,
            "a contour grid needs at least 2x2 samples"
        );
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            cols * rows,
            "contour grid length must equal cols * rows"
        );
        assert!(
            values.iter().all(|v| drawable(*v)),
            "contour grid values must be finite, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            source: Source::Grid { cols, rows, values },
            grid_size: (cols, rows),
            bandwidth: None,
            levels: 6,
            thresholds: None,
            axes: ("X".into(), "Y".into()),
            format: Rc::new(compact),
        }
    }

    /// Override the KDE grid resolution (clamped to `8..=96` per axis).
    pub fn grid_size(mut self, cols: usize, rows: usize) -> Self {
        self.grid_size = (cols.clamp(8, 96), rows.clamp(8, 96));
        self
    }

    /// Override the Gaussian KDE bandwidth in data units.
    pub fn bandwidth(mut self, bandwidth: f64) -> Self {
        assert!(
            bandwidth.is_finite() && bandwidth > 0.0,
            "contour bandwidth must be finite and positive"
        );
        self.bandwidth = Some(bandwidth);
        self
    }

    /// Set the number of automatically spaced contour levels (at least 1).
    pub fn levels(mut self, levels: usize) -> Self {
        self.levels = levels.max(1);
        self.thresholds = None;
        self
    }

    /// Set explicit sorted threshold values.
    pub fn thresholds(mut self, thresholds: impl IntoIterator<Item = f64>) -> Self {
        let mut vals: Vec<f64> = thresholds.into_iter().filter(|v| v.is_finite()).collect();
        vals.sort_by(f64::total_cmp);
        vals.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        self.thresholds = Some(vals);
        self
    }

    pub fn axes(mut self, x: impl Into<SharedString>, y: impl Into<SharedString>) -> Self {
        self.axes = (x.into(), y.into());
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export the scalar field grid as CSV (`col,row,x,y,value`).
    pub fn csv(&self) -> String {
        let field = self.build_field(None);
        let mut out = String::from("col,row,x,y,value");
        if field.cols == 0 || field.rows == 0 {
            return out;
        }
        for r in 0..field.rows {
            let ty = r as f64 / (field.rows - 1).max(1) as f64;
            let y = field.y_domain.0 + (field.y_domain.1 - field.y_domain.0) * ty;
            for c in 0..field.cols {
                let tx = c as f64 / (field.cols - 1).max(1) as f64;
                let x = field.x_domain.0 + (field.x_domain.1 - field.x_domain.0) * tx;
                let v = field.values[r * field.cols + c];
                write!(out, "\n{c},{r},{x},{y},{v}").expect("csv write");
            }
        }
        out
    }

    /// Export contour bands and isolines as standalone SVG.
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
        let field = self.build_field(None);
        let x_scale = Linear::new(field.x_domain, (rect.x, rect.x + rect.w));
        let y_scale = Linear::new(field.y_domain, (rect.y + rect.h, rect.y));
        let bands = build_bands(
            &field,
            x_scale,
            y_scale,
            self.levels,
            self.thresholds.as_deref(),
        );
        let ink = tint(colors, 0);
        let (bg_hex, _) = svg_hex(colors.bg);
        let (ink_hex, _) = svg_hex(ink);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        let total_levels = bands.len().max(1);
        for (lvl_ix, band) in bands.iter().enumerate() {
            let alpha = 0.14 + 0.68 * ((lvl_ix + 1) as f32 / total_levels as f32);
            for poly in &band.polygons {
                if poly.len() < 3 {
                    continue;
                }
                let mut d = String::new();
                for (idx, &(px, py)) in poly.iter().enumerate() {
                    let cmd = if idx == 0 { 'M' } else { 'L' };
                    write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
                }
                d.push('Z');
                write!(
                    out,
                    r#"<path d="{d}" fill="{ink_hex}" fill-opacity="{alpha:.2}"/>"#
                )
                .expect("svg write");
            }
            for &(a, b) in &band.isolines {
                write!(
                    out,
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{ink_hex}" stroke-opacity="0.7" stroke-width="1"/>"#,
                    a.0, a.1, b.0, b.1
                )
                .expect("svg write");
            }
        }
        out.push_str("</svg>");
        out
    }

    fn base_domain(&self) -> Domain {
        match &self.source {
            Source::Grid { cols, rows, .. } => {
                ((0.0, (*cols - 1) as f64), (0.0, (*rows - 1) as f64))
            }
            Source::Points(points) => points_domain(points),
        }
    }

    fn build_field(&self, domain: Option<Domain>) -> ScalarField {
        match &self.source {
            Source::Grid { cols, rows, values } => ScalarField {
                cols: *cols,
                rows: *rows,
                x_domain: (0.0, (*cols - 1) as f64),
                y_domain: (0.0, (*rows - 1) as f64),
                values: values.clone(),
            },
            Source::Points(points) => kde_field(points, self.grid_size, self.bandwidth, domain),
        }
    }
}

struct ScalarField {
    cols: usize,
    rows: usize,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    values: Vec<f64>,
}

struct ContourLevel {
    threshold: f64,
    polygons: Vec<Vec<(f32, f32)>>,
    isolines: Vec<Segment>,
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

fn points_domain(points: &[(f64, f64)]) -> Domain {
    if points.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }
    let (mut x_min, mut x_max) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y_min, mut y_max) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(x, y) in points {
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }
    let ((x0, x1), _) = nice(x_min, x_max, 5);
    let ((y0, y1), _) = nice(y_min, y_max, 5);
    ((x0, x1), (y0, y1))
}

fn kde_field(
    points: &[(f64, f64)],
    (cols, rows): (usize, usize),
    bandwidth_override: Option<f64>,
    domain: Option<Domain>,
) -> ScalarField {
    if points.is_empty() || cols < 2 || rows < 2 {
        return ScalarField {
            cols: 0,
            rows: 0,
            x_domain: (0.0, 1.0),
            y_domain: (0.0, 1.0),
            values: Vec::new(),
        };
    }
    let base = points_domain(points);
    let ((x0, x1), (y0, y1)) = domain.unwrap_or(base);
    let n = points.len() as f64;
    let mean_x = points.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = points.iter().map(|p| p.1).sum::<f64>() / n;
    let std_x = (points.iter().map(|p| (p.0 - mean_x).powi(2)).sum::<f64>() / n.max(1.0)).sqrt();
    let std_y = (points.iter().map(|p| (p.1 - mean_y).powi(2)).sum::<f64>() / n.max(1.0)).sqrt();
    let silverman = |std: f64, span: f64| -> f64 {
        let s = if std > 1e-12 { std } else { span * 0.12 };
        (0.9 * s * n.powf(-1.0 / 6.0)).max(span * 0.04).max(1e-6)
    };
    let hx = bandwidth_override.unwrap_or_else(|| silverman(std_x, (base.0.1 - base.0.0).abs()));
    let hy = bandwidth_override.unwrap_or_else(|| silverman(std_y, (base.1.1 - base.1.0).abs()));
    let inv_hx2 = 0.5 / (hx * hx);
    let inv_hy2 = 0.5 / (hy * hy);
    let norm = 1.0 / (n * 2.0 * std::f64::consts::PI * hx * hy);

    let mut values = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let gy = y0 + (y1 - y0) * (r as f64 / (rows - 1) as f64);
        for c in 0..cols {
            let gx = x0 + (x1 - x0) * (c as f64 / (cols - 1) as f64);
            let mut sum = 0.0;
            for &(px, py) in points {
                let dx = gx - px;
                let dy = gy - py;
                let arg = -(dx * dx * inv_hx2 + dy * dy * inv_hy2);
                if arg > -40.0 {
                    sum += arg.exp();
                }
            }
            values.push(sum * norm);
        }
    }
    ScalarField {
        cols,
        rows,
        x_domain: (x0, x1),
        y_domain: (y0, y1),
        values,
    }
}

fn resolve_thresholds(field: &ScalarField, levels: usize, explicit: Option<&[f64]>) -> Vec<f64> {
    if let Some(vals) = explicit {
        return vals.to_vec();
    }
    if field.values.is_empty() {
        return Vec::new();
    }
    let (lo, hi) = field
        .values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    if hi <= lo || hi.is_nan() || lo.is_nan() {
        return Vec::new();
    }
    (1..=levels)
        .map(|k| lo + (hi - lo) * (k as f64 / (levels + 1) as f64))
        .collect()
}

/// Marching Squares isolines + clipped `v >= threshold` band polygons per grid cell.
fn build_bands(
    field: &ScalarField,
    x_scale: Linear,
    y_scale: Linear,
    levels: usize,
    explicit: Option<&[f64]>,
) -> Vec<ContourLevel> {
    if field.cols < 2
        || field.rows < 2
        || (x_scale.range.1 - x_scale.range.0).abs() <= 0.0
        || (y_scale.range.0 - y_scale.range.1).abs() <= 0.0
    {
        return Vec::new();
    }
    let thresholds = resolve_thresholds(field, levels, explicit);
    let xs: Vec<f32> = (0..field.cols)
        .map(|c| {
            let dx = field.x_domain.0
                + (field.x_domain.1 - field.x_domain.0) * (c as f64 / (field.cols - 1) as f64);
            x_scale.at(dx)
        })
        .collect();
    let ys: Vec<f32> = (0..field.rows)
        .map(|r| {
            let dy = field.y_domain.0
                + (field.y_domain.1 - field.y_domain.0) * (r as f64 / (field.rows - 1) as f64);
            y_scale.at(dy)
        })
        .collect();
    thresholds
        .into_iter()
        .map(|threshold| {
            let mut polygons = Vec::new();
            let mut isolines = Vec::new();
            for r in 0..field.rows - 1 {
                let y_bot = ys[r];
                let y_top = ys[r + 1];
                for c in 0..field.cols - 1 {
                    let x_left = xs[c];
                    let x_right = xs[c + 1];
                    let v_bl = field.values[r * field.cols + c];
                    let v_br = field.values[r * field.cols + c + 1];
                    let v_tr = field.values[(r + 1) * field.cols + c + 1];
                    let v_tl = field.values[(r + 1) * field.cols + c];
                    let corners = [
                        (x_left, y_bot, v_bl),
                        (x_right, y_bot, v_br),
                        (x_right, y_top, v_tr),
                        (x_left, y_top, v_tl),
                    ];
                    let poly = clip_cell_above(&corners, threshold);
                    if poly.len() >= 3 {
                        polygons.push(poly);
                    }
                    marching_cell_segments(&corners, threshold, &mut isolines);
                }
            }
            ContourLevel {
                threshold,
                polygons,
                isolines,
            }
        })
        .collect()
}

fn interp(a: (f32, f32, f64), b: (f32, f32, f64), threshold: f64) -> (f32, f32) {
    let denom = b.2 - a.2;
    let t = if denom.abs() > 1e-15 {
        ((threshold - a.2) / denom).clamp(0.0, 1.0) as f32
    } else {
        0.5
    };
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn clip_cell_above(corners: &[(f32, f32, f64); 4], threshold: f64) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(6);
    let mut prev = corners[3];
    let mut prev_in = prev.2 >= threshold;
    for &curr in corners {
        let curr_in = curr.2 >= threshold;
        if curr_in != prev_in {
            out.push(interp(prev, curr, threshold));
        }
        if curr_in {
            out.push((curr.0, curr.1));
        }
        prev = curr;
        prev_in = curr_in;
    }
    out
}

fn marching_cell_segments(corners: &[(f32, f32, f64); 4], threshold: f64, out: &mut Vec<Segment>) {
    // corners: 0=BL, 1=BR, 2=TR, 3=TL
    let b0 = u8::from(corners[0].2 >= threshold);
    let b1 = u8::from(corners[1].2 >= threshold) << 1;
    let b2 = u8::from(corners[2].2 >= threshold) << 2;
    let b3 = u8::from(corners[3].2 >= threshold) << 3;
    let mask = b0 | b1 | b2 | b3;
    if mask == 0 || mask == 15 {
        return;
    }
    let e_bot = || interp(corners[0], corners[1], threshold);
    let e_right = || interp(corners[1], corners[2], threshold);
    let e_top = || interp(corners[2], corners[3], threshold);
    let e_left = || interp(corners[3], corners[0], threshold);
    let center_in = (corners[0].2 + corners[1].2 + corners[2].2 + corners[3].2) * 0.25 >= threshold;
    match mask {
        1 | 14 => out.push((e_left(), e_bot())),
        2 | 13 => out.push((e_bot(), e_right())),
        3 | 12 => out.push((e_left(), e_right())),
        4 | 11 => out.push((e_right(), e_top())),
        6 | 9 => out.push((e_bot(), e_top())),
        7 | 8 => out.push((e_top(), e_left())),
        5 => {
            if center_in {
                out.push((e_left(), e_top()));
                out.push((e_bot(), e_right()));
            } else {
                out.push((e_left(), e_bot()));
                out.push((e_right(), e_top()));
            }
        }
        10 => {
            if center_in {
                out.push((e_left(), e_bot()));
                out.push((e_top(), e_right()));
            } else {
                out.push((e_bot(), e_right()));
                out.push((e_left(), e_top()));
            }
        }
        _ => {}
    }
}

fn sample_field_at(field: &ScalarField, x: f64, y: f64) -> Option<f64> {
    if field.cols < 2 || field.rows < 2 {
        return None;
    }
    let x_span = field.x_domain.1 - field.x_domain.0;
    let y_span = field.y_domain.1 - field.y_domain.0;
    if x_span.abs() <= 1e-12 || y_span.abs() <= 1e-12 {
        return None;
    }
    let u = (x - field.x_domain.0) / x_span;
    let v = (y - field.y_domain.0) / y_span;
    if !(-1e-9..=1.0 + 1e-9).contains(&u) || !(-1e-9..=1.0 + 1e-9).contains(&v) {
        return None;
    }
    let u = u.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let fx = u * (field.cols - 1) as f64;
    let fy = v * (field.rows - 1) as f64;
    let c0 = (fx.floor() as usize).min(field.cols - 2);
    let r0 = (fy.floor() as usize).min(field.rows - 2);
    let tx = fx - c0 as f64;
    let ty = fy - r0 as f64;
    let v00 = field.values[r0 * field.cols + c0];
    let v10 = field.values[r0 * field.cols + c0 + 1];
    let v01 = field.values[(r0 + 1) * field.cols + c0];
    let v11 = field.values[(r0 + 1) * field.cols + c0 + 1];
    Some(
        v00 * (1.0 - tx) * (1.0 - ty)
            + v10 * tx * (1.0 - ty)
            + v01 * (1.0 - tx) * ty
            + v11 * tx * ty,
    )
}

fn steps(ink: Hsla) -> [Hsla; 5] {
    [0.14, 0.32, 0.5, 0.72, 0.92].map(|s| ink.opacity(s))
}

impl Styled for ContourChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ContourChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<ContourState> =
            window.use_keyed_state((self.id.clone(), "contour"), cx, |_, _| {
                ContourState::default()
            });
        let (bounds, pointer_at, domain) = {
            let s = state.read(cx);
            (s.bounds, s.hover, s.domain)
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
        let base_domain = self.base_domain();
        let active_domain = domain.unwrap_or(base_domain);
        let field = Rc::new(self.build_field(domain));
        let has_data = field.cols >= 2 && field.rows >= 2;
        let x_scale = Linear::new(active_domain.0, (rect.x, rect.x + rect.w));
        let y_scale = Linear::new(active_domain.1, (rect.y + rect.h, rect.y));
        let bands = build_bands(
            &field,
            x_scale,
            y_scale,
            self.levels,
            self.thresholds.as_deref(),
        );
        let (x_tick_vals, y_tick_vals) = if domain.is_some() {
            (
                domain_ticks(active_domain.0.0, active_domain.0.1),
                domain_ticks(active_domain.1.0, active_domain.1.1),
            )
        } else {
            (
                nice(active_domain.0.0, active_domain.0.1, 5).1,
                nice(active_domain.1.0, active_domain.1.1, 5).1,
            )
        };
        let hover_dims = (field.cols.max(24), field.rows.max(24));
        let format = self.format.clone();
        let ink = tint(&colors, 0);
        let (x_name, y_name) = self.axes.clone();

        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every_x = ((x_tick_vals.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        let x_labels: Vec<_> = x_tick_vals
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every_x == 0)
            .map(|(_, val)| below(x_scale.at(*val), rect, tabular(text(format(*val).into()))))
            .collect();
        let y_labels: Vec<_> = y_tick_vals
            .iter()
            .map(|val| {
                beside(
                    y_scale.at(*val),
                    sizes.gutter.to_pixels(rem),
                    tabular(text(format(*val).into())),
                )
            })
            .collect();

        let tooltip = pointer_at.and_then(|key_id| {
            if field.cols < 2 || field.rows < 2 {
                return None;
            }
            let c = key_id % hover_dims.0;
            let r = key_id / hover_dims.0;
            if r >= hover_dims.1 {
                return None;
            }
            let tx = c as f64 / (hover_dims.0 - 1) as f64;
            let ty = r as f64 / (hover_dims.1 - 1) as f64;
            let x = active_domain.0.0 + (active_domain.0.1 - active_domain.0.0) * tx;
            let y = active_domain.1.0 + (active_domain.1.1 - active_domain.1.0) * ty;
            let val = sample_field_at(&field, x, y)?;
            let px = x_scale.at(x);
            let py = y_scale.at(y);
            let card = ChartTooltip::new("Contour")
                .row(Some(ink), "Level", format(val))
                .row(None, x_name.clone(), format(x))
                .row(None, y_name.clone(), format(y));
            Some(anchored((px, py), px > box_size.0 * 0.55, card))
        });

        let y_ticks_px: Vec<f32> = y_tick_vals.iter().map(|v| y_scale.at(*v)).collect();
        let (hairline, grid_color) = (sizes.hairline, colors.border.opacity(0.6));
        let legend = key(steps(ink), ("Low", "High"), cx);
        let field_hit = field.clone();
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let find_hover = Rc::new(move |pos: (f32, f32)| {
            if !rect.contains(pos) || field_hit.cols < 2 || field_hit.rows < 2 {
                return None;
            }
            let x = x_scale.value(pos.0);
            let y = y_scale.value(pos.1);
            let _ = sample_field_at(&field_hit, x, y)?;
            let u = ((pos.0 - rect.x) / rect.w.max(1.0)).clamp(0.0, 1.0);
            let v = ((rect.y + rect.h - pos.1) / rect.h.max(1.0)).clamp(0.0, 1.0);
            let c = (u * (hover_dims.0 - 1) as f32).round() as usize;
            let r = (v * (hover_dims.1 - 1) as f32).round() as usize;
            Some(r.min(hover_dims.1 - 1) * hover_dims.0 + c.min(hover_dims.0 - 1))
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
        let end_drag = move |target: &Entity<ContourState>, cx: &mut App| {
            target.update(cx, |s, cx| {
                s.drag_last = None;
                if s.dragged {
                    s.dragged = false;
                    cx.notify();
                }
            });
        };

        let chart = div()
            .id((self.id.clone(), "contour"))
            .debug_selector(|| "contour-plot".into())
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
                        for &y_px in &y_ticks_px {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, y_px)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                grid_color,
                            ));
                        }
                        window.with_content_mask(Some(mask), |window| {
                            let count = bands.len().max(1);
                            for (lvl_ix, band) in bands.iter().enumerate() {
                                let _ = band.threshold;
                                let alpha = 0.12 + 0.52 * ((lvl_ix + 1) as f32 / count as f32);
                                let fill_color = ink.opacity(alpha);
                                for poly in &band.polygons {
                                    if poly.len() < 3 {
                                        continue;
                                    }
                                    let mut path = PathBuilder::fill();
                                    path.move_to(at(origin, poly[0]));
                                    for &pt in &poly[1..] {
                                        path.line_to(at(origin, pt));
                                    }
                                    path.close();
                                    finish(path, fill_color, window);
                                }
                                let stroke_color = ink.opacity(0.65);
                                for &(a, b) in &band.isolines {
                                    let mut line = PathBuilder::stroke(hairline);
                                    line.move_to(at(origin, a));
                                    line.line_to(at(origin, b));
                                    finish(line, stroke_color, window);
                                }
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
    fn marching_squares_and_kde_handle_peaks_flat_and_empty_inputs() {
        let empty = kde_field(&[], (20, 20), None, None);
        assert!(empty.values.is_empty());

        let grid = vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let chart = ContourChart::from_grid("g", 4, 4, grid).thresholds([0.5]);
        let field = chart.build_field(None);
        let bands = build_bands(
            &field,
            Linear::new(field.x_domain, (0.0, 90.0)),
            Linear::new(field.y_domain, (90.0, 0.0)),
            4,
            Some(&[0.5]),
        );
        assert_eq!(bands.len(), 1);
        assert!(!bands[0].polygons.is_empty());
        assert!(!bands[0].isolines.is_empty());

        let flat = vec![2.0; 9];
        let flat_chart = ContourChart::from_grid("flat", 3, 3, flat);
        let flat_field = flat_chart.build_field(None);
        let flat_bands = build_bands(
            &flat_field,
            Linear::new(flat_field.x_domain, (0.0, 50.0)),
            Linear::new(flat_field.y_domain, (50.0, 0.0)),
            4,
            None,
        );
        assert!(flat_bands.is_empty());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn contour_chart_renders_kde_and_exports_svg_csv(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{
            Context, Modifiers, PinchEvent, Render, ScrollDelta, ScrollEvent, Window, point, px,
        };
        use std::cell::RefCell;

        struct ContourHarness(Rc<RefCell<Vec<f64>>>);
        impl Render for ContourHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let seen = self.0.clone();
                let pts = vec![
                    (20.0, 30.0),
                    (22.0, 31.0),
                    (24.0, 29.0),
                    (70.0, 65.0),
                    (72.0, 68.0),
                ];
                div().w(px(360.0)).h(px(260.0)).child(
                    ContourChart::from_points("kde-test", pts)
                        .grid_size(16, 16)
                        .levels(4)
                        .format(move |v| {
                            seen.borrow_mut().push(v);
                            compact(v)
                        }),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            let c = ContourChart::from_points("export", [(1.0, 2.0), (2.0, 3.0), (5.0, 6.0)])
                .grid_size(12, 12)
                .levels(3);
            let svg = c.svg(320.0, 220.0, cx);
            assert!(svg.starts_with("<svg") && svg.contains("<path"));
            assert!(c.csv().starts_with("col,row,x,y,value\n"));
        });

        let recorded = Rc::new(RefCell::new(Vec::new()));
        let (_, vc) = cx.add_window_view(|_, _| ContourHarness(recorded.clone()));
        settle(vc);
        let bounds = vc
            .debug_bounds("contour-plot")
            .expect("contour plot bounds");
        let rect_center = point(
            bounds.left() + px(48.0) + (bounds.size.width - px(56.0)) * 0.5,
            bounds.top() + px(8.0) + (bounds.size.height - px(36.0)) * 0.5,
        );
        vc.simulate_mouse_move(rect_center, None, Modifiers::none());
        settle(vc);
        assert!(
            vc.debug_bounds("chart-tooltip").is_some(),
            "hover over contour opens tooltip"
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
        assert!(zoomed_span < 70.0, "wheel zoom shrinks visible domain");

        // Left-button drag (> 3px) pans the domain.
        recorded.borrow_mut().clear();
        vc.simulate_mouse_down(rect_center, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(
            rect_center + point(px(90.0), px(-90.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            rect_center + point(px(90.0), px(-90.0)),
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

        // Verify grid source also supports zoom, pan, and reset.
        struct GridHarness;
        impl Render for GridHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let grid = vec![
                    0.0, 0.2, 0.2, 0.0, 0.2, 1.0, 1.0, 0.2, 0.2, 1.0, 1.0, 0.2, 0.0, 0.2, 0.2, 0.0,
                ];
                div()
                    .w(px(360.0))
                    .h(px(260.0))
                    .child(ContourChart::from_grid("grid-test", 4, 4, grid).levels(3))
            }
        }
        let (_, gvc) = cx.add_window_view(|_, _| GridHarness);
        settle(gvc);
        let gbounds = gvc
            .debug_bounds("contour-plot")
            .expect("grid contour plot bounds");
        let gcenter = point(
            gbounds.left() + px(48.0) + (gbounds.size.width - px(56.0)) * 0.5,
            gbounds.top() + px(8.0) + (gbounds.size.height - px(36.0)) * 0.5,
        );
        gvc.simulate_event(ScrollEvent {
            position: gcenter,
            delta: ScrollDelta::Pixels(point(px(0.0), px(150.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        });
        settle(gvc);
        assert!(gvc.debug_bounds("show-all").is_some());
        gvc.simulate_mouse_down(gcenter, MouseButton::Left, Modifiers::none());
        gvc.simulate_mouse_move(
            gcenter + point(px(20.0), px(0.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        gvc.simulate_mouse_up(
            gcenter + point(px(20.0), px(0.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(gvc);
        let greset = gvc.debug_bounds("show-all").expect("grid show-all");
        gvc.simulate_click(greset.center(), Modifiers::none());
        settle(gvc);
        assert!(gvc.debug_bounds("show-all").is_none());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn contour_touch_mode_tap_pinch_scroll_and_reset(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, PinchEvent, Render, ScrollDelta, Window, point, px};
        use std::cell::RefCell;

        struct NarrowContour(Rc<RefCell<Vec<f64>>>);
        impl Render for NarrowContour {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let seen = self.0.clone();
                let pts = vec![(10.0, 20.0), (50.0, 50.0), (90.0, 80.0)];
                div().w(px(240.0)).h(px(240.0)).child(
                    ContourChart::from_points("contour-touch", pts)
                        .grid_size(12, 12)
                        .levels(3)
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
        let (_, vc) = cx.add_window_view(|_, _| NarrowContour(recorded.clone()));
        vc.simulate_resize(size(px(240.0), px(240.0)));
        settle(vc);

        let plot = vc.debug_bounds("contour-plot").expect("contour plot");
        let right_tap = point(plot.left() + plot.size.width * 0.78, plot.center().y);
        vc.simulate_mouse_down(right_tap, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(right_tap, MouseButton::Left, Modifiers::none());
        settle(vc);
        let tip = vc.debug_bounds("chart-tooltip").expect("tap opens tooltip");
        assert!(
            tip.right() <= plot.right() + px(1.0),
            "right-side tooltip flips inside narrow 240px card"
        );

        // Single-finger down followed by pinch clears drag_last so trailing move does not pan.
        let center = plot.center();
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        vc.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        settle(vc);
        recorded.borrow_mut().clear();
        vc.simulate_event(PinchEvent {
            position: center,
            delta: 0.5,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        recorded.borrow_mut().clear();
        vc.update(|window, _| window.refresh());
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
            center + point(px(70.0), px(-70.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            center + point(px(70.0), px(-70.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        vc.simulate_mouse_move(point(px(2.0), px(2.0)), None, Modifiers::none());
        settle(vc);
        recorded.borrow_mut().clear();
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
