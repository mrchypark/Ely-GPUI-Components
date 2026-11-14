use crate::charts::advanced::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::render::{
    LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCommand, RenderList,
    TextStyle,
};
use crate::charts::advanced::transform::Transform;

use super::super::constants::*;
use super::super::geometry::clamp_point;
use super::super::state::PlotUiState;
use super::super::text::GpuiTextMeasurer;
use super::pins::marker_style_and_size;
use super::with_alpha;

pub(super) fn build_hover(
    render: &mut RenderList,
    plot: &Plot,
    state: &PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let Some(cursor) = state.hover else { return };
    if cursor.x < plot_rect.min.x
        || cursor.x > plot_rect.max.x
        || cursor.y < plot_rect.min.y
        || cursor.y > plot_rect.max.y
    {
        return;
    }

    if let Some(target) = state.hover_target {
        let Some(series) = plot
            .series()
            .iter()
            .find(|series| series.id() == target.pin.series_id)
        else {
            return;
        };
        let Some(point) = series.with_store(|store| store.pinned_point(target.pin.point_index))
        else {
            return;
        };
        let screen = target.screen;
        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            return;
        }

        if target.is_pinned {
            let (_, base_size) = marker_style_and_size(series);
            let ring_outer = base_size + PIN_RING_OUTER_PAD;
            let ring_inner = base_size + PIN_RING_INNER_PAD;
            render.push(RenderCommand::Points {
                points: vec![screen],
                style: MarkerStyle {
                    color: PIN_UNPIN_HIGHLIGHT,
                    size: ring_outer,
                    shape: MarkerShape::Circle,
                },
            });
            render.push(RenderCommand::Points {
                points: vec![screen],
                style: MarkerStyle {
                    color: theme.background,
                    size: ring_inner,
                    shape: MarkerShape::Circle,
                },
            });
            return;
        }

        let (marker_style, base_size) = marker_style_and_size(series);
        let ring_outer = base_size + PIN_RING_OUTER_PAD;
        let ring_inner = base_size + PIN_RING_INNER_PAD;
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.axis,
                size: ring_outer,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: MarkerStyle {
                color: theme.background,
                size: ring_inner,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![screen],
            style: marker_style,
        });

        let x_text = plot.x_axis().format_value(point.x);
        let y_text = plot.y_axis().format_value(point.y);
        let label = format!("{}\nx: {x_text}\ny: {y_text}", series.name());
        let size = measurer.measure_multiline(&label, 12.0);
        let mut origin = ScreenPoint::new(screen.x + 12.0, screen.y + 12.0);
        if origin.x + size.0 > plot_rect.max.x {
            origin.x = screen.x - size.0 - 12.0;
        }
        if origin.y + size.1 > plot_rect.max.y {
            origin.y = screen.y - size.1 - 12.0;
        }
        origin = clamp_point(origin, plot_rect, size);

        render.push(RenderCommand::Rect {
            rect: ScreenRect::new(
                origin,
                ScreenPoint::new(origin.x + size.0, origin.y + size.1),
            ),
            style: RectStyle {
                fill: theme.pin_bg,
                stroke: theme.pin_border,
                stroke_width: 1.0,
            },
        });

        for (index, line) in label.lines().enumerate() {
            let line_y = origin.y + index as f32 * 14.0 + 2.0;
            render.push(RenderCommand::Text {
                position: ScreenPoint::new(origin.x + 4.0, line_y),
                text: line.to_string(),
                style: TextStyle {
                    color: theme.axis,
                    size: 12.0,
                },
            });
        }
        return;
    }

    let Some(data) = transform.screen_to_data(cursor) else {
        return;
    };
    let x_text = plot.x_axis().format_value(data.x);
    let y_text = plot.y_axis().format_value(data.y);
    let label = format!("x: {x_text}\ny: {y_text}");

    let size = measurer.measure_multiline(&label, 12.0);
    let mut origin = ScreenPoint::new(cursor.x + 12.0, cursor.y + 12.0);
    if origin.x + size.0 > plot_rect.max.x {
        origin.x = cursor.x - size.0 - 12.0;
    }
    if origin.y + size.1 > plot_rect.max.y {
        origin.y = cursor.y - size.1 - 12.0;
    }
    origin = clamp_point(origin, plot_rect, size);

    render.push(RenderCommand::Rect {
        rect: ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        ),
        style: RectStyle {
            fill: theme.hover_bg,
            stroke: theme.hover_border,
            stroke_width: 1.0,
        },
    });

    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * 14.0 + 2.0;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(origin.x + 4.0, line_y),
            text: line.to_string(),
            style: TextStyle {
                color: theme.axis,
                size: 12.0,
            },
        });
    }
}

