use std::collections::HashSet;

use gpui::SharedString;

use super::{
    scale::{Band, ScaleMode, ValueScale, nice_for},
    series::{Points, Symbol, extent, stacked, stream_stacked},
};

/// How a category series draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    Line,
    Area,
    Bar,
}

/// What a mark paints with: a series' palette color, a rise or a fall, or the chart's own ink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ink {
    Series(usize),
    Rise,
    Fall,
    /// Connectors and whiskers, quiet.
    Rule,
    /// Medians and marks that read over a fill.
    Strong,
}

/// A place in a box's own pixels: across, then down.
pub(crate) type Place = (f32, f32);

/// A rectangle in a box's own pixels: left, top, width, height.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub(crate) fn contains(&self, (x, y): (f32, f32)) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

/// A run of points drawn as a line, filled down to its floor when it is an area.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Line {
    pub ink: Ink,
    pub points: Vec<(f32, f32)>,
    pub floor: Option<Vec<(f32, f32)>>,
}

/// A dot, and the index its tooltip reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dot {
    pub ink: Ink,
    pub center: (f32, f32),
    pub radius: f32,
    pub symbol: Symbol,
    pub key: usize,
}

/// What a chart draws, in its box's own pixels, for the canvas and for SVG alike.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Geometry {
    pub value_scale: Option<ValueScale>,
    /// Value ticks: where each sits along the value axis, and its value.
    pub ticks: Vec<(f32, f64)>,
    /// Labels along the other axis and where each sits: categories, or a scatter's x ticks.
    pub labels: Vec<(f32, SharedString)>,
    pub lines: Vec<Line>,
    pub bars: Vec<(Ink, Rect)>,
    pub dots: Vec<Dot>,
    /// Straight strokes from one point to another.
    pub strokes: Vec<(Ink, Place, Place)>,
    /// Closed outlines, filled.
    pub shapes: Vec<(Ink, Vec<(f32, f32)>)>,
}

/// The plot inside a box: past the gutter on the left and the foot below, the inset above and to the right.
pub(crate) fn frame((width, height): (f32, f32), gutter: f32, foot: f32, inset: f32) -> Rect {
    Rect {
        x: gutter,
        y: inset,
        w: (width - gutter - inset).max(0.0),
        h: (height - foot - inset).max(0.0),
    }
}

/// What a category chart asks of its geometry.
pub(crate) struct Categories<'a> {
    pub labels: &'a [SharedString],
    pub names: &'a [SharedString],
    pub values: &'a [Vec<f64>],
    pub mark: Mark,
    pub stacked: bool,
    pub stream: bool,
    pub horizontal: bool,
    /// The share of each band bars leave empty.
    pub gap: f32,
    pub y_scale: ScaleMode,
    pub hidden: &'a HashSet<SharedString>,
    pub span: (usize, usize),
}

