use gpui::Rgba;

use crate::charts::advanced::axis::{AxisLayout, TextMeasurer as _};
use crate::charts::advanced::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::render::{
    LineSegment, LineStyle, RectStyle, RenderCommand, RenderList, TextStyle,
};
use crate::charts::advanced::transform::Transform;

use super::super::constants::*;
use super::super::geometry::rect_intersects;
use super::super::text::GpuiTextMeasurer;
use super::axis_title_text;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_axes(
    render: &mut RenderList,
    plot: &Plot,
    x_layout: &AxisLayout,
    y_layout: &AxisLayout,
    plot_rect: ScreenRect,
    transform: &Transform,
    x_axis_rect: ScreenRect,
    y_axis_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let mut x_ticks_major = Vec::new();
    let mut x_ticks_minor = Vec::new();
    let mut y_ticks_major = Vec::new();
    let mut y_ticks_minor = Vec::new();
    let label_gap = 2.0_f32;
    let mut last_x_label_right = f32::NEG_INFINITY;
    let mut last_y_label_top = f32::INFINITY;
    let x_title_rect = axis_title_text(plot.x_axis()).map(|title| {
        let size = measurer.measure(&title, plot.x_axis().label_size());
        let pos = clamp_label_position(
            ScreenPoint::new(
                plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
                x_axis_rect.max.y - size.1 - AXIS_PADDING,
            ),
            size,
            x_axis_rect,
        );
        ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, pos.y + size.1))
    });
    let y_title_rect = axis_title_text(plot.y_axis()).map(|title| {
        let size = measurer.measure(&title, plot.y_axis().label_size());
        let pos = clamp_label_position(
            ScreenPoint::new(
                y_axis_rect.min.x + AXIS_PADDING,
                y_axis_rect.min.y + AXIS_PADDING,
            ),
            size,
            y_axis_rect,
        );
        ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, pos.y + size.1))
    });

    if plot.x_axis().show_border() {
        render.push(RenderCommand::Rect {
            rect: plot_rect,
            style: RectStyle {
                fill: Rgba {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                },
                stroke: theme.axis,
                stroke_width: 1.0,
            },
        });
    }

    render.push(RenderCommand::ClipRect(x_axis_rect));
    for tick in &x_layout.ticks {
        if let Some(x) = transform
            .data_to_screen(DataPoint::new(tick.value, transform.viewport().y.min))
            .map(|p| p.x)
        {
            let length = if tick.is_major {
                TICK_LENGTH_MAJOR
            } else {
                TICK_LENGTH_MINOR
            };
            let segment = LineSegment::new(
                ScreenPoint::new(x, plot_rect.max.y),
                ScreenPoint::new(x, plot_rect.max.y + length),
            );
            if tick.is_major {
                x_ticks_major.push(segment);
            } else if plot.x_axis().show_minor_grid() {
                x_ticks_minor.push(segment);
            }

            if tick.is_major && !tick.label.is_empty() {
                let size = measurer.measure(&tick.label, plot.x_axis().label_size());
                let mut pos = x_tick_label_position(x, size, plot_rect);
                let max_x = (x_axis_rect.max.x - size.0).max(x_axis_rect.min.x);
                pos.x = pos.x.clamp(x_axis_rect.min.x, max_x);
                let label_rect =
                    ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, pos.y + size.1));
                if let Some(visible_rect) = tick_label_visible_rect(label_rect, x_axis_rect) {
                    let overlaps_title = x_title_rect
                        .map(|rect| rect_intersects(visible_rect, rect))
                        .unwrap_or(false);
                    if !overlaps_title && visible_rect.min.x >= last_x_label_right + label_gap {
                        render.push(RenderCommand::Text {
                            position: pos,
                            text: tick.label.clone(),
                            style: TextStyle {
                                color: theme.axis,
                                size: plot.x_axis().label_size(),
                            },
                        });
                        last_x_label_right = visible_rect.max.x;
                    }
                }
            }
        }
    }
    if !x_ticks_minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: x_ticks_minor,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
    if !x_ticks_major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: x_ticks_major,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
    render.push(RenderCommand::ClipEnd);

    render.push(RenderCommand::ClipRect(y_axis_rect));
    for tick in &y_layout.ticks {
        if let Some(y) = transform
            .data_to_screen(DataPoint::new(transform.viewport().x.min, tick.value))
            .map(|p| p.y)
        {
            let length = if tick.is_major {
                TICK_LENGTH_MAJOR
            } else {
                TICK_LENGTH_MINOR
            };
            let segment = LineSegment::new(
                ScreenPoint::new(plot_rect.min.x - length, y),
                ScreenPoint::new(plot_rect.min.x, y),
            );
            if tick.is_major {
                y_ticks_major.push(segment);
            } else if plot.y_axis().show_minor_grid() {
                y_ticks_minor.push(segment);
            }

            if tick.is_major && !tick.label.is_empty() {
                let size = measurer.measure(&tick.label, plot.y_axis().label_size());
                let pos = y_tick_label_position(y, size, plot_rect);
                let label_rect =
                    ScreenRect::new(pos, ScreenPoint::new(pos.x + size.0, pos.y + size.1));
                if let Some(visible_rect) = tick_label_visible_rect(label_rect, y_axis_rect) {
                    let overlaps_title = y_title_rect
                        .map(|rect| rect_intersects(visible_rect, rect))
                        .unwrap_or(false);
                    if !overlaps_title && visible_rect.max.y <= last_y_label_top - label_gap {
                        render.push(RenderCommand::Text {
                            position: pos,
                            text: tick.label.clone(),
                            style: TextStyle {
                                color: theme.axis,
                                size: plot.y_axis().label_size(),
                            },
                        });
                        last_y_label_top = visible_rect.min.y;
                    }
                }
            }
        }
    }
    if !y_ticks_minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: y_ticks_minor,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
    if !y_ticks_major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: y_ticks_major,
            style: LineStyle {
                color: theme.axis,
                width: 1.0,
            },
        });
    }
    render.push(RenderCommand::ClipEnd);
}

