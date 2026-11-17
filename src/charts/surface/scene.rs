use super::*;

#[derive(Clone, Debug)]
pub(super) struct SurfaceQuad {
    pub(super) col: usize,
    pub(super) row: usize,
    pub(super) z: f64,
    pub(super) t: f32,
    pub(super) depth: f32,
    pub(super) pts: [(f32, f32); 4],
    pub(super) center: (f32, f32),
}

pub(super) struct SurfaceScene {
    pub(super) quads: Vec<SurfaceQuad>,
    pub(super) axes: Vec<((f32, f32), (f32, f32))>,
    pub(super) axis_labels: Vec<((f32, f32), SharedString)>,
}

pub(super) fn project_raw(nx: f32, ny: f32, nz: f32, az: f32, el: f32) -> (f32, f32, f32) {
    let (sin_az, cos_az) = az.sin_cos();
    let (sin_el, cos_el) = el.sin_cos();
    let rx = nx * cos_az - ny * sin_az;
    let ry = nx * sin_az + ny * cos_az;
    let depth = ry * cos_el - nz * sin_el;
    let up = ry * sin_el + nz * cos_el;
    let persp = 3.6 / (3.6 + depth);
    (rx * persp, -up * persp, depth)
}

pub(super) fn build_surface_scene(
    cols: usize,
    rows: usize,
    values: &[f64],
    azimuth_deg: f32,
    elevation_deg: f32,
    zoom: f32,
    rect: Rect,
) -> SurfaceScene {
    if cols < 2 || rows < 2 || rect.w <= 1.0 || rect.h <= 1.0 || values.len() != cols * rows {
        return SurfaceScene {
            quads: Vec::new(),
            axes: Vec::new(),
            axis_labels: Vec::new(),
        };
    }
    let (z_min, z_max) = values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let half_span = z_max * 0.5 - z_min * 0.5;
    let norm_z = |v: f64| -> f32 {
        if half_span > 0.0 {
            (((v * 0.5 - z_min * 0.5) / half_span) as f32).clamp(0.0, 1.0)
        } else {
            0.5
        }
    };
    let (az, el) = (azimuth_deg.to_radians(), elevation_deg.to_radians());

    // Fit the normalized [-0.5, 0.5]^3 bounding cube into rect.
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for &cx in &[-0.5_f32, 0.5] {
        for &cy in &[-0.5_f32, 0.5] {
            for &cz in &[-0.5_f32, 0.5] {
                let (px, py, _) = project_raw(cx, cy, cz, az, el);
                min_x = min_x.min(px);
                max_x = max_x.max(px);
                min_y = min_y.min(py);
                max_y = max_y.max(py);
            }
        }
    }
    let span_x = (max_x - min_x).max(1e-4);
    let span_y = (max_y - min_y).max(1e-4);
    let scale = (rect.w / span_x).min(rect.h / span_y) * 0.82 * zoom.clamp(0.5, 4.0);
    let offset_x = rect.x + rect.w * 0.5 - (min_x + max_x) * 0.5 * scale;
    let offset_y = rect.y + rect.h * 0.5 - (min_y + max_y) * 0.5 * scale;
    let to_screen = |nx: f32, ny: f32, nz: f32| -> ((f32, f32), f32) {
        let (px, py, depth) = project_raw(nx, ny, nz, az, el);
        ((offset_x + px * scale, offset_y + py * scale), depth)
    };

    let mut projected = Vec::with_capacity(cols * rows);
    let mut heights = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let ny = r as f32 / (rows - 1) as f32 - 0.5;
        for c in 0..cols {
            let nx = c as f32 / (cols - 1) as f32 - 0.5;
            let t = norm_z(values[r * cols + c]);
            heights.push(t);
            projected.push(to_screen(nx, ny, t - 0.5));
        }
    }

    let mut quads = Vec::with_capacity((cols - 1) * (rows - 1));
    for r in 0..rows - 1 {
        for c in 0..cols - 1 {
            let i00 = r * cols + c;
            let i10 = r * cols + c + 1;
            let i11 = (r + 1) * cols + c + 1;
            let i01 = (r + 1) * cols + c;
            let (p00, d00) = projected[i00];
            let (p10, d10) = projected[i10];
            let (p11, d11) = projected[i11];
            let (p01, d01) = projected[i01];
            let depth = (d00 + d10 + d11 + d01) * 0.25;
            let t = (heights[i00] + heights[i10] + heights[i11] + heights[i01]) * 0.25;
            let z = (values[i00] * 0.25 + values[i10] * 0.25)
                + (values[i11] * 0.25 + values[i01] * 0.25);
            let center = (
                (p00.0 + p10.0 + p11.0 + p01.0) * 0.25,
                (p00.1 + p10.1 + p11.1 + p01.1) * 0.25,
            );
            quads.push(SurfaceQuad {
                col: c,
                row: r,
                z,
                t,
                depth,
                pts: [p00, p10, p11, p01],
                center,
            });
        }
    }
    // Painter's algorithm: farthest depth drawn first.
    quads.sort_by(|a, b| b.depth.total_cmp(&a.depth));

    // 3D bounding base frame on z = -0.5 plus vertical Z axis at the backmost base corner.
    let base_corners = [(-0.5_f32, -0.5_f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
    let base_proj: Vec<((f32, f32), f32)> = base_corners
        .iter()
        .map(|&(bx, by)| to_screen(bx, by, -0.5))
        .collect();
    let mut axes = vec![
        (base_proj[0].0, base_proj[1].0),
        (base_proj[1].0, base_proj[2].0),
        (base_proj[2].0, base_proj[3].0),
        (base_proj[3].0, base_proj[0].0),
    ];
    let back_ix = base_proj
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.1.total_cmp(&b.1.1))
        .map(|(ix, _)| ix)
        .unwrap_or(0);
    let (bx, by) = base_corners[back_ix];
    let (z_top, _) = to_screen(bx, by, 0.5);
    axes.push((base_proj[back_ix].0, z_top));

    let (x_mid, _) = to_screen(0.0, -0.58, -0.5);
    let (y_mid, _) = to_screen(0.58, 0.0, -0.5);
    let (z_lbl, _) = to_screen(bx, by, 0.57);
    let axis_labels = vec![
        (x_mid, SharedString::from("X")),
        (y_mid, SharedString::from("Y")),
        (z_lbl, SharedString::from("Z")),
    ];

    SurfaceScene {
        quads,
        axes,
        axis_labels,
    }
}

pub(super) fn point_in_quad((px, py): (f32, f32), pts: [(f32, f32); 4]) -> bool {
    let mut inside = false;
    let mut j = 3;
    for i in 0..4 {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        let crosses = (yi > py) != (yj > py);
        if crosses {
            let denom = yj - yi;
            if denom.abs() > 1e-6 && px < (xj - xi) * (py - yi) / denom + xi {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

pub(super) struct SurfaceState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) azimuth_offset: f32,
    pub(super) elevation_offset: f32,
    pub(super) zoom: f32,
    pub(super) drag_last: Option<(f32, f32)>,
}

impl Default for SurfaceState {
    fn default() -> Self {
        Self {
            bounds: Bounds::default(),
            hover: None,
            azimuth_offset: 0.0,
            elevation_offset: 0.0,
            zoom: 1.0,
            drag_last: None,
        }
    }
}
