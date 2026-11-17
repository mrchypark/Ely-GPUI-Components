use gpui::{
    Bounds, Entity, Hsla, IntoElement, PathBuilder, Pixels, Point, Styled, Window, canvas, fill,
    point, size,
};

use super::{
    geometry::{Geometry, Ink, Rect},
    series::{Curve, Symbol, tangents},
};
use crate::theme::Palette;

/// A place in a box's own pixels, moved to where the box sits.
pub(crate) fn at(origin: Point<Pixels>, (x, y): (f32, f32)) -> Point<Pixels> {
    origin + point(Pixels::from(x), Pixels::from(y))
}

/// A rectangle in a box's own pixels, moved to where the box sits.
pub(crate) fn place(origin: Point<Pixels>, rect: Rect) -> Bounds<Pixels> {
    Bounds::new(
        at(origin, (rect.x, rect.y)),
        size(Pixels::from(rect.w), Pixels::from(rect.h)),
    )
}

/// A canvas over its parent that keeps the parent's bounds in `state` and draws once more when they change.
pub(crate) fn measure<T: 'static>(
    state: Entity<T>,
    slot: fn(&mut T) -> &mut Bounds<Pixels>,
) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            let changed = state.update(cx, |state, cx| {
                let held = slot(state);
                let changed = *held != bounds;
                if changed {
                    *held = bounds;
                    cx.notify();
                }
                changed
            });
            if changed {
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The color series `ix` takes from the chart palette.
pub(crate) fn tint(colors: &Palette, ix: usize) -> Hsla {
    colors.chart[ix % colors.chart.len()]
}

impl Ink {
    pub(crate) fn color(self, colors: &Palette) -> Hsla {
        match self {
            Ink::Series(ix) => tint(colors, ix),
            Ink::Rise => colors.success,
            Ink::Fall => colors.danger,
            Ink::Rule => colors.fg_subtle,
            Ink::Strong => colors.fg,
        }
    }
}

/// How a geometry paints: its colors and measures in pixels, and whether lines curve.
#[derive(Clone)]
pub(crate) struct Pen {
    pub colors: Palette,
    pub stroke: Pixels,
    pub hairline: Pixels,
    pub corner: Pixels,
    pub curve: Curve,
    pub horizontal: bool,
}

/// Follows `points`, straight or through curves that never overshoot.
pub(crate) fn trace(
    path: &mut PathBuilder,
    points: &[(f32, f32)],
    smooth: bool,
    origin: Point<Pixels>,
) {
    trace_curve(
        path,
        points,
        if smooth {
            Curve::Monotone
        } else {
            Curve::Linear
        },
        origin,
    );
}

pub(crate) fn trace_curve(
    path: &mut PathBuilder,
    points: &[(f32, f32)],
    curve: Curve,
    origin: Point<Pixels>,
) {
    let Some(first) = points.first() else {
        return;
    };
    if matches!(curve, Curve::Basis) && points.len() >= 3 {
        let expanded = sample_basis(points);
        path.move_to(at(origin, expanded[0]));
        for &pt in &expanded[1..] {
            path.line_to(at(origin, pt));
        }
        return;
    }
    path.move_to(at(origin, *first));
    if points.len() < 2 {
        return;
    }
    match curve {
        Curve::Linear => {
            for next in &points[1..] {
                path.line_to(at(origin, *next));
            }
        }
        Curve::Step => {
            for pair in points.windows(2) {
                let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
                let xm = (x0 + x1) * 0.5;
                path.line_to(at(origin, (xm, y0)));
                path.line_to(at(origin, (xm, y1)));
                path.line_to(at(origin, (x1, y1)));
            }
        }
        Curve::StepBefore => {
            for pair in points.windows(2) {
                let ((x0, _), (x1, y1)) = (pair[0], pair[1]);
                path.line_to(at(origin, (x0, y1)));
                path.line_to(at(origin, (x1, y1)));
            }
        }
        Curve::StepAfter => {
            for pair in points.windows(2) {
                let ((_, y0), (x1, y1)) = (pair[0], pair[1]);
                path.line_to(at(origin, (x1, y0)));
                path.line_to(at(origin, (x1, y1)));
            }
        }
        Curve::Monotone if points.len() >= 3 => {
            let slopes = tangents(points);
            for ix in 0..points.len() - 1 {
                let ((x0, y0), (x1, y1)) = (points[ix], points[ix + 1]);
                let third = (x1 - x0) / 3.0;
                path.cubic_bezier_to(
                    at(origin, (x1, y1)),
                    at(origin, (x0 + third, y0 + slopes[ix] * third)),
                    at(origin, (x1 - third, y1 - slopes[ix + 1] * third)),
                );
            }
        }
        Curve::Cardinal if points.len() >= 3 => {
            let n = points.len();
            for ix in 0..n - 1 {
                let p0 = points[ix.saturating_sub(1)];
                let p1 = points[ix];
                let p2 = points[ix + 1];
                let p3 = points[(ix + 2).min(n - 1)];
                let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
                let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
                path.cubic_bezier_to(at(origin, p2), at(origin, c1), at(origin, c2));
            }
        }
        _ => {
            for next in &points[1..] {
                path.line_to(at(origin, *next));
            }
        }
    }
}

pub(crate) fn sample_basis(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let get = |i: isize| -> (f32, f32) { points[i.clamp(0, (n - 1) as isize) as usize] };
    let mut out = Vec::with_capacity((n + 1) * 8);
    for i in -1..(n as isize - 1) {
        let (p0, p1, p2, p3) = (get(i), get(i + 1), get(i + 2), get(i + 3));
        for step in 0..8 {
            let t = step as f32 / 8.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let b0 = (1.0 - t).powi(3) / 6.0;
            let b1 = (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0;
            let b2 = (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0;
            let b3 = t3 / 6.0;
            out.push((
                b0 * p0.0 + b1 * p1.0 + b2 * p2.0 + b3 * p3.0,
                b0 * p0.1 + b1 * p1.1 + b2 * p2.1 + b3 * p3.1,
            ));
        }
    }
    if let Some(&last) = points.last() {
        out.push(last);
    }
    out
}

pub(crate) fn symbol_vertices(symbol: Symbol, (cx, cy): (f32, f32), r: f32) -> Vec<(f32, f32)> {
    match symbol {
        Symbol::Circle => Vec::new(),
        Symbol::Square => vec![
            (cx - r, cy - r),
            (cx + r, cy - r),
            (cx + r, cy + r),
            (cx - r, cy + r),
        ],
        Symbol::Diamond => vec![
            (cx, cy - r * 1.25),
            (cx + r * 1.1, cy),
            (cx, cy + r * 1.25),
            (cx - r * 1.1, cy),
        ],
        Symbol::Triangle => {
            let h = r * 1.2;
            vec![
                (cx, cy - h),
                (cx + h * 0.95, cy + h * 0.75),
                (cx - h * 0.95, cy + h * 0.75),
            ]
        }
        Symbol::Cross => {
            let w = r * 0.36;
            vec![
                (cx - w, cy - r),
                (cx + w, cy - r),
                (cx + w, cy - w),
                (cx + r, cy - w),
                (cx + r, cy + w),
                (cx + w, cy + w),
                (cx + w, cy + r),
                (cx - w, cy + r),
                (cx - w, cy + w),
                (cx - r, cy + w),
                (cx - r, cy - w),
                (cx - w, cy - w),
            ]
        }
        Symbol::Star => (0..10)
            .map(|i| {
                let rad = if i % 2 == 0 { r * 1.3 } else { r * 0.52 };
                let a = std::f32::consts::PI * i as f32 / 5.0 - std::f32::consts::FRAC_PI_2;
                (cx + rad * a.cos(), cy + rad * a.sin())
            })
            .collect(),
    }
}

/// Paints a path, or logs why it would not build.
pub(crate) fn finish(path: PathBuilder, color: Hsla, window: &mut Window) {
    match path.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => log::error!("chart: a path failed to build: {error:#}"),
    }
}

/// A round dot filled with one color and edged with another.
pub(crate) fn ring(
    center: Point<Pixels>,
    radius: Pixels,
    (inside, edge): (Hsla, Hsla),
    width: Pixels,
    window: &mut Window,
) {
    let bounds = Bounds::new(
        center - point(radius, radius),
        size(radius * 2.0, radius * 2.0),
    );
    window.paint_quad(
        fill(bounds, inside)
            .corner_radii(radius)
            .border_widths(width)
            .border_color(edge),
    );
}

/// Paints a geometry laid out in its box's own pixels: gridlines, areas, lines, bars, shapes, strokes and dots.
pub(crate) fn paint(
    geometry: &Geometry,
    frame: Rect,
    pen: &Pen,
    origin: Point<Pixels>,
    window: &mut Window,
) {
    let colors = &pen.colors;
    let grid = colors.border.opacity(0.6);
    for (tick, _) in &geometry.ticks {
        let line = if pen.horizontal {
            Bounds::new(
                at(origin, (*tick, frame.y)),
                size(pen.hairline, Pixels::from(frame.h)),
            )
        } else {
            Bounds::new(
                at(origin, (frame.x, *tick)),
                size(Pixels::from(frame.w), pen.hairline),
            )
        };
        window.paint_quad(fill(line, grid));
    }
    for line in &geometry.lines {
        let Some(floor) = &line.floor else {
            continue;
        };
        let mut area = PathBuilder::fill();
        trace_curve(&mut area, &line.points, pen.curve, origin);
        floor
            .iter()
            .rev()
            .for_each(|place| area.line_to(at(origin, *place)));
        area.close();
        finish(area, line.ink.color(colors).opacity(0.14), window);
    }
    for line in geometry.lines.iter().filter(|line| line.points.len() > 1) {
        let mut path = PathBuilder::stroke(pen.stroke);
        trace_curve(&mut path, &line.points, pen.curve, origin);
        finish(path, line.ink.color(colors), window);
    }
    let seam = f32::from(pen.hairline);
    for (ink, rect) in &geometry.bars {
        let rect = if pen.horizontal {
            Rect {
                w: (rect.w - seam).max(0.0),
                ..*rect
            }
        } else {
            Rect {
                y: rect.y + seam,
                h: (rect.h - seam).max(0.0),
                ..*rect
            }
        };
        let corner = Pixels::from(f32::from(pen.corner).min(rect.w.min(rect.h) / 2.0));
        window.paint_quad(fill(place(origin, rect), ink.color(colors)).corner_radii(corner));
    }
    for (ink, outline) in geometry
        .shapes
        .iter()
        .filter(|(_, outline)| outline.len() > 2)
    {
        let mut shape = PathBuilder::fill();
        let mut edge = PathBuilder::stroke(pen.hairline);
        trace(&mut shape, outline, false, origin);
        trace(&mut edge, outline, false, origin);
        shape.close();
        edge.close();
        finish(shape, ink.color(colors).opacity(0.3), window);
        finish(edge, ink.color(colors), window);
    }
    for (ink, from, to) in &geometry.strokes {
        let width = if *ink == Ink::Rule {
            pen.hairline
        } else {
            pen.stroke
        };
        let mut path = PathBuilder::stroke(width);
        path.move_to(at(origin, *from));
        path.line_to(at(origin, *to));
        finish(path, ink.color(colors), window);
    }
    for dot in &geometry.dots {
        let color = dot.ink.color(colors);
        let poly = symbol_vertices(dot.symbol, dot.center, dot.radius);
        if poly.is_empty() {
            ring(
                at(origin, dot.center),
                Pixels::from(dot.radius),
                (color.opacity(0.6), color),
                pen.hairline,
                window,
            );
        } else {
            let mut fill_p = PathBuilder::fill();
            let mut edge_p = PathBuilder::stroke(pen.hairline);
            trace(&mut fill_p, &poly, false, origin);
            trace(&mut edge_p, &poly, false, origin);
            fill_p.close();
            edge_p.close();
            finish(fill_p, color.opacity(0.6), window);
            finish(edge_p, color, window);
        }
    }
}
