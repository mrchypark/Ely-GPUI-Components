use super::*;

pub(super) struct HexLayout {
    pub(super) bins: Vec<HexBin>,
    pub(super) max_count: usize,
    pub(super) radius: f32,
    pub(super) x_ticks: Vec<(f32, f64)>,
    pub(super) y_ticks: Vec<(f32, f64)>,
    pub(super) base_domain: Domain,
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

pub(super) fn strength(count: usize, max_count: usize) -> f32 {
    if max_count <= 1 {
        0.65
    } else {
        0.14 + 0.86 * (count as f32 / max_count as f32)
    }
}

pub(super) fn steps(ink: Hsla) -> [Hsla; 5] {
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
pub(super) fn snap_hex(px: f32, py: f32, radius: f32) -> ((i32, i32), (f32, f32)) {
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

pub(super) fn compute_layout(
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
