use std::{
    collections::HashSet,
    f32::consts::{FRAC_PI_2, TAU},
    fmt::Write as _,
    rc::Rc,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Point, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, TouchPhase, Window, canvas, div,
};

use super::{
    axes::anchored,
    paint::{at, finish, measure, ring, tint},
    parts::ChartTooltip,
    scale::compact,
    sunburst::Slice,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// Layout variant for [`TreeChart`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeLayoutMode {
    /// Tidy orthogonal tree where each node sits at `depth / max_depth`.
    #[default]
    Orthogonal,
    /// Orthogonal dendrogram where every leaf sits at `max_depth`.
    Cluster,
    /// Radial tidy tree with root at center and depth mapped to radius.
    RadialTree,
    /// Radial dendrogram where every leaf sits on the outermost ring.
    RadialCluster,
}

/// Tidy hierarchical tree and dendrogram chart over [`Slice`] nodes.
#[derive(IntoElement)]
pub struct TreeChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) root: Option<Slice>,
    pub(super) mode: TreeLayoutMode,
    pub(super) horizontal: bool,
}

impl TreeChart {
    /// Create a tree chart rooted at `root_slice`.
    pub fn new(id: impl Into<ElementId>, root_slice: Slice) -> Self {
        Self {
            base: div(),
            id: id.into(),
            root: Some(root_slice),
            mode: TreeLayoutMode::Orthogonal,
            horizontal: true,
        }
    }

    /// Create a tree chart from one or more top-level slices.
    pub fn from_slices(id: impl Into<ElementId>, slices: impl IntoIterator<Item = Slice>) -> Self {
        let mut items: Vec<Slice> = slices.into_iter().collect();
        let root = match items.len() {
            0 => None,
            1 => Some(items.pop().expect("one slice")),
            _ => Some(Slice::new("Root", 0.0).children(items)),
        };
        Self {
            base: div(),
            id: id.into(),
            root,
            mode: TreeLayoutMode::Orthogonal,
            horizontal: true,
        }
    }

    /// Set the tree layout mode (orthogonal, cluster dendrogram, or radial variants).
    pub fn mode(mut self, mode: TreeLayoutMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set orientation for orthogonal/cluster modes (`true` = left-to-right, `false` = top-to-bottom).
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Export the tree nodes as CSV (`depth,path,value`).
    pub fn csv(&self) -> String {
        let mut out = String::from("depth,path,value");
        let flat = flatten_tree(self.root.as_ref(), &HashSet::new());
        for node in &flat {
            let path = csv_cell(&node.path.join(" › "));
            write!(out, "\n{},{},{}", node.depth, path, node.weight).expect("csv write");
        }
        out
    }

    /// Export the chart as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (colors, sizes, rem) = (&theme.colors, theme.chart(), theme.base_rem());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let empty = HashSet::new();
        let layout = compute_layout(
            self.root.as_ref(),
            self.mode,
            self.horizontal,
            (width, height),
            pixels(sizes.inset),
            pixels(sizes.label),
            &empty,
        );
        let (bg_hex, _) = svg_hex(colors.bg);
        let (edge_hex, _) = svg_hex(colors.border_strong);
        let (fg_hex, _) = svg_hex(colors.fg);
        let (label_hex, _) = svg_hex(colors.fg_muted);
        let font_px = pixels(theme.text_size(TextSize::Xs));
        let stroke_w = f32::from(sizes.hairline).max(1.0);

        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="{}" font-size="{font_px:.1}">"#,
            svg_escape(&theme.font_family),
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");

        for link in &layout.links {
            write!(
                out,
                r#"<path d="M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{edge_hex}" stroke-opacity="0.7" stroke-width="{stroke_w:.1}"/>"#,
                link.from.0,
                link.from.1,
                link.ctrl1.0,
                link.ctrl1.1,
                link.ctrl2.0,
                link.ctrl2.1,
                link.to.0,
                link.to.1,
            )
            .expect("svg write");
        }

        for node in &layout.nodes {
            let ink = if node.depth == 0 {
                colors.fg
            } else {
                tint(colors, node.branch)
            };
            let (ink_hex, _) = svg_hex(ink);
            let fill_hex = if node.children.is_empty() {
                &ink_hex
            } else {
                &bg_hex
            };
            write!(
                out,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{fill_hex}" stroke="{ink_hex}" stroke-width="1.5"/>"#,
                node.x, node.y, node.radius,
            )
            .expect("svg write");
        }

        let radial = matches!(
            self.mode,
            TreeLayoutMode::RadialTree | TreeLayoutMode::RadialCluster
        );
        for lbl in select_labels(
            &layout.nodes,
            radial,
            None,
            (width, height),
            font_px,
            pixels(sizes.label),
        ) {
            let anchor = match lbl.align {
                LabelAlign::Start => "start",
                LabelAlign::End => "end",
                LabelAlign::Center => "middle",
            };
            let fill = if lbl.depth == 0 { &fg_hex } else { &label_hex };
            write!(
                out,
                r#"<text x="{:.1}" y="{:.1}" text-anchor="{anchor}" fill="{fill}">{}</text>"#,
                lbl.ax,
                lbl.ay + font_px * 0.35,
                svg_escape(&lbl.text),
            )
            .expect("svg write");
        }

        out.push_str("</svg>");
        out
    }
}

pub(super) struct TreeState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) collapsed: HashSet<String>,
    pub(super) zoom: f32,
    pub(super) pan: (f32, f32),
    pub(super) drag_start: Option<(f32, f32)>,
    pub(super) drag_last: Option<(f32, f32)>,
    pub(super) dragged: bool,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            bounds: Bounds::default(),
            hover: None,
            collapsed: HashSet::new(),
            zoom: 1.0,
            pan: (0.0, 0.0),
            drag_start: None,
            drag_last: None,
            dragged: false,
        }
    }
}

impl TreeState {
    fn pointed(&self, count: usize) -> Option<usize> {
        self.hover.filter(|ix| *ix < count)
    }

    fn is_modified(&self) -> bool {
        (self.zoom - 1.0).abs() > 1e-6
            || self.pan.0.abs() > 1e-6
            || self.pan.1.abs() > 1e-6
            || !self.collapsed.is_empty()
    }

    fn reset(&mut self) {
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.collapsed.clear();
        self.drag_start = None;
        self.drag_last = None;
        self.dragged = false;
    }

    fn zoom_at(&mut self, scale: f32, pointer: (f32, f32)) {
        if !scale.is_finite() || scale <= 0.0 || !pointer.0.is_finite() || !pointer.1.is_finite() {
            return;
        }
        let next_zoom = (self.zoom * scale).clamp(0.5, 6.0);
        let ratio = next_zoom / self.zoom;
        if (ratio - 1.0).abs() <= 1e-6 {
            return;
        }
        self.pan.0 = pointer.0 - (pointer.0 - self.pan.0) * ratio;
        self.pan.1 = pointer.1 - (pointer.1 - self.pan.1) * ratio;
        self.zoom = next_zoom;
    }
}

mod layout;
mod render;
#[cfg(test)]
mod tests;
use layout::*;
use render::*;
