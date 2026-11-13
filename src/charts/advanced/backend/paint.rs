use gpui::{
    App, BorderStyle, Bounds, ContentMask, Corners, Edges, Hsla, Path, PathBuilder, Pixels, Rgba,
    TextAlign, TextRun, Window, font, point, quad,
};

use crate::charts::advanced::geom::{ScreenPoint, ScreenRect};
use crate::charts::advanced::render::{
    LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCommand, TextStyle,
};

use super::frame::PlotFrame;

pub(crate) fn paint_frame(frame: &PlotFrame, window: &mut Window, cx: &mut App) {
    let mut clip_stack: Vec<ContentMask<Pixels>> = Vec::new();
    for command in frame.render.commands() {
        match command {
            RenderCommand::ClipRect(rect) => {
                clip_stack.push(ContentMask {
                    bounds: to_bounds(*rect),
                });
            }
            RenderCommand::ClipEnd => {
                clip_stack.pop();
            }
            RenderCommand::LineSegments { segments, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_lines(window, segments, *style);
                });
            }
            RenderCommand::Points { points, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_points(window, points, *style);
                });
            }
            RenderCommand::Rect { rect, style } => {
                with_clip(window, &clip_stack, |window| {
                    paint_rect(window, *rect, *style);
                });
            }
            RenderCommand::Text {
                position,
                text,
                style,
            } => {
                with_clip(window, &clip_stack, |window| {
                    paint_text(window, cx, *position, text, style);
                });
            }
        }
    }
}

fn paint_lines(window: &mut Window, segments: &[LineSegment], style: LineStyle) {
    if segments.is_empty() {
        return;
    }
    window.paint_path(build_line_path(segments, style.width), style.color);
}

pub(super) fn build_line_path(segments: &[LineSegment], width: f32) -> Path<Pixels> {
    let radius = width.max(0.5) * 0.5;
    let origin = segments
        .first()
        .map_or(point(Pixels::from(0.), Pixels::from(0.)), |segment| {
            point(Pixels::from(segment.start.x), Pixels::from(segment.start.y))
        });
    let mut path = Path::new(origin);
    path.vertices.reserve(segments.len().saturating_mul(9));
    let vertex = |p: ScreenPoint| point(Pixels::from(p.x), Pixels::from(p.y));
    let offset = |p: ScreenPoint, normal: ScreenPoint, side: f32| {
        ScreenPoint::new(p.x + normal.x * side, p.y + normal.y * side)
    };
    let uv = (point(0., 1.), point(0., 1.), point(0., 1.));
    let mut previous: Option<(ScreenPoint, ScreenPoint)> = None;
    for segment in segments {
        let dx = segment.end.x - segment.start.x;
        let dy = segment.end.y - segment.start.y;
        let length = dx.hypot(dy);
        if length == 0. || !length.is_finite() {
            continue;
        }
        let normal = ScreenPoint::new(-dy * radius / length, dx * radius / length);
        let a = vertex(offset(segment.start, normal, 1.));
        let b = vertex(offset(segment.start, normal, -1.));
        let c = vertex(offset(segment.end, normal, 1.));
        let d = vertex(offset(segment.end, normal, -1.));
        // Independent quads retain every segment even at subpixel lengths and near-180-degree folds. Generic stroke tessellation may merge or fold these segments, opening cracks in dense oscillations.
        path.push_triangle((a, b, c), uv);
        path.push_triangle((b, d, c), uv);
        if let Some((end, previous_normal)) = previous
            && end == segment.start
        {
            let turn = previous_normal.x * normal.y - previous_normal.y * normal.x;
            if turn != 0. {
                let side = if turn > 0. { -1. } else { 1. };
                // A bevel fills only the outside join wedge; it cannot extend a sharp data peak beyond half the configured stroke width.
                path.push_triangle(
                    (
                        vertex(segment.start),
                        vertex(offset(segment.start, previous_normal, side)),
                        vertex(offset(segment.start, normal, side)),
                    ),
                    uv,
                );
            }
        }
        previous = Some((segment.end, normal));
    }
    path
}

