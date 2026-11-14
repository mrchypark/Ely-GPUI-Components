use super::*;

pub(super) fn validate_ring(ring: impl IntoIterator<Item = (f64, f64)>) -> Vec<(f64, f64)> {
    let pts: Vec<(f64, f64)> = ring.into_iter().collect();
    for &(lon, lat) in &pts {
        assert!(
            lon.is_finite()
                && (-180.0..=180.0).contains(&lon)
                && lat.is_finite()
                && (-90.0..=90.0).contains(&lat),
            "geo ring coordinates must be within lon -180..=180 and lat -90..=90"
        );
    }
    pts
}

pub(super) fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

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

pub(super) fn write_polyline_svg(out: &mut String, pts: &[(f32, f32)]) {
    if let Some((first, rest)) = pts.split_first() {
        write!(out, "M{:.1},{:.1}", first.0, first.1).expect("svg write");
        for &(x, y) in rest {
            write!(out, " L{x:.1},{y:.1}").expect("svg write");
        }
    }
}

pub(super) fn write_polygon_svg(out: &mut String, pts: &[(f32, f32)]) {
    write_polyline_svg(out, pts);
    out.push_str(" Z ");
}

pub(super) fn region_extent(regions: &[GeoRegion]) -> (f64, f64) {
    if regions.is_empty() {
        return (0.0, 1.0);
    }
    regions
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), r| {
            (lo.min(r.value), hi.max(r.value))
        })
}

pub(super) fn strength(value: f64, low: f64, high: f64) -> f32 {
    if high > low {
        0.14 + 0.86 * ((value - low) / (high - low)) as f32
    } else {
        0.55
    }
}

pub(super) fn steps(ink: Hsla) -> [Hsla; 5] {
    [0.14, 0.35, 0.56, 0.78, 1.0].map(|s| ink.opacity(s))
}

pub(super) fn centroid_deg(ring: &[(f64, f64)]) -> Option<(f64, f64)> {
    if ring.is_empty() {
        return None;
    }
    let (sx, sy) = ring
        .iter()
        .fold((0.0, 0.0), |(ax, ay), &(x, y)| (ax + x, ay + y));
    let n = ring.len() as f64;
    Some((sx / n, sy / n))
}

/// Rotate `(lon_deg, lat_deg)` by `(yaw_deg, pitch_deg)` into a 3D unit vector `(x, y, z)` (`+x` viewer, `+y` East, `+z` North).
pub(super) fn rotate_vec(lon_deg: f64, lat_deg: f64, (yaw_deg, pitch_deg): (f64, f64)) -> [f64; 3] {
    let dl = (lon_deg + yaw_deg).to_radians();
    let phi = lat_deg.to_radians();
    let x = phi.cos() * dl.cos();
    let y = phi.cos() * dl.sin();
    let z = phi.sin();
    let p = pitch_deg.to_radians();
    let (sp, cp) = p.sin_cos();
    let x1 = x * cp - z * sp;
    let z1 = x * sp + z * cp;
    [x1, y, z1]
}
