use std::{collections::HashMap, fmt::Write as _, rc::Rc};

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
    typography::{format, tabular},
};

pub(super) const SQRT_3: f32 = 1.732_050_8;

pub(super) type Domain = ((f64, f64), (f64, f64));

#[derive(Default)]
pub(super) struct HexState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) domain: Option<Domain>,
    pub(super) drag_last: Option<(f32, f32)>,
    pub(super) dragged: bool,
}

impl HexState {
    fn pointed(&self, count: usize) -> Option<usize> {
        self.hover.filter(|ix| *ix < count)
    }
}

/// A hexagonal bin holding aggregated screen and data centers and its point count.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HexBin {
    pub center: (f32, f32),
    pub data_center: (f64, f64),
    pub count: usize,
}

/// Dense 2D point clouds aggregated into regular hexagonal bins shaded by count.
#[derive(IntoElement)]
pub struct HexbinChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) points: Vec<(f64, f64)>,
    pub(super) radius: f32,
    pub(super) axes: (SharedString, SharedString),
    pub(super) format: Format,
}

impl HexbinChart {
    /// Create a hexagonal-binning density chart.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            points: Vec::new(),
            radius: 12.0,
            axes: ("X".into(), "Y".into()),
            format: Rc::new(compact),
        }
    }

    /// Append a single `(x, y)` point.
    pub fn point(mut self, x: f64, y: f64) -> Self {
        assert!(
            drawable(x) && drawable(y),
            "a hexbin point needs finite values, within charts::LIMIT"
        );
        self.points.push((x, y));
        self
    }

    /// Append multiple `(x, y)` points.
    pub fn points(mut self, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        for (x, y) in points {
            self = self.point(x, y);
        }
        self
    }

    /// Set the hexagon circumradius in logical pixels (clamped to at least 4 px).
    pub fn radius(mut self, radius: f32) -> Self {
        assert!(
            radius.is_finite() && radius > 0.0,
            "a hexbin radius must be finite and positive"
        );
        self.radius = radius.max(4.0);
        self
    }

    /// Set the axis labels used in tooltips.
    pub fn axes(mut self, x: impl Into<SharedString>, y: impl Into<SharedString>) -> Self {
        self.axes = (x.into(), y.into());
        self
    }

    /// Format numeric coordinates in axes and tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export aggregated bins as CSV (`x,y,count`) at the given reference layout size.
    pub fn csv(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem) = (theme.chart(), theme.base_rem());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let layout = compute_layout(&self.points, rect, self.radius, None);
        let mut out = String::from("x,y,count");
        for bin in &layout.bins {
            write!(
                out,
                "\n{},{},{}",
                bin.data_center.0, bin.data_center.1, bin.count
            )
            .expect("csv write");
        }
        out
    }

    /// Export the chart as a standalone SVG document.
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
        let layout = compute_layout(&self.points, rect, self.radius, None);
        let ink = tint(colors, 0);
        let (bg_hex, _) = svg_hex(colors.bg);
        let (ink_hex, _) = svg_hex(ink);
        let (grid_hex, _) = svg_hex(colors.border);
        let (label_hex, _) = svg_hex(colors.fg_subtle);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for (y_px, _) in &layout.y_ticks {
            write!(
                out,
                r#"<line x1="{:.1}" y1="{y_px:.1}" x2="{:.1}" y2="{y_px:.1}" stroke="{grid_hex}" stroke-opacity="0.6" stroke-width="1"/>"#,
                rect.x,
                rect.x + rect.w
            )
            .expect("svg write");
        }
        let max_count = layout.max_count.max(1);
        for bin in &layout.bins {
            let alpha = strength(bin.count, max_count);
            let verts = hex_vertices(bin.center, layout.radius);
            let mut d = String::new();
            for (idx, (vx, vy)) in verts.iter().enumerate() {
                let cmd = if idx == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{vx:.1},{vy:.1} ").expect("svg write");
            }
            d.push('Z');
            write!(
                out,
                r#"<path d="{d}" fill="{ink_hex}" fill-opacity="{alpha:.2}" stroke="{ink_hex}" stroke-opacity="0.45" stroke-width="1"/>"#,
            )
            .expect("svg write");
        }
        for (x_px, val) in &layout.x_ticks {
            write!(
                out,
                r#"<text x="{x_px:.1}" y="{:.1}" text-anchor="middle" fill="{label_hex}" font-size="11">{}</text>"#,
                rect.y + rect.h + 16.0,
                (self.format)(*val)
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

mod layout;
mod render;
#[cfg(test)]
mod tests;
use layout::*;