fn paint_points(window: &mut Window, points: &[ScreenPoint], style: MarkerStyle) {
    if points.is_empty() {
        return;
    }

    let size = style.size.max(2.0);
    match style.shape {
        MarkerShape::Circle => {
            let radius = size * 0.5;
            for pt in points {
                let bounds = Bounds::from_corners(
                    point(Pixels::from(pt.x - radius), Pixels::from(pt.y - radius)),
                    point(Pixels::from(pt.x + radius), Pixels::from(pt.y + radius)),
                );
                window.paint_quad(quad(
                    bounds,
                    Corners::all(Pixels::from(radius)),
                    style.color,
                    Edges::all(Pixels::from(0.0)),
                    style.color,
                    BorderStyle::default(),
                ));
            }
        }
        MarkerShape::Square => {
            let half = size * 0.5;
            for pt in points {
                let bounds = Bounds::from_corners(
                    point(Pixels::from(pt.x - half), Pixels::from(pt.y - half)),
                    point(Pixels::from(pt.x + half), Pixels::from(pt.y + half)),
                );
                window.paint_quad(quad(
                    bounds,
                    Corners::all(Pixels::from(0.0)),
                    style.color,
                    Edges::all(Pixels::from(0.0)),
                    style.color,
                    BorderStyle::default(),
                ));
            }
        }
        MarkerShape::Cross => {
            let half = size * 0.5;
            let mut builder = PathBuilder::stroke(Pixels::from(1.0));
            for pt in points {
                let h_start = point(Pixels::from(pt.x - half), Pixels::from(pt.y));
                let h_end = point(Pixels::from(pt.x + half), Pixels::from(pt.y));
                let v_start = point(Pixels::from(pt.x), Pixels::from(pt.y - half));
                let v_end = point(Pixels::from(pt.x), Pixels::from(pt.y + half));
                builder.move_to(h_start);
                builder.line_to(h_end);
                builder.move_to(v_start);
                builder.line_to(v_end);
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, style.color);
            }
        }
    }
}

fn paint_rect(window: &mut Window, rect: ScreenRect, style: RectStyle) {
    let bounds = to_bounds(rect);
    let quad = quad(
        bounds,
        Corners::all(Pixels::from(0.0)),
        style.fill,
        Edges::all(Pixels::from(style.stroke_width)),
        style.stroke,
        BorderStyle::default(),
    );
    window.paint_quad(quad);
}

fn paint_text(
    window: &mut Window,
    cx: &mut App,
    position: ScreenPoint,
    text: &str,
    style: &TextStyle,
) {
    if text.is_empty() {
        return;
    }
    let font_size = Pixels::from(style.size);
    let run = TextRun {
        len: text.len(),
        font: font(if cx.has_global::<crate::theme::Theme>() {
            crate::theme::ActiveTheme::theme(cx).font_family.clone()
        } else {
            ".SystemUIFont".into()
        }),
        color: to_hsla(style.color),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = window
        .text_system()
        .shape_line(text.to_string().into(), font_size, &[run], None);
    let line_height = shaped.ascent + shaped.descent;
    let origin = point(Pixels::from(position.x), Pixels::from(position.y));
    let _ = shaped.paint(origin, line_height, TextAlign::Left, None, window, cx);
}

pub(crate) fn to_hsla(color: Rgba) -> Hsla {
    Hsla::from(color)
}

fn to_bounds(rect: ScreenRect) -> Bounds<Pixels> {
    Bounds::from_corners(
        point(Pixels::from(rect.min.x), Pixels::from(rect.min.y)),
        point(Pixels::from(rect.max.x), Pixels::from(rect.max.y)),
    )
}

fn with_clip(window: &mut Window, stack: &[ContentMask<Pixels>], f: impl FnOnce(&mut Window)) {
    if let Some(mask) = stack.last() {
        window.with_content_mask(Some(*mask), f);
    } else {
        f(window);
    }
}
