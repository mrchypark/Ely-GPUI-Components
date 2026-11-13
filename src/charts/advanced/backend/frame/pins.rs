use std::cmp::Ordering;

use crate::charts::advanced::geom::{ScreenPoint, ScreenRect};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::render::{
    LineSegment, LineStyle, MarkerShape, MarkerStyle, RectStyle, RenderCommand, RenderList,
    TextStyle,
};
use crate::charts::advanced::series::{Series, SeriesKind};
use crate::charts::advanced::style::Theme;
use crate::charts::advanced::transform::Transform;

use super::super::constants::*;
use super::super::geometry::{clamp_point, distance_sq, rect_intersects_any};
use super::super::text::GpuiTextMeasurer;

pub(super) fn build_pins(
    render: &mut RenderList,
    plot: &Plot,
    transform: &Transform,
    plot_rect: ScreenRect,
    measurer: &GpuiTextMeasurer<'_>,
) {
    if plot.pins().is_empty() {
        return;
    }

    let theme = plot.theme();
    let font_size = 12.0;
    let line_height = 14.0;
    let mut labels: Vec<PinLabel> = Vec::new();
    render.push(RenderCommand::ClipRect(plot_rect));

    for pin in plot.pins() {
        let Some(series) = plot
            .series()
            .iter()
            .find(|series| series.id() == pin.series_id)
        else {
            continue;
        };
        if !series.is_visible() {
            continue;
        }
        let Some(point) = series.with_store(|store| store.pinned_point(pin.point_index)) else {
            continue;
        };
        let Some(screen) = transform.data_to_screen(point) else {
            continue;
        };

        if screen.x < plot_rect.min.x
            || screen.x > plot_rect.max.x
            || screen.y < plot_rect.min.y
            || screen.y > plot_rect.max.y
        {
            continue;
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
        let size = measurer.measure_multiline(&label, font_size);
        labels.push(PinLabel {
            screen,
            label,
            size,
        });
    }

    if labels.is_empty() {
        render.push(RenderCommand::ClipEnd);
        return;
    }

    let plot_area = plot_rect.width().max(1.0) * plot_rect.height().max(1.0);
    let total_label_area: f32 = labels.iter().map(|label| label.size.0 * label.size.1).sum();
    let dense =
        labels.len() > MAX_PIN_LABELS || total_label_area > plot_area * MAX_PIN_LABEL_COVERAGE;

    let mut clusters = cluster_pin_labels(&labels, PIN_CLUSTER_RADIUS);
    clusters.sort_by(|a, b| {
        let size_cmp = b.len().cmp(&a.len());
        if size_cmp != Ordering::Equal {
            return size_cmp;
        }
        let min_a = a.iter().copied().min().unwrap_or(0);
        let min_b = b.iter().copied().min().unwrap_or(0);
        min_a.cmp(&min_b)
    });

    let mut placed: Vec<ScreenRect> = Vec::new();
    let mut single_budget = if dense { MAX_PIN_LABELS } else { usize::MAX };
    for cluster in clusters {
        if cluster.len() >= 2 {
            if !dense {
                let mut local_placed = placed.clone();
                let mut placements: Vec<(ScreenPoint, ScreenRect, usize)> = Vec::new();
                let mut success = true;
                for index in &cluster {
                    let entry = &labels[*index];
                    if let Some((origin, rect)) = place_label(
                        entry.screen,
                        entry.size,
                        plot_rect,
                        PIN_LABEL_OFFSET,
                        &local_placed,
                    ) {
                        local_placed.push(rect);
                        placements.push((origin, rect, *index));
                    } else {
                        success = false;
                        break;
                    }
                }

                if success {
                    placed = local_placed;
                    for (origin, rect, index) in placements {
                        let entry = &labels[index];
                        push_label_with_leader(
                            render,
                            rect,
                            origin,
                            entry.screen,
                            &entry.label,
                            font_size,
                            line_height,
                            theme,
                        );
                    }
                    continue;
                }
            }

            let center = cluster_center(&labels, &cluster);
            let label = format!("{} pins", cluster.len());
            let size = measurer.measure_multiline(&label, font_size);
            if let Some((origin, rect)) =
                place_label(center, size, plot_rect, PIN_LABEL_OFFSET, &placed)
            {
                placed.push(rect);
                push_label_with_leader(
                    render,
                    rect,
                    origin,
                    center,
                    &label,
                    font_size,
                    line_height,
                    theme,
                );
            }
            continue;
        }

        if single_budget == 0 {
            continue;
        }
        let index = cluster[0];
        let entry = &labels[index];
        if let Some((origin, rect)) = place_label(
            entry.screen,
            entry.size,
            plot_rect,
            PIN_LABEL_OFFSET,
            &placed,
        ) {
            placed.push(rect);
            push_label_with_leader(
                render,
                rect,
                origin,
                entry.screen,
                &entry.label,
                font_size,
                line_height,
                theme,
            );
            single_budget = single_budget.saturating_sub(1);
        }
    }

    render.push(RenderCommand::ClipEnd);
}

#[allow(clippy::too_many_arguments)]
#[derive(Debug, Clone)]
struct PinLabel {
    screen: ScreenPoint,
    label: String,
    size: (f32, f32),
}

pub(super) fn marker_style_and_size(series: &Series) -> (MarkerStyle, f32) {
    match series.kind() {
        SeriesKind::Line(line) => (
            MarkerStyle {
                color: line.color,
                size: 6.0,
                shape: MarkerShape::Circle,
            },
            6.0,
        ),
        SeriesKind::Scatter(marker) => (
            MarkerStyle {
                color: marker.color,
                size: marker.size.max(6.0),
                shape: marker.shape,
            },
            marker.size.max(6.0),
        ),
    }
}

fn cluster_pin_labels(labels: &[PinLabel], radius: f32) -> Vec<Vec<usize>> {
    let radius_sq = radius * radius;
    let mut visited = vec![false; labels.len()];
    let mut clusters: Vec<Vec<usize>> = Vec::new();

    for start in 0..labels.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut cluster = Vec::new();
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            cluster.push(index);
            for next in 0..labels.len() {
                if visited[next] {
                    continue;
                }
                if distance_sq(labels[index].screen, labels[next].screen) <= radius_sq {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }
        clusters.push(cluster);
    }

    clusters
}

fn cluster_center(labels: &[PinLabel], cluster: &[usize]) -> ScreenPoint {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for index in cluster {
        let screen = labels[*index].screen;
        sum_x += screen.x;
        sum_y += screen.y;
    }
    let count = cluster.len().max(1) as f32;
    ScreenPoint::new(sum_x / count, sum_y / count)
}

fn pin_label_candidates(screen: ScreenPoint, size: (f32, f32), offset: f32) -> [ScreenPoint; 6] {
    [
        ScreenPoint::new(screen.x + offset, screen.y + offset),
        ScreenPoint::new(screen.x + offset, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 - offset, screen.y + offset),
        ScreenPoint::new(screen.x - size.0 - offset, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 * 0.5, screen.y - size.1 - offset),
        ScreenPoint::new(screen.x - size.0 * 0.5, screen.y + offset),
    ]
}

fn place_label(
    screen: ScreenPoint,
    size: (f32, f32),
    plot_rect: ScreenRect,
    offset: f32,
    placed: &[ScreenRect],
) -> Option<(ScreenPoint, ScreenRect)> {
    for origin in pin_label_candidates(screen, size, offset) {
        let origin = clamp_point(origin, plot_rect, size);
        let rect = ScreenRect::new(
            origin,
            ScreenPoint::new(origin.x + size.0, origin.y + size.1),
        );
        if !rect_intersects_any(rect, placed) {
            return Some((origin, rect));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn push_label_with_leader(
    render: &mut RenderList,
    rect: ScreenRect,
    origin: ScreenPoint,
    screen: ScreenPoint,
    label: &str,
    font_size: f32,
    line_height: f32,
    theme: &Theme,
) {
    let anchor = ScreenPoint::new(
        screen.x.clamp(rect.min.x, rect.max.x),
        screen.y.clamp(rect.min.y, rect.max.y),
    );
    render.push(RenderCommand::LineSegments {
        segments: vec![LineSegment::new(screen, anchor)],
        style: LineStyle {
            color: theme.pin_border,
            width: 1.0,
        },
    });
    render.push(RenderCommand::Rect {
        rect,
        style: RectStyle {
            fill: theme.pin_bg,
            stroke: theme.pin_border,
            stroke_width: 1.0,
        },
    });
    for (index, line) in label.lines().enumerate() {
        let line_y = origin.y + index as f32 * line_height + 2.0;
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