/// Lays out categories and their series inside `frame`: bands along one axis, values along the other.
pub(crate) fn categories(chart: &Categories, frame: Rect) -> Geometry {
    if chart.labels.is_empty() {
        return Geometry::default();
    }
    let (first, last) = chart.span;
    let shown: Vec<(usize, &[f64])> = chart
        .names
        .iter()
        .zip(chart.values)
        .enumerate()
        .filter(|(_, (name, _))| !chart.hidden.contains(*name))
        .map(|(ix, (_, values))| (ix, &values[first..=last]))
        .collect();
    let runs: Vec<&[f64]> = shown.iter().map(|(_, values)| *values).collect();
    let stacks = if chart.stream {
        Some(stream_stacked(&runs))
    } else if chart.stacked {
        Some(stacked(&runs))
    } else {
        None
    };
    let (low, high) = if matches!(chart.y_scale, ScaleMode::Log(_)) {
        let pos = runs
            .iter()
            .flat_map(|v| v.iter().copied())
            .filter(|v| *v > 0.0)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        if pos.0 <= pos.1 { pos } else { (1.0, 10.0) }
    } else {
        match &stacks {
            Some(stacks) => extent(
                stacks
                    .iter()
                    .flatten()
                    .flat_map(|(bottom, top)| [*bottom, *top]),
            ),
            None => extent(runs.iter().flat_map(|values| values.iter().copied())),
        }
    };
    let ((low, high), ticks) = nice_for(chart.y_scale, low, high, 5);
    let count = last - first + 1;
    let (value_range, band_range) = if chart.horizontal {
        ((frame.x, frame.x + frame.w), (frame.y, frame.y + frame.h))
    } else {
        ((frame.y + frame.h, frame.y), (frame.x, frame.x + frame.w))
    };
    let scale = ValueScale::new(chart.y_scale, (low, high), value_range);
    let bars = chart.mark == Mark::Bar;
    let band = Band {
        count,
        range: band_range,
        padding: if bars { chart.gap } else { 0.0 },
    };
    let along = |ix: usize| {
        if bars {
            band.middle(ix)
        } else {
            point_at(&band, ix, count)
        }
    };
    let place = |band_at: f32, value_at: f32| {
        if chart.horizontal {
            (value_at, band_at)
        } else {
            (band_at, value_at)
        }
    };
    let mut geometry = Geometry {
        value_scale: Some(scale),
        ticks: ticks.iter().map(|tick| (scale.at(*tick), *tick)).collect(),
        labels: (0..count)
            .map(|ix| (along(ix), chart.labels[first + ix].clone()))
            .collect(),
        ..Geometry::default()
    };
    for (lane, (color, values)) in shown.iter().enumerate() {
        let baseline = if matches!(chart.y_scale, ScaleMode::Log(_)) {
            low
        } else {
            0.0
        };
        let spans: Vec<(f64, f64)> = match &stacks {
            Some(stacks) => stacks[lane].clone(),
            None => values.iter().map(|value| (baseline, *value)).collect(),
        };
        let ink = Ink::Series(*color);
        if bars {
            for (ix, (bottom, top)) in spans.iter().enumerate() {
                let (start, width) = band.slot(ix);
                let (start, width) = if chart.stacked || shown.len() == 1 {
                    (start, width)
                } else {
                    let each = width / shown.len() as f32;
                    (start + each * lane as f32 + each * 0.07, each * 0.86)
                };
                let (a, b) = (scale.at(*bottom), scale.at(*top));
                let (from, to) = (a.min(b), a.max(b));
                let rect = if chart.horizontal {
                    Rect {
                        x: from,
                        y: start,
                        w: to - from,
                        h: width,
                    }
                } else {
                    Rect {
                        x: start,
                        y: from,
                        w: width,
                        h: to - from,
                    }
                };
                geometry.bars.push((ink, rect));
            }
            continue;
        }
        let points = spans
            .iter()
            .enumerate()
            .map(|(ix, (_, top))| place(along(ix), scale.at(*top)))
            .collect();
        let floor = (chart.mark == Mark::Area).then(|| {
            spans
                .iter()
                .enumerate()
                .map(|(ix, (bottom, _))| place(along(ix), scale.at(*bottom)))
                .collect()
        });
        geometry.lines.push(Line { ink, points, floor });
    }
    geometry
}

/// Where point `ix` of `count` sits: spread edge to edge, or centered when alone.
pub(crate) fn point_at(band: &Band, ix: usize, count: usize) -> f32 {
    if count < 2 {
        return (band.range.0 + band.range.1) / 2.0;
    }
    band.range.0 + (band.range.1 - band.range.0) * ix as f32 / (count - 1) as f32
}

