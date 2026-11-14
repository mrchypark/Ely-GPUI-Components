use gpui::{Bounds, Pixels, Rgba, Window};

use crate::charts::advanced::axis::{AxisConfig, AxisLayout, TextMeasurer};
use crate::charts::advanced::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::render::{
    LineSegment, LineStyle, RectStyle, RenderCacheKey, RenderCommand, RenderList, TextStyle,
    build_line_segments, build_scatter_points,
};
use crate::charts::advanced::series::{Series, SeriesKind};
use crate::charts::advanced::transform::Transform;
use crate::charts::advanced::view::{Range, Viewport};

use super::config::PlotViewConfig;
use super::constants::*;
use super::geometry::normalized_rect;
use super::hover::update_hover_target;
use super::state::PlotUiState;
use super::text::GpuiTextMeasurer;

mod axes;
mod hud;
mod legend;
mod pins;
#[cfg(test)]
mod tests;

use axes::{build_axes, build_axis_titles};
use hud::{build_hover, build_linked_brush, build_linked_cursor};
use legend::build_legend;
use pins::build_pins;

#[derive(Debug, Clone)]
pub(crate) struct PlotFrame {
    pub(crate) render: RenderList,
}

pub(crate) fn build_frame(
    plot: &mut Plot,
    state: &mut PlotUiState,
    config: &PlotViewConfig,
    bounds: Bounds<Pixels>,
    window: &Window,
    font_family: gpui::SharedString,
) -> PlotFrame {
    let mut render = RenderList::new();

    let full_width = f32::from(bounds.size.width);
    let full_height = f32::from(bounds.size.height);
    if full_width <= 1.0 || full_height <= 1.0 {
        state.view_bounds = None;
        return PlotFrame { render };
    }
    state.view_bounds = Some(bounds);

    let viewport = plot
        .refresh_viewport(config.padding_frac, config.min_padding)
        .unwrap_or_else(|| Viewport::new(Range::new(0.0, 1.0), Range::new(0.0, 1.0)));

    state.viewport = Some(viewport);

    let measurer = GpuiTextMeasurer::new(window, font_family);

    let mut plot_width = full_width;
    let mut plot_height = full_height;

    let x_layout = state
        .x_layout
        .update(plot.x_axis(), viewport.x, plot_width as u32, &measurer)
        .clone();
    let y_layout = state
        .y_layout
        .update(plot.y_axis(), viewport.y, plot_height as u32, &measurer)
        .clone();

    let x_title = axis_title_text(plot.x_axis());
    let x_title_size = x_title
        .as_ref()
        .map(|title| measurer.measure(title, plot.x_axis().label_size()))
        .unwrap_or((0.0, 0.0));
    let y_title = axis_title_text(plot.y_axis());
    let y_title_size = y_title
        .as_ref()
        .map(|title| measurer.measure(title, plot.y_axis().label_size()))
        .unwrap_or((0.0, 0.0));

    let x_axis_height =
        x_layout.max_label_size.1 + TICK_LENGTH_MAJOR + AXIS_PADDING * 2.0 + x_title_size.1;
    let y_axis_width = (y_layout.max_label_size.0 + TICK_LENGTH_MAJOR + AXIS_PADDING * 2.0)
        .max(y_title_size.0 + AXIS_PADDING * 2.0);

    let x_axis_height = x_axis_height.clamp(0.0, full_height - 1.0);
    let y_axis_width = y_axis_width.clamp(0.0, full_width - 1.0);

    plot_width = (full_width - y_axis_width).max(1.0);
    plot_height = (full_height - x_axis_height).max(1.0);

    let x_layout = state
        .x_layout
        .update(plot.x_axis(), viewport.x, plot_width as u32, &measurer)
        .clone();
    let y_layout = state
        .y_layout
        .update(plot.y_axis(), viewport.y, plot_height as u32, &measurer)
        .clone();

    let origin_x = f32::from(bounds.origin.x);
    let origin_y = f32::from(bounds.origin.y);
    let full_max_x = origin_x + full_width;
    let full_max_y = origin_y + full_height;

    let plot_rect = ScreenRect::new(
        ScreenPoint::new(origin_x + y_axis_width, origin_y),
        ScreenPoint::new(full_max_x, full_max_y - x_axis_height),
    );
    let x_axis_rect = ScreenRect::new(
        ScreenPoint::new(plot_rect.min.x, plot_rect.max.y),
        ScreenPoint::new(plot_rect.max.x, full_max_y),
    );
    let y_axis_rect = ScreenRect::new(
        ScreenPoint::new(origin_x, plot_rect.min.y),
        ScreenPoint::new(plot_rect.min.x, plot_rect.max.y),
    );

    state.regions = crate::charts::advanced::interaction::PlotRegions {
        plot: plot_rect,
        x_axis: x_axis_rect,
        y_axis: y_axis_rect,
    };
    state.plot_rect = Some(plot_rect);

    let transform = Transform::with_scales(
        viewport,
        plot_rect,
        plot.x_axis().scale(),
        plot.y_axis().scale(),
    );
    state.transform = transform.clone();

    if let Some(transform) = transform {
        build_grid(
            &mut render,
            plot,
            &x_layout,
            &y_layout,
            &transform,
            plot_rect,
        );
        build_series(&mut render, plot, state, &transform, plot_rect);
        build_linked_brush(&mut render, plot, state, &transform, plot_rect);
        build_selection(&mut render, plot, state);
        update_hover_target(
            plot,
            state,
            &transform,
            plot_rect,
            config.pin_threshold_px,
            config.unpin_threshold_px,
        );
        build_pins(&mut render, plot, &transform, plot_rect, &measurer);
        build_axes(
            &mut render,
            plot,
            &x_layout,
            &y_layout,
            plot_rect,
            &transform,
            x_axis_rect,
            y_axis_rect,
            &measurer,
        );
        if config.show_legend {
            build_legend(&mut render, plot, state, plot_rect, &measurer);
        } else {
            state.legend_layout = None;
        }
        build_linked_cursor(&mut render, plot, state, &transform, plot_rect, &measurer);
        if config.show_hover {
            build_hover(&mut render, plot, state, &transform, plot_rect, &measurer);
        }
        build_axis_titles(
            &mut render,
            plot,
            plot_rect,
            x_axis_rect,
            y_axis_rect,
            &measurer,
        );
    } else {
        state.legend_layout = None;
        let message = "Invalid axis range";
        let size = measurer.measure(message, 14.0);
        let pos = ScreenPoint::new(
            plot_rect.min.x + (plot_rect.width() - size.0) * 0.5,
            plot_rect.min.y + (plot_rect.height() - size.1) * 0.5,
        );
        render.push(RenderCommand::Text {
            position: pos,
            text: message.to_string(),
            style: TextStyle {
                color: plot.theme().axis,
                size: 14.0,
            },
        });
    }

    PlotFrame { render }
}