pub(super) fn build_linked_cursor(
    render: &mut RenderList,
    plot: &Plot,
    state: &PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let Some(x) = state.linked_cursor_x.or(state.selected_x) else {
        return;
    };
    if state.hover.is_some() {
        return;
    }

    let Some(screen_x) = transform
        .data_to_screen(DataPoint::new(x, transform.viewport().y.min))
        .map(|point| point.x)
    else {
        return;
    };

    if screen_x < plot_rect.min.x || screen_x > plot_rect.max.x {
        return;
    }

    let theme = plot.theme();

    render.push(RenderCommand::ClipRect(plot_rect));
    render.push(RenderCommand::LineSegments {
        segments: vec![LineSegment::new(
            ScreenPoint::new(screen_x, plot_rect.min.y),
            ScreenPoint::new(screen_x, plot_rect.max.y),
        )],
        style: LineStyle {
            color: with_alpha(theme.axis, LINK_CURSOR_ALPHA),
            width: LINK_CURSOR_WIDTH,
        },
    });
    render.push(RenderCommand::ClipEnd);

    let mut lines = Vec::new();
    lines.push(format!("x: {}", plot.x_axis().format_value(x)));

    let mut hidden = 0usize;
    for series in plot.series() {
        if !series.is_visible() {
            continue;
        }
        let point = series.with_store(|store| {
            let data = store.data();
            data.nearest_index_by_x(x)
                .and_then(|index| data.point(index))
        });
        if let Some(point) = point {
            if lines.len() <= 6 {
                lines.push(format!(
                    "{}: {}",
                    series.name(),
                    plot.y_axis().format_value(point.y)
                ));
            } else {
                hidden += 1;
            }
        }
    }
    if hidden > 0 {
        lines.push(format!("+{hidden} more"));
    }
    if lines.is_empty() {
        return;
    }

    let label = lines.join("\n");
    let font_size = 12.0;
    let size = measurer.measure_multiline(&label, font_size);
    let mut origin = ScreenPoint::new(screen_x + 10.0, plot_rect.min.y + 10.0);
    if origin.x + size.0 > plot_rect.max.x {
        origin.x = screen_x - size.0 - 10.0;
    }
    origin = clamp_point(origin, plot_rect, size);
    origin = avoid_legend_overlap(
        origin,
        size,
        state.legend_layout.as_ref().map(|legend| legend.rect),
        plot_rect,
    );

    render.push(RenderCommand::Rect {
        rect: ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        ),
        style: RectStyle {
            fill: with_alpha(theme.hover_bg, 0.9),
            stroke: with_alpha(theme.hover_border, 0.9),
            stroke_width: 1.0,
        },
    });

    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * 14.0 + 2.0;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(origin.x + 4.0, line_y),
            text: line.to_string(),
            style: TextStyle {
                color: theme.axis,
                size: font_size,
            },
        });
    }
}

pub(super) fn avoid_legend_overlap(
    mut origin: ScreenPoint,
    size: (f32, f32),
    legend_rect: Option<ScreenRect>,
    plot_rect: ScreenRect,
) -> ScreenPoint {
    if let Some(legend) = legend_rect {
        let overlaps_x = origin.x < legend.max.x && origin.x + size.0 > legend.min.x;
        let overlaps_y = origin.y < legend.max.y && origin.y + size.1 > legend.min.y;
        let below_y = legend.max.y + 6.0;
        if overlaps_x && overlaps_y && below_y + size.1 <= plot_rect.max.y {
            origin.y = below_y;
            origin = clamp_point(origin, plot_rect, size);
        }
    }
    origin
}

pub(super) fn build_linked_brush(
    render: &mut RenderList,
    plot: &Plot,
    state: &PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let Some(range) = state.linked_brush_x else {
        return;
    };
    if state.selection_rect.is_some() {
        return;
    }

    let Some(start_x) = transform
        .data_to_screen(DataPoint::new(range.min, transform.viewport().y.min))
        .map(|point| point.x)
    else {
        return;
    };
    let Some(end_x) = transform
        .data_to_screen(DataPoint::new(range.max, transform.viewport().y.min))
        .map(|point| point.x)
    else {
        return;
    };

    let min_x = start_x.min(end_x).clamp(plot_rect.min.x, plot_rect.max.x);
    let max_x = start_x.max(end_x).clamp(plot_rect.min.x, plot_rect.max.x);
    if (max_x - min_x).abs() < 1.0 {
        return;
    }

    let theme = plot.theme();
    render.push(RenderCommand::ClipRect(plot_rect));
    render.push(RenderCommand::Rect {
        rect: ScreenRect::new(
            ScreenPoint::new(min_x, plot_rect.min.y),
            ScreenPoint::new(max_x, plot_rect.max.y),
        ),
        style: RectStyle {
            fill: with_alpha(theme.selection_fill, LINK_BRUSH_FILL_ALPHA),
            stroke: with_alpha(theme.selection_border, LINK_BRUSH_BORDER_ALPHA),
            stroke_width: 1.0,
        },
    });
    render.push(RenderCommand::ClipEnd);
}
