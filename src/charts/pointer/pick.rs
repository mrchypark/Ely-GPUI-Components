use super::*;

pub(crate) fn pick_at(
    pick: Pick,
    place: (f32, f32),
    frame: Rect,
    horizontal: bool,
    span: (usize, usize),
    dots: &[Dot],
    reach: f32,
) -> Option<usize> {
    if !frame.contains(place) {
        return None;
    }
    let count = span.1 - span.0 + 1;
    let (along, start, length) = if horizontal {
        (place.1, frame.y, frame.h)
    } else {
        (place.0, frame.x, frame.w)
    };
    match pick {
        Pick::Band => Band {
            count,
            range: (start, start + length),
            padding: 0.0,
        }
        .index(along)
        .map(|ix| ix + span.0),
        Pick::Point if count < 2 => Some(span.0),
        Pick::Point => {
            let ix = ((along - start) / (length / (count - 1) as f32)).round() as usize;
            Some(ix.min(count - 1) + span.0)
        }
        Pick::Dot => dots
            .iter()
            .map(|dot| (dot, (dot.center.0 - place.0).hypot(dot.center.1 - place.1)))
            .filter(|(dot, far)| *far <= dot.radius.max(reach))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(dot, _)| dot.key),
    }
}

/// What the picked index lights: its band, its points on each line, or its dot.
pub(crate) enum Focus {
    Band(Rect),
    Line(f32, Vec<(Ink, (f32, f32))>),
    Dot(Dot),
}

/// What the picked index lights: its band, its points on each line, or its dot.
pub(crate) fn focus(
    plot: &Plot,
    geometry: &Geometry,
    hover: Option<usize>,
    (rect, span): (Rect, (usize, usize)),
) -> Option<Focus> {
    let ix = hover?;
    match plot.pick {
        Pick::Dot => geometry
            .dots
            .iter()
            .find(|dot| dot.key == ix)
            .map(|dot| Focus::Dot(*dot)),
        Pick::Band => {
            let range = if plot.horizontal {
                (rect.y, rect.y + rect.h)
            } else {
                (rect.x, rect.x + rect.w)
            };
            let lanes = Band {
                count: span.1 - span.0 + 1,
                range,
                padding: 0.0,
            };
            let (start, width) = lanes.slot(ix - span.0);
            Some(Focus::Band(if plot.horizontal {
                Rect {
                    x: rect.x,
                    y: start,
                    w: rect.w,
                    h: width,
                }
            } else {
                Rect {
                    x: start,
                    y: rect.y,
                    w: width,
                    h: rect.h,
                }
            }))
        }
        Pick::Point => {
            let x = geometry.labels.get(ix - span.0)?.0;
            let points = geometry
                .lines
                .iter()
                .filter_map(|line| Some((line.ink, *line.points.get(ix - span.0)?)))
                .collect();
            Some(Focus::Line(x, points))
        }
    }
}

/// The plot's canvas: the band under the pointer, the marks, dashed rules and note lines, the crosshair or the lit dot, and a brush.
pub(crate) fn drawing(
    geometry: Rc<Geometry>,
    pen: Pen,
    rect: Rect,
    (focus, brush): (Option<Focus>, Option<(f32, f32)>),
    (rules, notes): (Vec<f32>, Vec<f32>),
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            let colors = &pen.colors;
            if let Some(Focus::Band(band)) = &focus {
                window.paint_quad(fill(place(origin, *band), colors.hover));
            }
            paint(&geometry, rect, &pen, origin, window);
            let dash = [pen.stroke * 2.0, pen.stroke * 2.0];
            for level in &rules {
                let mut rule = PathBuilder::stroke(pen.hairline).dash_array(&dash);
                let (from, to) = if pen.horizontal {
                    ((*level, rect.y), (*level, rect.y + rect.h))
                } else {
                    ((rect.x, *level), (rect.x + rect.w, *level))
                };
                rule.move_to(at(origin, from));
                rule.line_to(at(origin, to));
                finish(rule, colors.fg_muted, window);
            }
            let upright = |x: f32| {
                Bounds::new(
                    at(origin, (x, rect.y)),
                    size(pen.hairline, Pixels::from(rect.h)),
                )
            };
            for x in &notes {
                window.paint_quad(fill(upright(*x), colors.fg_subtle.opacity(0.6)));
            }
            match &focus {
                Some(Focus::Line(x, points)) => {
                    window.paint_quad(fill(upright(*x), colors.border_strong));
                    for (ink, point) in points {
                        let ends = (colors.bg, ink.color(colors));
                        ring(
                            at(origin, *point),
                            pen.stroke * 2.0,
                            ends,
                            pen.stroke,
                            window,
                        );
                    }
                }
                Some(Focus::Dot(dot)) => {
                    let ends = (dot.ink.color(colors), colors.bg);
                    ring(
                        at(origin, dot.center),
                        Pixels::from(dot.radius),
                        ends,
                        pen.stroke,
                        window,
                    );
                }
                _ => {}
            }
            if let Some((a, b)) = brush {
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
    .inset_0()
}

/// A place in window pixels as the plot's own.
pub(super) fn local(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

pub(super) fn pick_hover(
    state: &Entity<State>,
    place: (f32, f32),
    (pick, horizontal): (Pick, bool),
    (rect, span, reach): (Rect, (usize, usize), f32),
    geometry: &Geometry,
    cx: &mut App,
) {
    state.update(cx, |state, cx| {
        let next = pick_at(pick, place, rect, horizontal, span, &geometry.dots, reach);
        if state.hover != next || (next.is_some() && pick != Pick::Dot) {
            state.hover = next;
            state.pointer = place;
            cx.notify();
        }
    });
}

type NormView = ((f64, f64), (f64, f64));

pub(super) fn zoom_xy_view(
    current: Option<NormView>,
    (tx, ty): (f64, f64),
    factor: f64,
) -> Option<NormView> {
    let ((ux0, ux1), (uy0, uy1)) = current.unwrap_or(((0.0, 1.0), (0.0, 1.0)));
    let cx = ux0 + (ux1 - ux0) * tx.clamp(0.0, 1.0);
    let cy = uy0 + (uy1 - uy0) * ty.clamp(0.0, 1.0);
    let nx0 = cx + (ux0 - cx) * factor;
    let nx1 = cx + (ux1 - cx) * factor;
    let ny0 = cy + (uy0 - cy) * factor;
    let ny1 = cy + (uy1 - cy) * factor;
    if (nx1 - nx0).abs() < 1e-4 || (ny1 - ny0).abs() < 1e-4 {
        return current;
    }
    if (nx1 - nx0) > 10.0 || (ny1 - ny0) > 10.0 {
        return None;
    }
    Some(((nx0, nx1), (ny0, ny1)))
}

pub(super) fn pan_xy_view(
    current: Option<NormView>,
    (dx_px, dy_px): (f32, f32),
    rect: Rect,
) -> Option<NormView> {
    if rect.w <= 1.0 || rect.h <= 1.0 {
        return current;
    }
    let ((ux0, ux1), (uy0, uy1)) = current.unwrap_or(((0.0, 1.0), (0.0, 1.0)));
    let sx = (ux1 - ux0) * f64::from(-dx_px / rect.w);
    let sy = (uy1 - uy0) * f64::from(dy_px / rect.h);
    Some(((ux0 + sx, ux1 + sx), (uy0 + sy, uy1 + sy)))
}
