use crate::charts::advanced::axis::TextMeasurer as _;
use crate::charts::advanced::geom::{ScreenPoint, ScreenRect};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::render::{
    LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCommand, RenderList,
    TextStyle,
};

use super::super::constants::*;
use super::super::geometry::clamp_point;
use super::super::state::{LegendEntry, LegendLayout, PlotUiState};
use super::super::text::GpuiTextMeasurer;
use super::{series_color, with_alpha};

pub(super) fn build_legend(
    render: &mut RenderList,
    plot: &Plot,
    state: &mut PlotUiState,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    let theme = plot.theme();
    let series_list = plot.series();
    if series_list.is_empty() {
        state.legend_layout = None;
        return;
    }

    let font_size = LEGEND_FONT_SIZE;
    let line_height = LEGEND_LINE_HEIGHT;
    let padding = LEGEND_PADDING;
    let text_start_x = padding
        + LEGEND_TOGGLE_DIAMETER
        + LEGEND_TOGGLE_GAP
        + LEGEND_SWATCH_WIDTH
        + LEGEND_SWATCH_GAP;
    let mut max_width: f32 = 0.0;
    for series in series_list {
        let size = measurer.measure(series.name(), font_size);
        max_width = max_width.max(size.0);
    }
    let legend_width = text_start_x + max_width + padding;
    let legend_height = series_list.len() as f32 * line_height + padding * 2.0;

    let mut origin = ScreenPoint::new(
        plot_rect.max.x - legend_width - padding,
        plot_rect.min.y + padding,
    );
    origin = clamp_point(origin, plot_rect, (legend_width, legend_height));
    let legend_rect = ScreenRect::new(
        origin,
        ScreenPoint::new(origin.x + legend_width, origin.y + legend_height),
    );

    render.push(RenderCommand::Rect {
        rect: legend_rect,
        style: RectStyle {
            fill: theme.legend_bg,
            stroke: theme.legend_border,
            stroke_width: 1.0,
        },
    });

    let mut entries = Vec::with_capacity(series_list.len());
    for (idx, series) in series_list.iter().enumerate() {
        let row_y = origin.y + padding + idx as f32 * line_height;
        let row_rect = ScreenRect::new(
            ScreenPoint::new(origin.x, row_y),
            ScreenPoint::new(origin.x + legend_width, row_y + line_height),
        );
        let row_center_y = row_y + line_height * 0.5;
        let toggle_origin = ScreenPoint::new(
            origin.x + padding,
            row_center_y - LEGEND_TOGGLE_DIAMETER * 0.5,
        );
        let toggle_rect = ScreenRect::new(
            toggle_origin,
            ScreenPoint::new(
                toggle_origin.x + LEGEND_TOGGLE_DIAMETER,
                toggle_origin.y + LEGEND_TOGGLE_DIAMETER,
            ),
        );
        entries.push(LegendEntry {
            series_id: series.id(),
            row_rect,
        });

        let visible = series.is_visible();
        let series_color = series_color(series);
        let swatch_color = if visible {
            series_color
        } else {
            with_alpha(series_color, LEGEND_HIDDEN_ALPHA)
        };
        let text_color = if visible {
            theme.axis
        } else {
            with_alpha(theme.axis, LEGEND_TEXT_HIDDEN_ALPHA)
        };
        let ring_color = if visible {
            with_alpha(theme.axis, 0.7)
        } else {
            with_alpha(theme.axis, 0.45)
        };
        let fill_color = if visible {
            series_color
        } else {
            theme.legend_bg
        };
        let toggle_center = ScreenPoint::new(
            toggle_rect.min.x + LEGEND_TOGGLE_DIAMETER * 0.5,
            toggle_rect.min.y + LEGEND_TOGGLE_DIAMETER * 0.5,
        );

        render.push(RenderCommand::Points {
            points: vec![toggle_center],
            style: MarkerStyle {
                color: ring_color,
                size: LEGEND_TOGGLE_DIAMETER,
                shape: MarkerShape::Circle,
            },
        });
        render.push(RenderCommand::Points {
            points: vec![toggle_center],
            style: MarkerStyle {
                color: fill_color,
                size: LEGEND_TOGGLE_INNER_DIAMETER,
                shape: MarkerShape::Circle,
            },
        });

        let swatch_start = ScreenPoint::new(toggle_rect.max.x + LEGEND_TOGGLE_GAP, row_center_y);
        let swatch_end = ScreenPoint::new(swatch_start.x + LEGEND_SWATCH_WIDTH, row_center_y);
        render.push(RenderCommand::LineSegments {
            segments: vec![LineSegment::new(swatch_start, swatch_end)],
            style: LineStyle {
                color: swatch_color,
                width: 2.0,
            },
        });
        let text_y = row_y + (line_height - font_size) * 0.5;
        render.push(RenderCommand::Text {
            position: ScreenPoint::new(swatch_end.x + LEGEND_SWATCH_GAP, text_y),
            text: series.name().to_string(),
            style: TextStyle {
                color: text_color,
                size: font_size,
            },
        });
    }

    state.legend_layout = Some(LegendLayout {
        rect: legend_rect,
        entries,
    });
}