fn build_grid(
    render: &mut RenderList,
    plot: &Plot,
    x_layout: &AxisLayout,
    y_layout: &AxisLayout,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let theme = plot.theme();
    let mut major = Vec::new();
    let mut minor = Vec::new();

    if plot.x_axis().show_grid() {
        for tick in &x_layout.ticks {
            let x = transform
                .data_to_screen(DataPoint::new(tick.value, transform.viewport().y.min))
                .map(|p| p.x);
            let Some(x) = x else { continue };
            let segment = LineSegment::new(
                ScreenPoint::new(x, plot_rect.min.y),
                ScreenPoint::new(x, plot_rect.max.y),
            );
            if tick.is_major {
                major.push(segment);
            } else if plot.x_axis().show_minor_grid() {
                minor.push(segment);
            }
        }
    }

    if plot.y_axis().show_grid() {
        for tick in &y_layout.ticks {
            let y = transform
                .data_to_screen(DataPoint::new(transform.viewport().x.min, tick.value))
                .map(|p| p.y);
            let Some(y) = y else { continue };
            let segment = LineSegment::new(
                ScreenPoint::new(plot_rect.min.x, y),
                ScreenPoint::new(plot_rect.max.x, y),
            );
            if tick.is_major {
                major.push(segment);
            } else if plot.y_axis().show_minor_grid() {
                minor.push(segment);
            }
        }
    }

    render.push(RenderCommand::ClipRect(plot_rect));
    if !minor.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: minor,
            style: LineStyle {
                color: theme.grid_minor,
                width: 1.0,
            },
        });
    }
    if !major.is_empty() {
        render.push(RenderCommand::LineSegments {
            segments: major,
            style: LineStyle {
                color: theme.grid_major,
                width: 1.0,
            },
        });
    }

    if plot.x_axis().show_zero_line() {
        if transform.viewport().y.min <= 0.0 && transform.viewport().y.max >= 0.0 {
            if let Some(y) = transform
                .data_to_screen(DataPoint::new(transform.viewport().x.min, 0.0))
                .map(|p| p.y)
            {
                render.push(RenderCommand::LineSegments {
                    segments: vec![LineSegment::new(
                        ScreenPoint::new(plot_rect.min.x, y),
                        ScreenPoint::new(plot_rect.max.x, y),
                    )],
                    style: LineStyle {
                        color: theme.axis,
                        width: 1.0,
                    },
                });
            }
        }
    }

    if plot.y_axis().show_zero_line() {
        if transform.viewport().x.min <= 0.0 && transform.viewport().x.max >= 0.0 {
            if let Some(x) = transform
                .data_to_screen(DataPoint::new(0.0, transform.viewport().y.min))
                .map(|p| p.x)
            {
                render.push(RenderCommand::LineSegments {
                    segments: vec![LineSegment::new(
                        ScreenPoint::new(x, plot_rect.min.y),
                        ScreenPoint::new(x, plot_rect.max.y),
                    )],
                    style: LineStyle {
                        color: theme.axis,
                        width: 1.0,
                    },
                });
            }
        }
    }

    render.push(RenderCommand::ClipEnd);
}