/// Lays out clouds of points: both axes linear, bubbles sized by area up to `widest`; x ticks read through `format`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn scatter(
    clouds: &[Points],
    hidden: &HashSet<SharedString>,
    frame: Rect,
    (widest, dot): (f32, f32),
    (x_mode, y_mode): (ScaleMode, ScaleMode),
    xy_view: Option<((f64, f64), (f64, f64))>,
    voronoi: bool,
    format: &dyn Fn(f64) -> String,
) -> Geometry {
    let shown = || clouds.iter().filter(|cloud| !hidden.contains(&cloud.name));
    let every = || shown().flat_map(|cloud| cloud.points.iter());
    let (x_low, x_high) = every()
        .filter(|(x, _)| !matches!(x_mode, ScaleMode::Log(_)) || *x > 0.0)
        .fold((f64::MAX, f64::MIN), |(low, high), (x, _)| {
            (low.min(*x), high.max(*x))
        });
    let (y_low, y_high) = every()
        .filter(|(_, y)| !matches!(y_mode, ScaleMode::Log(_)) || *y > 0.0)
        .fold((f64::MAX, f64::MIN), |(low, high), (_, y)| {
            (low.min(*y), high.max(*y))
        });
    if x_low > x_high || y_low > y_high {
        return Geometry::default();
    }
    let ((x_low, x_high), x_ticks) = nice_for(x_mode, x_low, x_high, 5);
    let ((y_low, y_high), y_ticks) = nice_for(y_mode, y_low, y_high, 5);
    let (xs, ys, x_ticks, y_ticks) = if let Some(((ux0, ux1), (uy0, uy1))) = xy_view {
        let base_x = ValueScale::new(x_mode, (x_low, x_high), (0.0, 1.0));
        let base_y = ValueScale::new(y_mode, (y_low, y_high), (0.0, 1.0));
        let (zx0, zx1) = (base_x.value(ux0 as f32), base_x.value(ux1 as f32));
        let (zy0, zy1) = (base_y.value(uy0 as f32), base_y.value(uy1 as f32));
        let (_, xt) = nice_for(x_mode, zx0, zx1, 5);
        let (_, yt) = nice_for(y_mode, zy0, zy1, 5);
        let xs = ValueScale::new(x_mode, (zx0, zx1), (frame.x, frame.x + frame.w));
        let ys = ValueScale::new(y_mode, (zy0, zy1), (frame.y + frame.h, frame.y));
        let xt: Vec<f64> = xt
            .into_iter()
            .filter(|t| {
                let px = xs.at(*t);
                px >= frame.x && px <= frame.x + frame.w
            })
            .collect();
        let yt: Vec<f64> = yt
            .into_iter()
            .filter(|t| {
                let py = ys.at(*t);
                py >= frame.y && py <= frame.y + frame.h
            })
            .collect();
        (xs, ys, xt, yt)
    } else {
        (
            ValueScale::new(x_mode, (x_low, x_high), (frame.x, frame.x + frame.w)),
            ValueScale::new(y_mode, (y_low, y_high), (frame.y + frame.h, frame.y)),
            x_ticks,
            y_ticks,
        )
    };
    let largest = shown()
        .filter_map(|cloud| cloud.sizes.as_ref())
        .flatten()
        .copied()
        .fold(0.0, f64::max);
    let mut geometry = Geometry {
        value_scale: Some(ys),
        ticks: y_ticks.iter().map(|tick| (ys.at(*tick), *tick)).collect(),
        labels: x_ticks
            .iter()
            .map(|tick| (xs.at(*tick), format(*tick).into()))
            .collect(),
        ..Geometry::default()
    };
    let mut key = 0;
    for (color, cloud) in clouds.iter().enumerate() {
        let skip = hidden.contains(&cloud.name);
        for (ix, (x, y)) in cloud.points.iter().enumerate() {
            let radius = match &cloud.sizes {
                Some(sizes) if largest > 0.0 => widest * (sizes[ix] / largest).sqrt() as f32,
                _ => dot,
            };
            if !skip {
                let center = (xs.at(*x), ys.at(*y));
                if xy_view.is_none() || frame.contains(center) {
                    geometry.dots.push(Dot {
                        ink: Ink::Series(color),
                        center,
                        radius: radius.max(dot / 2.0),
                        symbol: cloud.symbol,
                        key,
                    });
                }
            }
            key += 1;
        }
    }
    if voronoi && geometry.dots.len() >= 2 {
        let sites: Vec<(f32, f32)> = geometry.dots.iter().map(|dot| dot.center).collect();
        for cell in voronoi_cells(&sites, frame) {
            for pair in cell.windows(2) {
                geometry.strokes.push((Ink::Rule, pair[0], pair[1]));
            }
            if let (Some(&first), Some(&last)) = (cell.first(), cell.last())
                && cell.len() > 2
            {
                geometry.strokes.push((Ink::Rule, last, first));
            }
        }
    }
    geometry
}

/// Clipped Voronoi polygons inside `frame` using Sutherland-Hodgman bisector clipping.
pub(crate) fn voronoi_cells(sites: &[(f32, f32)], frame: Rect) -> Vec<Vec<(f32, f32)>> {
    if sites.is_empty() || frame.w <= 0.0 || frame.h <= 0.0 {
        return Vec::new();
    }
    let rect_poly = vec![
        (frame.x, frame.y),
        (frame.x + frame.w, frame.y),
        (frame.x + frame.w, frame.y + frame.h),
        (frame.x, frame.y + frame.h),
    ];
    sites
        .iter()
        .enumerate()
        .map(|(i, &(ax, ay))| {
            let mut poly = rect_poly.clone();
            for (j, &(bx, by)) in sites.iter().enumerate() {
                if i == j {
                    continue;
                }
                let (dx, dy) = (bx - ax, by - ay);
                if dx * dx + dy * dy < 1e-6 {
                    continue;
                }
                let (mx, my) = ((ax + bx) * 0.5, (ay + by) * 0.5);
                poly = clip_half_plane(&poly, mx, my, dx, dy);
                if poly.is_empty() {
                    break;
                }
            }
            poly
        })
        .collect()
}

fn clip_half_plane(poly: &[(f32, f32)], mx: f32, my: f32, dx: f32, dy: f32) -> Vec<(f32, f32)> {
    if poly.is_empty() {
        return Vec::new();
    }
    let side = |p: (f32, f32)| (p.0 - mx) * dx + (p.1 - my) * dy;
    let mut out = Vec::with_capacity(poly.len() + 1);
    let mut prev = *poly.last().expect("non-empty poly");
    let mut prev_d = side(prev);
    for &curr in poly {
        let curr_d = side(curr);
        let prev_in = prev_d <= 1e-5;
        let curr_in = curr_d <= 1e-5;
        if curr_in != prev_in {
            let denom = prev_d - curr_d;
            let t = if denom.abs() > 1e-9 {
                (prev_d / denom).clamp(0.0, 1.0)
            } else {
                0.5
            };
            out.push((
                prev.0 + (curr.0 - prev.0) * t,
                prev.1 + (curr.1 - prev.1) * t,
            ));
        }
        if curr_in {
            out.push(curr);
        }
        prev = curr;
        prev_d = curr_d;
    }
    out
}

#[cfg(test)]
mod tests;