pub(super) fn build_axis_titles(
    render: &mut RenderList,
    plot: &Plot,
    plot_rect: ScreenRect,
    x_axis_rect: ScreenRect,
    y_axis_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    if let Some(title) = axis_title_text(plot.x_axis()) {
        let size = measurer.measure(&title, plot.x_axis().label_size());
        let pos = clamp_label_position(
            ScreenPoint::new(
                plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
                x_axis_rect.max.y - size.1 - AXIS_PADDING,
            ),
            size,
            x_axis_rect,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: title,
            style: TextStyle {
                color: theme.axis,
                size: plot.x_axis().label_size(),
            },
        });
    }

    if let Some(title) = axis_title_text(plot.y_axis()) {
        let pos = clamp_label_position(
            ScreenPoint::new(
                y_axis_rect.min.x + AXIS_PADDING,
                y_axis_rect.min.y + AXIS_PADDING,
            ),
            measurer.measure(&title, plot.y_axis().label_size()),
            y_axis_rect,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: title,
            style: TextStyle {
                color: theme.axis,
                size: plot.y_axis().label_size(),
            },
        });
    }
}

fn clamp_label_position(pos: ScreenPoint, size: (f32, f32), rect: ScreenRect) -> ScreenPoint {
    let max_x = (rect.max.x - size.0).max(rect.min.x);
    let max_y = (rect.max.y - size.1).max(rect.min.y);
    ScreenPoint::new(
        pos.x.clamp(rect.min.x, max_x),
        pos.y.clamp(rect.min.y, max_y),
    )
}

pub(super) fn x_tick_label_position(
    x: f32,
    size: (f32, f32),
    plot_rect: ScreenRect,
) -> ScreenPoint {
    ScreenPoint::new(
        x - size.0 * 0.5,
        plot_rect.max.y + TICK_LENGTH_MAJOR + AXIS_PADDING,
    )
}

pub(super) fn y_tick_label_position(
    y: f32,
    size: (f32, f32),
    plot_rect: ScreenRect,
) -> ScreenPoint {
    ScreenPoint::new(
        plot_rect.min.x - TICK_LENGTH_MAJOR - AXIS_PADDING - size.0,
        y - size.1 * 0.5,
    )
}

pub(super) fn tick_label_visible_rect(
    label_rect: ScreenRect,
    axis_rect: ScreenRect,
) -> Option<ScreenRect> {
    let min_x = label_rect.min.x.max(axis_rect.min.x);
    let min_y = label_rect.min.y.max(axis_rect.min.y);
    let max_x = label_rect.max.x.min(axis_rect.max.x);
    let max_y = label_rect.max.y.min(axis_rect.max.y);

    (min_x < max_x && min_y < max_y).then(|| {
        ScreenRect::new(
            ScreenPoint::new(min_x, min_y),
            ScreenPoint::new(max_x, max_y),
        )
    })
}
