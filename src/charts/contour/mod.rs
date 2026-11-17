use std::{fmt::Write as _, rc::Rc};

use gpui::{
    App, Bounds, ContentMask, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchPhase, Window, canvas, div, fill,
    size,
};

use super::{
    axes::{anchored, below, beside},
    geometry::{Rect, frame},
    paint::{at, finish, measure, place, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::{Linear, compact, nice},
    series::drawable,
    tiles::key,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::tabular,
};

pub(super) type Segment = ((f32, f32), (f32, f32));
pub(super) type Domain = ((f64, f64), (f64, f64));

#[derive(Default)]
pub(super) struct ContourState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) domain: Option<Domain>,
    pub(super) drag_last: Option<(f32, f32)>,
    pub(super) dragged: bool,
}

pub(super) enum Source {
    Points(Vec<(f64, f64)>),
    Grid {
        cols: usize,
        rows: usize,
        values: Vec<f64>,
    },
}

/// 2D density and scalar-field contour bands and isolines extracted via Marching Squares.
#[derive(IntoElement)]
pub struct ContourChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) source: Source,
    pub(super) grid_size: (usize, usize),
    pub(super) bandwidth: Option<f64>,
    pub(super) levels: usize,
    pub(super) thresholds: Option<Vec<f64>>,
    pub(super) axes: (SharedString, SharedString),
    pub(super) format: Format,
}

impl ContourChart {
    /// Estimate 2D kernel density contours from scattered `(x, y)` points.
    pub fn from_points(
        id: impl Into<ElementId>,
        points: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        let pts: Vec<(f64, f64)> = points.into_iter().collect();
        assert!(
            pts.iter().all(|(x, y)| drawable(*x) && drawable(*y)),
            "contour points need finite values, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            source: Source::Points(pts),
            grid_size: (36, 36),
            bandwidth: None,
            levels: 6,
            thresholds: None,
            axes: ("X".into(), "Y".into()),
            format: Rc::new(compact),
        }
    }

    /// Build contours from an explicit row-major scalar field (`cols >= 2`, `rows >= 2`).
    pub fn from_grid(
        id: impl Into<ElementId>,
        cols: usize,
        rows: usize,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        assert!(
            cols >= 2 && rows >= 2,
            "a contour grid needs at least 2x2 samples"
        );
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            cols * rows,
            "contour grid length must equal cols * rows"
        );
        assert!(
            values.iter().all(|v| drawable(*v)),
            "contour grid values must be finite, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            source: Source::Grid { cols, rows, values },
            grid_size: (cols, rows),
            bandwidth: None,
            levels: 6,
            thresholds: None,
            axes: ("X".into(), "Y".into()),
            format: Rc::new(compact),
        }
    }

    /// Override the KDE grid resolution (clamped to `8..=96` per axis).
    pub fn grid_size(mut self, cols: usize, rows: usize) -> Self {
        self.grid_size = (cols.clamp(8, 96), rows.clamp(8, 96));
        self
    }

    /// Override the Gaussian KDE bandwidth in data units.
    pub fn bandwidth(mut self, bandwidth: f64) -> Self {
        assert!(
            bandwidth.is_finite() && bandwidth > 0.0,
            "contour bandwidth must be finite and positive"
        );
        self.bandwidth = Some(bandwidth);
        self
    }

    /// Set the number of automatically spaced contour levels (at least 1).
    pub fn levels(mut self, levels: usize) -> Self {
        self.levels = levels.max(1);
        self.thresholds = None;
        self
    }

    /// Set explicit sorted threshold values.
    pub fn thresholds(mut self, thresholds: impl IntoIterator<Item = f64>) -> Self {
        let mut vals: Vec<f64> = thresholds.into_iter().filter(|v| v.is_finite()).collect();
        vals.sort_by(f64::total_cmp);
        vals.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        self.thresholds = Some(vals);
        self
    }

    pub fn axes(mut self, x: impl Into<SharedString>, y: impl Into<SharedString>) -> Self {
        self.axes = (x.into(), y.into());
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export the scalar field grid as CSV (`col,row,x,y,value`).
    pub fn csv(&self) -> String {
        let field = self.build_field(None);
        let mut out = String::from("col,row,x,y,value");
        if field.cols == 0 || field.rows == 0 {
            return out;
        }
        for r in 0..field.rows {
            let ty = r as f64 / (field.rows - 1).max(1) as f64;
            let y = field.y_domain.0 + (field.y_domain.1 - field.y_domain.0) * ty;
            for c in 0..field.cols {
                let tx = c as f64 / (field.cols - 1).max(1) as f64;
                let x = field.x_domain.0 + (field.x_domain.1 - field.x_domain.0) * tx;
                let v = field.values[r * field.cols + c];
                write!(out, "\n{c},{r},{x},{y},{v}").expect("csv write");
            }
        }
        out
    }

    /// Export contour bands and isolines as standalone SVG.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let field = self.build_field(None);
        let x_scale = Linear::new(field.x_domain, (rect.x, rect.x + rect.w));
        let y_scale = Linear::new(field.y_domain, (rect.y + rect.h, rect.y));
        let bands = build_bands(
            &field,
            x_scale,
            y_scale,
            self.levels,
            self.thresholds.as_deref(),
        );
        let ink = tint(colors, 0);
        let (bg_hex, _) = svg_hex(colors.bg);
        let (ink_hex, _) = svg_hex(ink);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        let total_levels = bands.len().max(1);
        for (lvl_ix, band) in bands.iter().enumerate() {
            let alpha = 0.14 + 0.68 * ((lvl_ix + 1) as f32 / total_levels as f32);
            for poly in &band.polygons {
                if poly.len() < 3 {
                    continue;
                }
                let mut d = String::new();
                for (idx, &(px, py)) in poly.iter().enumerate() {
                    let cmd = if idx == 0 { 'M' } else { 'L' };
                    write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
                }
                d.push('Z');
                write!(
                    out,
                    r#"<path d="{d}" fill="{ink_hex}" fill-opacity="{alpha:.2}"/>"#
                )
                .expect("svg write");
            }
            for &(a, b) in &band.isolines {
                write!(
                    out,
                    r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{ink_hex}" stroke-opacity="0.7" stroke-width="1"/>"#,
                    a.0, a.1, b.0, b.1
                )
                .expect("svg write");
            }
        }
        out.push_str("</svg>");
        out
    }

    fn base_domain(&self) -> Domain {
        match &self.source {
            Source::Grid { cols, rows, .. } => {
                ((0.0, (*cols - 1) as f64), (0.0, (*rows - 1) as f64))
            }
            Source::Points(points) => points_domain(points),
        }
    }

    fn build_field(&self, domain: Option<Domain>) -> ScalarField {
        match &self.source {
            Source::Grid { cols, rows, values } => ScalarField {
                cols: *cols,
                rows: *rows,
                x_domain: (0.0, (*cols - 1) as f64),
                y_domain: (0.0, (*rows - 1) as f64),
                values: values.clone(),
            },
            Source::Points(points) => kde_field(points, self.grid_size, self.bandwidth, domain),
        }
    }
}

mod field;
mod render;
#[cfg(test)]
mod tests;
use field::*;
