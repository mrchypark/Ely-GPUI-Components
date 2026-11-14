use super::*;

pub(super) const MERCATOR_MAX_LAT_RAD: f64 = 85.0 * PI / 180.0;
pub(super) const STEREO_CLIP_COS: f64 = -0.5;
pub(super) const STEREO_CLIP_SCALE: f64 = 1.732_050_807_568_877_2;

pub(super) fn mercator_max_y() -> f64 {
    (FRAC_PI_4 + MERCATOR_MAX_LAT_RAD * 0.5).tan().ln()
}

pub(super) fn projection_bounds(proj: GeoProjection) -> (f64, f64) {
    match proj {
        GeoProjection::Equirectangular => (PI, FRAC_PI_2),
        GeoProjection::Mercator => (PI, mercator_max_y()),
        GeoProjection::Orthographic | GeoProjection::Stereographic => (1.0, 1.0),
    }
}

pub(super) fn clip_threshold(proj: GeoProjection) -> Option<f64> {
    match proj {
        GeoProjection::Orthographic => Some(0.0),
        GeoProjection::Stereographic => Some(STEREO_CLIP_COS),
        GeoProjection::Equirectangular | GeoProjection::Mercator => None,
    }
}

/// Project a rotated 3D unit vector `[x, y, z]` to normalized `(nx, ny)` coordinates (`+ny` North).
pub(super) fn project_vec(v: [f64; 3], proj: GeoProjection) -> Option<(f64, f64)> {
    let [x, y, z] = v;
    match proj {
        GeoProjection::Equirectangular => {
            let lon = y.atan2(x);
            let lat = z.clamp(-1.0, 1.0).asin();
            Some((lon, lat))
        }
        GeoProjection::Mercator => {
            let lon = y.atan2(x);
            let lat = z
                .clamp(-1.0, 1.0)
                .asin()
                .clamp(-MERCATOR_MAX_LAT_RAD, MERCATOR_MAX_LAT_RAD);
            let ny = (FRAC_PI_4 + lat * 0.5).tan().ln();
            Some((lon, ny))
        }
        GeoProjection::Orthographic => {
            if x < 0.0 {
                None
            } else {
                Some((y, z))
            }
        }
        GeoProjection::Stereographic => {
            if x < STEREO_CLIP_COS {
                None
            } else {
                let denom = (1.0 + x) * STEREO_CLIP_SCALE;
                if denom <= 1e-6 {
                    None
                } else {
                    Some((y / denom, z / denom))
                }
            }
        }
    }
}

/// Subdivide spherical edges larger than ~5 deg so projected arcs and horizon clips remain smooth.
pub(super) fn subdivide_ring(vecs: &[[f64; 3]]) -> Vec<[f64; 3]> {
    if vecs.len() < 2 {
        return vecs.to_vec();
    }
    let max_angle = 5.0_f64.to_radians();
    let mut out = Vec::with_capacity(vecs.len() * 2);
    for (ix, &a) in vecs.iter().enumerate() {
        let b = vecs[(ix + 1) % vecs.len()];
        out.push(a);
        let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
        let angle = dot.acos();
        if angle > max_angle {
            let steps = (angle / max_angle).ceil() as usize;
            for s in 1..steps {
                let t = s as f64 / steps as f64;
                let p = [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ];
                let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-12);
                out.push([p[0] / len, p[1] / len, p[2] / len]);
            }
        }
    }
    out
}

pub(super) fn horizon_intersect(a: [f64; 3], b: [f64; 3], threshold: f64) -> [f64; 3] {
    let denom = b[0] - a[0];
    let t = if denom.abs() < 1e-12 {
        0.5
    } else {
        ((threshold - a[0]) / denom).clamp(0.0, 1.0)
    };
    let y = a[1] + (b[1] - a[1]) * t;
    let z = a[2] + (b[2] - a[2]) * t;
    let r_yz = (1.0 - threshold * threshold).max(0.0).sqrt();
    let len = (y * y + z * z).sqrt();
    if len > 1e-12 {
        [threshold, y / len * r_yz, z / len * r_yz]
    } else {
        [threshold, r_yz, 0.0]
    }
}

/// Sutherland-Hodgman clip of a closed 3D spherical ring against `v[0] >= threshold`.
pub(super) fn clip_ring_horizon(vecs: &[[f64; 3]], threshold: f64) -> Vec<[f64; 3]> {
    if vecs.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(vecs.len());
    let n = vecs.len();
    for (ix, &curr) in vecs.iter().enumerate() {
        let prev = vecs[(ix + n - 1) % n];
        let prev_in = prev[0] >= threshold;
        let curr_in = curr[0] >= threshold;
        match (prev_in, curr_in) {
            (true, true) => out.push(curr),
            (false, true) => {
                out.push(horizon_intersect(prev, curr, threshold));
                out.push(curr);
            }
            (true, false) => {
                out.push(horizon_intersect(prev, curr, threshold));
            }
            (false, false) => {}
        }
    }
    out
}

pub(super) fn project_polyline(
    pts_deg: &[(f64, f64)],
    rotate: (f64, f64),
    proj: GeoProjection,
    to_px: impl Fn((f64, f64)) -> (f32, f32),
) -> Vec<Vec<(f32, f32)>> {
    let vecs: Vec<[f64; 3]> = pts_deg
        .iter()
        .map(|&(lon, lat)| rotate_vec(lon, lat, rotate))
        .collect();
    let mut segments: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut current: Vec<(f32, f32)> = Vec::new();

    if let Some(threshold) = clip_threshold(proj) {
        for win in vecs.windows(2) {
            let (a, b) = (win[0], win[1]);
            let (a_in, b_in) = (a[0] >= threshold, b[0] >= threshold);
            match (a_in, b_in) {
                (true, true) => {
                    if current.is_empty()
                        && let Some(p) = project_vec(a, proj)
                    {
                        current.push(to_px(p));
                    }
                    if let Some(p) = project_vec(b, proj) {
                        current.push(to_px(p));
                    }
                }
                (true, false) => {
                    if current.is_empty()
                        && let Some(p) = project_vec(a, proj)
                    {
                        current.push(to_px(p));
                    }
                    let mid = horizon_intersect(a, b, threshold);
                    if let Some(p) = project_vec(mid, proj) {
                        current.push(to_px(p));
                    }
                    if current.len() >= 2 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
                (false, true) => {
                    let mid = horizon_intersect(a, b, threshold);
                    if let Some(p) = project_vec(mid, proj) {
                        current.push(to_px(p));
                    }
                    if let Some(p) = project_vec(b, proj) {
                        current.push(to_px(p));
                    }
                }
                (false, false) => {
                    if current.len() >= 2 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
            }
        }
    } else {
        let mut prev_nx: Option<f64> = None;
        for &v in &vecs {
            if let Some((nx, ny)) = project_vec(v, proj) {
                if let Some(px_prev) = prev_nx
                    && (nx - px_prev).abs() > PI
                {
                    if current.len() >= 2 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                }
                current.push(to_px((nx, ny)));
                prev_nx = Some(nx);
            }
        }
    }

    if current.len() >= 2 {
        segments.push(current);
    }
    segments
}
