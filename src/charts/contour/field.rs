use super::*;

pub(super) struct ScalarField {
    pub(super) cols: usize,
    pub(super) rows: usize,
    pub(super) x_domain: (f64, f64),
    pub(super) y_domain: (f64, f64),
    pub(super) values: Vec<f64>,
}

pub(super) struct ContourLevel {
    pub(super) threshold: f64,
    pub(super) polygons: Vec<Vec<(f32, f32)>>,
    pub(super) isolines: Vec<Segment>,
}

pub(super) fn svg_hex(color: Hsla) -> (String, f32) {
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

pub(super) fn local_pos(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

pub(super) fn domain_ticks(low: f64, high: f64) -> Vec<f64> {
    let (_, ticks) = nice(low, high, 5);
    let tol = (high - low).abs() * 1e-9;
    ticks
        .into_iter()
        .filter(|v| *v >= low - tol && *v <= high + tol)
        .collect()
}

pub(super) fn pan_domain(((x0, x1), (y0, y1)): Domain, rect: Rect, dx: f32, dy: f32) -> Domain {
    let shift_x = -(f64::from(dx) / f64::from(rect.w.max(1.0))) * (x1 - x0);
    let shift_y = (f64::from(dy) / f64::from(rect.h.max(1.0))) * (y1 - y0);
    ((x0 + shift_x, x1 + shift_x), (y0 + shift_y, y1 + shift_y))
}

pub(super) fn zoom_axis(
    (lo, hi): (f64, f64),
    base_span: f64,
    anchor: f64,
    factor: f64,
) -> (f64, f64) {
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

pub(super) fn zoom_domain(
    current: Domain,
    base: Domain,
    anchor: (f64, f64),
    factor: f64,
) -> Domain {
    let bx = (base.0.1 - base.0.0).abs().max(1e-9);
    let by = (base.1.1 - base.1.0).abs().max(1e-9);
    (
        zoom_axis(current.0, bx, anchor.0, factor),
        zoom_axis(current.1, by, anchor.1, factor),
    )
}

pub(super) fn cursor_data((x_dom, y_dom): Domain, rect: Rect, pos: (f32, f32)) -> (f64, f64) {
    let px = pos.0.clamp(rect.x, rect.x + rect.w);
    let py = pos.1.clamp(rect.y, rect.y + rect.h);
    let x_scale = Linear::new(x_dom, (rect.x, rect.x + rect.w));
    let y_scale = Linear::new(y_dom, (rect.y + rect.h, rect.y));
    (x_scale.value(px), y_scale.value(py))
}

pub(super) fn points_domain(points: &[(f64, f64)]) -> Domain {
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

pub(super) fn kde_field(
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

pub(super) fn resolve_thresholds(
    field: &ScalarField,
    levels: usize,
    explicit: Option<&[f64]>,
) -> Vec<f64> {
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
pub(super) fn build_bands(
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

pub(super) fn interp(a: (f32, f32, f64), b: (f32, f32, f64), threshold: f64) -> (f32, f32) {
    let denom = b.2 - a.2;
    let t = if denom.abs() > 1e-15 {
        ((threshold - a.2) / denom).clamp(0.0, 1.0) as f32
    } else {
        0.5
    };
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

pub(super) fn clip_cell_above(corners: &[(f32, f32, f64); 4], threshold: f64) -> Vec<(f32, f32)> {
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

pub(super) fn marching_cell_segments(
    corners: &[(f32, f32, f64); 4],
    threshold: f64,
    out: &mut Vec<Segment>,
) {
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

pub(super) fn sample_field_at(field: &ScalarField, x: f64, y: f64) -> Option<f64> {
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

pub(super) fn steps(ink: Hsla) -> [Hsla; 5] {
    [0.14, 0.32, 0.5, 0.72, 0.92].map(|s| ink.opacity(s))
}