fn build_series(
    render: &mut RenderList,
    plot: &Plot,
    state: &mut PlotUiState,
    transform: &Transform,
    plot_rect: ScreenRect,
) {
    let plot_width = plot_rect.width().max(1.0) as usize;
    let size = (
        plot_rect.width().round() as u32,
        plot_rect.height().round() as u32,
    );

    let active_ids: std::collections::HashSet<_> = plot.series().iter().map(Series::id).collect();
    state.series_cache.retain(|id, _| active_ids.contains(id));

    render.push(RenderCommand::ClipRect(plot_rect));

    for series in plot.series() {
        if !series.is_visible() {
            continue;
        }
        let cache = state.series_cache.entry(series.id()).or_default();
        let key = RenderCacheKey {
            viewport: transform.viewport(),
            size,
            generation: series.generation(),
        };
        if cache.key.as_ref() != Some(&key) {
            series.with_store(|store| {
                let decimated = store.decimate(
                    transform.viewport().x,
                    plot_width,
                    &mut state.decimation_scratch,
                );
                cache.points.clear();
                cache.points.extend_from_slice(decimated);
            });
            cache.key = Some(key.clone());
        }

        match series.kind() {
            SeriesKind::Line(style) => {
                let mut segments = Vec::new();
                build_line_segments(&cache.points, transform, plot_rect, &mut segments);
                if !segments.is_empty() {
                    render.push(RenderCommand::LineSegments {
                        segments,
                        style: *style,
                    });
                }
            }
            SeriesKind::Scatter(style) => {
                let mut points = Vec::new();
                build_scatter_points(&cache.points, transform, plot_rect, &mut points);
                if !points.is_empty() {
                    render.push(RenderCommand::Points {
                        points,
                        style: *style,
                    });
                }
            }
        }
    }

    render.push(RenderCommand::ClipEnd);
}

fn build_selection(render: &mut RenderList, plot: &Plot, state: &PlotUiState) {
    if let Some(rect) = state.selection_rect {
        let rect = normalized_rect(rect);
        render.push(RenderCommand::Rect {
            rect,
            style: RectStyle {
                fill: plot.theme().selection_fill,
                stroke: plot.theme().selection_border,
                stroke_width: 1.0,
            },
        });
    }
}

pub(super) fn axis_title_text(axis: &AxisConfig) -> Option<String> {
    match (axis.title(), axis.units()) {
        (Some(title), Some(units)) => Some(format!("{title} ({units})")),
        (Some(title), None) => Some(title.to_string()),
        (None, Some(units)) => Some(units.to_string()),
        (None, None) => None,
    }
}

pub(super) fn series_color(series: &Series) -> Rgba {
    match series.kind() {
        SeriesKind::Line(style) => style.color,
        SeriesKind::Scatter(style) => style.color,
    }
}

pub(super) fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        a: (color.a * alpha).clamp(0.0, 1.0),
        ..color
    }
}
