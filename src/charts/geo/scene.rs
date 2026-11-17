use super::*;

pub(super) fn build_scene(chart: &GeoChart, size: (f32, f32), pad: f32, view: GeoView) -> GeoScene {
    let avail_w = (size.0 - pad * 2.0).max(0.0);
    let avail_h = (size.1 - pad * 2.0).max(0.0);
    if avail_w <= 1.0 || avail_h <= 1.0 {
        return GeoScene {
            globe: None,
            frame_x: 0.0,
            frame_y: 0.0,
            frame_w: 0.0,
            frame_h: 0.0,
            graticule: Vec::new(),
            regions: Vec::new(),
            points: Vec::new(),
        };
    }

    let (dom_w, dom_h) = projection_bounds(chart.projection);
    let fit_scale = (avail_w / (2.0 * dom_w as f32)).min(avail_h / (2.0 * dom_h as f32));
    let scale = fit_scale * view.zoom.clamp(0.5, 12.0);
    let (cx_px, cy_px) = (size.0 * 0.5 + view.pan.0, size.1 * 0.5 + view.pan.1);
    let to_px = |(nx, ny): (f64, f64)| (cx_px + nx as f32 * scale, cy_px - ny as f32 * scale);

    let globe = match chart.projection {
        GeoProjection::Orthographic | GeoProjection::Stereographic => Some((cx_px, cy_px, scale)),
        GeoProjection::Equirectangular | GeoProjection::Mercator => None,
    };
    let frame_w = 2.0 * dom_w as f32 * scale;
    let frame_h = 2.0 * dom_h as f32 * scale;
    let frame_x = cx_px - frame_w * 0.5;
    let frame_y = cy_px - frame_h * 0.5;

    let mut graticule = Vec::new();
    if chart.show_graticule {
        let step = chart.graticule_step;
        let lat_limit = match chart.projection {
            GeoProjection::Mercator => 85.0,
            _ => 90.0,
        };
        let mut lon = -180.0_f64;
        while lon <= 180.0 + 1e-6 {
            let mut meridian = Vec::new();
            let mut lat = -lat_limit;
            while lat < lat_limit {
                meridian.push((lon.clamp(-180.0, 180.0), lat));
                lat += 4.0;
            }
            meridian.push((lon.clamp(-180.0, 180.0), lat_limit));
            graticule.extend(project_polyline(
                &meridian,
                view.rotate,
                chart.projection,
                to_px,
            ));
            lon += step;
        }
        let mut lat = -((90.0 / step).floor() * step);
        while lat <= 90.0 - 1e-6 {
            if lat.abs() <= lat_limit {
                let mut parallel = Vec::new();
                let mut l = -180.0;
                while l < 180.0 {
                    parallel.push((l, lat));
                    l += 4.0;
                }
                parallel.push((180.0, lat));
                graticule.extend(project_polyline(
                    &parallel,
                    view.rotate,
                    chart.projection,
                    to_px,
                ));
            }
            lat += step;
        }
    }

    let threshold = clip_threshold(chart.projection);
    let mut regions = Vec::new();
    for (r_ix, region) in chart.regions.iter().enumerate() {
        let mut projected_rings = Vec::new();
        for ring in &region.rings {
            if ring.len() < 3 {
                continue;
            }
            let vecs: Vec<[f64; 3]> = ring
                .iter()
                .map(|&(lon, lat)| rotate_vec(lon, lat, view.rotate))
                .collect();
            let vecs = if let Some(thresh) = threshold {
                let dense = subdivide_ring(&vecs);
                clip_ring_horizon(&dense, thresh)
            } else {
                vecs
            };
            let px_ring: Vec<(f32, f32)> = vecs
                .into_iter()
                .filter_map(|v| project_vec(v, chart.projection))
                .map(to_px)
                .collect();
            if px_ring.len() >= 3 {
                projected_rings.push(px_ring);
            }
        }
        if !projected_rings.is_empty() {
            let anchor = polygon_anchor(&projected_rings[0]);
            regions.push(ProjectedRegion {
                index: r_ix,
                value: region.value,
                rings: projected_rings,
                anchor,
            });
        }
    }

    let max_pt_val = chart
        .points
        .iter()
        .map(|p| p.value.max(0.0))
        .fold(0.0_f64, f64::max);
    let mut points = Vec::new();
    for (p_ix, pt) in chart.points.iter().enumerate() {
        let v = rotate_vec(pt.lon, pt.lat, view.rotate);
        if let Some(norm) = project_vec(v, chart.projection) {
            let center = to_px(norm);
            let radius = if max_pt_val > 0.0 {
                let ratio = (pt.value.max(0.0) / max_pt_val).sqrt() as f32;
                3.5 + 8.5 * ratio
            } else {
                5.5
            };
            points.push(ProjectedPoint {
                index: p_ix,
                center,
                radius,
            });
        }
    }

    GeoScene {
        globe,
        frame_x,
        frame_y,
        frame_w,
        frame_h,
        graticule,
        regions,
        points,
    }
}

pub(super) fn polygon_anchor(ring: &[(f32, f32)]) -> (f32, f32) {
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for &(x, y) in ring {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
}

pub(super) fn point_in_polygon((px, py): (f32, f32), ring: &[(f32, f32)]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for (i, &(xi, yi)) in ring.iter().enumerate() {
        let (xj, yj) = ring[j];
        let intersects = ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}
