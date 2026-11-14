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
    base: Div,
    id: ElementId,
    root: Option<Slice>,
    mode: TreeLayoutMode,
    horizontal: bool,
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

struct TreeState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    collapsed: HashSet<String>,
    zoom: f32,
    pan: (f32, f32),
    drag_start: Option<(f32, f32)>,
    drag_last: Option<(f32, f32)>,
    dragged: bool,
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

#[derive(Clone, Debug)]
struct FlatNode {
    name: SharedString,
    path: Vec<SharedString>,
    path_key: String,
    depth: usize,
    parent: Option<usize>,
    children: Vec<usize>,
    total_children: usize,
    is_collapsed: bool,
    branch: usize,
    weight: f64,
}

#[derive(Clone, Debug)]
struct PlacedNode {
    name: SharedString,
    path: Vec<SharedString>,
    path_key: String,
    depth: usize,
    parent: Option<usize>,
    children: Vec<usize>,
    total_children: usize,
    is_collapsed: bool,
    branch: usize,
    weight: f64,
    x: f32,
    y: f32,
    radius: f32,
    theta: f32,
}

#[derive(Clone, Copy, Debug)]
struct LinkCurve {
    parent: usize,
    child: usize,
    from: (f32, f32),
    ctrl1: (f32, f32),
    ctrl2: (f32, f32),
    to: (f32, f32),
    branch: usize,
}

struct TreeLayout {
    nodes: Vec<PlacedNode>,
    links: Vec<LinkCurve>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LabelAlign {
    Start,
    End,
    Center,
}

#[derive(Clone, Debug)]
struct PlacedLabel {
    text: SharedString,
    depth: usize,
    ax: f32,
    ay: f32,
    align: LabelAlign,
    rect: (f32, f32, f32, f32),
}

#[derive(Clone, Debug)]
struct HitNode {
    pos: (f32, f32),
    path_key: String,
    expandable: bool,
}

fn node_path_key(path: &[SharedString]) -> String {
    let mut key = String::new();
    for (ix, seg) in path.iter().enumerate() {
        if ix > 0 {
            key.push('\u{1f}');
        }
        key.push_str(seg.as_ref());
    }
    key
}

fn flatten_tree(root: Option<&Slice>, collapsed: &HashSet<String>) -> Vec<FlatNode> {
    let Some(root) = root else {
        return Vec::new();
    };
    fn walk(
        slice: &Slice,
        depth: usize,
        parent: Option<usize>,
        branch: usize,
        trail: &[SharedString],
        out: &mut Vec<FlatNode>,
        collapsed: &HashSet<String>,
    ) -> usize {
        let ix = out.len();
        let mut path = trail.to_vec();
        path.push(slice.name.clone());
        let path_key = node_path_key(&path);
        let total_children = slice.children.len();
        let is_collapsed = total_children > 0 && collapsed.contains(&path_key);
        out.push(FlatNode {
            name: slice.name.clone(),
            path: path.clone(),
            path_key,
            depth,
            parent,
            children: Vec::new(),
            total_children,
            is_collapsed,
            branch,
            weight: slice.weight(),
        });
        if !is_collapsed {
            let mut child_indices = Vec::with_capacity(total_children);
            for (c_ix, child) in slice.children.iter().enumerate() {
                let child_branch = if ix == 0 { c_ix } else { branch };
                let child_id = walk(
                    child,
                    depth + 1,
                    Some(ix),
                    child_branch,
                    &path,
                    out,
                    collapsed,
                );
                child_indices.push(child_id);
            }
            out[ix].children = child_indices;
        }
        ix
    }
    let mut out = Vec::new();
    walk(root, 0, None, 0, &[], &mut out, collapsed);
    out
}

fn compute_layout(
    root: Option<&Slice>,
    mode: TreeLayoutMode,
    horizontal: bool,
    (width, height): (f32, f32),
    inset: f32,
    label_room: f32,
    collapsed: &HashSet<String>,
) -> TreeLayout {
    let flat = flatten_tree(root, collapsed);
    if flat.is_empty() || width <= 0.0 || height <= 0.0 {
        return TreeLayout {
            nodes: Vec::new(),
            links: Vec::new(),
        };
    }

    let n = flat.len();
    let max_depth = flat.iter().map(|node| node.depth).max().unwrap_or(0);
    let leaf_count = flat
        .iter()
        .filter(|node| node.children.is_empty())
        .count()
        .max(1);

    // Pre-order index visits leaves in strict left-to-right order.
    let mut v_coords = vec![0.0_f32; n];
    let mut leaf_ix = 0usize;
    for (ix, node) in flat.iter().enumerate() {
        if node.children.is_empty() {
            v_coords[ix] = (leaf_ix as f32 + 0.5) / leaf_count as f32;
            leaf_ix += 1;
        }
    }

    let mut heights = vec![0usize; n];
    for ix in (0..n).rev() {
        let children = &flat[ix].children;
        if !children.is_empty() {
            let first = v_coords[children[0]];
            let last = v_coords[children[children.len() - 1]];
            v_coords[ix] = (first + last) * 0.5;
            heights[ix] = 1 + children.iter().map(|&c| heights[c]).max().unwrap_or(0);
        }
    }

    let radial = matches!(
        mode,
        TreeLayoutMode::RadialTree | TreeLayoutMode::RadialCluster
    );
    let cluster = matches!(
        mode,
        TreeLayoutMode::Cluster | TreeLayoutMode::RadialCluster
    );

    let mut u_coords = vec![0.0_f32; n];
    for (ix, node) in flat.iter().enumerate() {
        u_coords[ix] = if max_depth == 0 {
            if radial { 0.0 } else { 0.5 }
        } else if cluster {
            if node.children.is_empty() {
                1.0
            } else {
                max_depth.saturating_sub(heights[ix]) as f32 / max_depth as f32
            }
        } else {
            node.depth as f32 / max_depth as f32
        };
    }

    let mut placed = Vec::with_capacity(n);
    let mut links = Vec::with_capacity(n.saturating_sub(1));

    if radial {
        let center = (width * 0.5, height * 0.5);
        let label_margin = (label_room * 0.95)
            .min(width.min(height) * 0.24)
            .max(inset * 2.0);
        let outer = (center.0.min(center.1) - label_margin).max(0.0);

        for (ix, node) in flat.into_iter().enumerate() {
            let theta = -FRAC_PI_2 + v_coords[ix] * TAU;
            let r = u_coords[ix] * outer;
            let x = center.0 + r * theta.cos();
            let y = center.1 + r * theta.sin();
            let radius = if node.depth == 0 {
                5.0
            } else if node.is_collapsed {
                4.5
            } else if node.children.is_empty() {
                3.5
            } else {
                4.0
            };
            placed.push(PlacedNode {
                name: node.name,
                path: node.path,
                path_key: node.path_key,
                depth: node.depth,
                parent: node.parent,
                children: node.children,
                total_children: node.total_children,
                is_collapsed: node.is_collapsed,
                branch: node.branch,
                weight: node.weight,
                x,
                y,
                radius,
                theta,
            });
        }

        for child_ix in 0..placed.len() {
            let Some(parent_ix) = placed[child_ix].parent else {
                continue;
            };
            let (r0, a0) = (u_coords[parent_ix] * outer, placed[parent_ix].theta);
            let (r1, a1) = (u_coords[child_ix] * outer, placed[child_ix].theta);
            let r_mid = (r0 + r1) * 0.5;
            let ctrl1 = if r0 < 1e-3 {
                (
                    center.0 + r_mid * 0.5 * a1.cos(),
                    center.1 + r_mid * 0.5 * a1.sin(),
                )
            } else {
                (center.0 + r_mid * a0.cos(), center.1 + r_mid * a0.sin())
            };
            let ctrl2 = (center.0 + r_mid * a1.cos(), center.1 + r_mid * a1.sin());
            links.push(LinkCurve {
                parent: parent_ix,
                child: child_ix,
                from: (placed[parent_ix].x, placed[parent_ix].y),
                ctrl1,
                ctrl2,
                to: (placed[child_ix].x, placed[child_ix].y),
                branch: placed[child_ix].branch,
            });
        }
    } else {
        let (pad_x, pad_right, pad_top, pad_bottom) = if horizontal {
            (
                (label_room * 0.85).min(width * 0.22).max(inset * 2.0),
                (label_room * 1.1).min(width * 0.28).max(inset * 2.0),
                (inset * 2.0).min(height * 0.2),
                (inset * 2.0).min(height * 0.2),
            )
        } else {
            let side = (label_room * 0.6).min(width * 0.18).max(inset * 2.0);
            (
                side,
                side,
                (inset * 2.5).min(height * 0.2),
                (inset * 3.5).min(height * 0.25),
            )
        };
        let plot_w = (width - pad_x - pad_right).max(0.0);
        let plot_h = (height - pad_top - pad_bottom).max(0.0);

        for (ix, node) in flat.into_iter().enumerate() {
            let (x, y) = if horizontal {
                (
                    pad_x + u_coords[ix] * plot_w,
                    pad_top + v_coords[ix] * plot_h,
                )
            } else {
                (
                    pad_x + v_coords[ix] * plot_w,
                    pad_top + u_coords[ix] * plot_h,
                )
            };
            let radius = if node.depth == 0 {
                5.0
            } else if node.is_collapsed {
                4.5
            } else if node.children.is_empty() {
                3.5
            } else {
                4.0
            };
            placed.push(PlacedNode {
                name: node.name,
                path: node.path,
                path_key: node.path_key,
                depth: node.depth,
                parent: node.parent,
                children: node.children,
                total_children: node.total_children,
                is_collapsed: node.is_collapsed,
                branch: node.branch,
                weight: node.weight,
                x,
                y,
                radius,
                theta: if horizontal { 0.0 } else { FRAC_PI_2 },
            });
        }

        for child_ix in 0..placed.len() {
            let Some(parent_ix) = placed[child_ix].parent else {
                continue;
            };
            let from = (placed[parent_ix].x, placed[parent_ix].y);
            let to = (placed[child_ix].x, placed[child_ix].y);
            let (ctrl1, ctrl2) = if horizontal {
                let mid_x = (from.0 + to.0) * 0.5;
                ((mid_x, from.1), (mid_x, to.1))
            } else {
                let mid_y = (from.1 + to.1) * 0.5;
                ((from.0, mid_y), (to.0, mid_y))
            };
            links.push(LinkCurve {
                parent: parent_ix,
                child: child_ix,
                from,
                ctrl1,
                ctrl2,
                to,
                branch: placed[child_ix].branch,
            });
        }
    }

    TreeLayout {
        nodes: placed,
        links,
    }
}

fn select_labels(
    nodes: &[PlacedNode],
    radial_mode: bool,
    hover: Option<usize>,
    (width, height): (f32, f32),
    font_px: f32,
    label_room: f32,
) -> Vec<PlacedLabel> {
    if nodes.is_empty() || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    let mut order: Vec<usize> = (0..nodes.len()).collect();
    order.sort_by_key(|&ix| {
        let is_hover = if hover == Some(ix) { 0 } else { 1 };
        let tier = if nodes[ix].depth == 0 {
            0
        } else if nodes[ix].children.is_empty() {
            1
        } else {
            2
        };
        (is_hover, tier, nodes[ix].depth, ix)
    });

    let mut accepted: Vec<PlacedLabel> = Vec::new();
    let text_h = font_px * 1.35;

    for ix in order {
        let node = &nodes[ix];
        let text_w = (node.name.chars().count() as f32 * font_px * 0.58 + 4.0)
            .min(label_room * 1.4)
            .max(12.0);
        let r = node.radius;

        let (ax, ay, align) = if radial_mode {
            if node.depth == 0 {
                (node.x, node.y + r + text_h * 0.6, LabelAlign::Center)
            } else {
                let (cos, sin) = (node.theta.cos(), node.theta.sin());
                let offset = r + 5.0;
                let (ax, ay) = (node.x + offset * cos, node.y + offset * sin);
                if cos > 0.2 {
                    (ax, ay, LabelAlign::Start)
                } else if cos < -0.2 {
                    (ax, ay, LabelAlign::End)
                } else {
                    let y_shift = if sin < 0.0 {
                        ay - text_h * 0.45
                    } else {
                        ay + text_h * 0.45
                    };
                    (ax, y_shift, LabelAlign::Center)
                }
            }
        } else if (node.theta - FRAC_PI_2).abs() < 1e-3 {
            // Vertical orthogonal / cluster
            if node.children.is_empty() {
                (node.x, node.y + r + text_h * 0.65, LabelAlign::Center)
            } else if node.depth == 0 {
                (node.x, node.y - r - text_h * 0.65, LabelAlign::Center)
            } else {
                (node.x + r + 5.0, node.y, LabelAlign::Start)
            }
        } else {
            // Horizontal orthogonal / cluster
            if node.children.is_empty() {
                (node.x + r + 5.0, node.y, LabelAlign::Start)
            } else {
                (node.x - r - 5.0, node.y, LabelAlign::End)
            }
        };

        let rect = match align {
            LabelAlign::Start => (ax, ay - text_h * 0.5, ax + text_w, ay + text_h * 0.5),
            LabelAlign::End => (ax - text_w, ay - text_h * 0.5, ax, ay + text_h * 0.5),
            LabelAlign::Center => (
                ax - text_w * 0.5,
                ay - text_h * 0.5,
                ax + text_w * 0.5,
                ay + text_h * 0.5,
            ),
        };

        if rect.0 < 1.0 || rect.1 < 1.0 || rect.2 > width - 1.0 || rect.3 > height - 1.0 {
            continue;
        }
        let overlaps = accepted.iter().any(|other| {
            let pad = 2.0;
            rect.0 < other.rect.2 + pad
                && rect.2 + pad > other.rect.0
                && rect.1 < other.rect.3 + pad
                && rect.3 + pad > other.rect.1
        });
        if overlaps {
            continue;
        }

        accepted.push(PlacedLabel {
            text: node.name.clone(),
            depth: node.depth,
            ax,
            ay,
            align,
            rect,
        });
    }

    accepted
}

fn svg_hex(color: Hsla) -> (String, f32) {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            byte(rgba.r),
            byte(rgba.g),
            byte(rgba.b)
        ),
        rgba.a,
    )
}

fn svg_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn transform_point(p: (f32, f32), zoom: f32, pan: (f32, f32)) -> (f32, f32) {
    (p.0 * zoom + pan.0, p.1 * zoom + pan.1)
}

fn local_point(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

fn hit_node_index(nodes: &[HitNode], pos: (f32, f32), reach: f32) -> Option<usize> {
    let limit = reach * reach;
    nodes
        .iter()
        .enumerate()
        .map(|(ix, node)| {
            (
                ix,
                (pos.0 - node.pos.0).powi(2) + (pos.1 - node.pos.1).powi(2),
            )
        })
        .filter(|(_, d2)| *d2 <= limit)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(ix, _)| ix)
}

impl Styled for TreeChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for TreeChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<TreeState> =
            window.use_keyed_state((self.id.clone(), "tree"), cx, |_, _| TreeState::default());
        let (bounds, zoom, pan, collapsed, modified) = {
            let s = state.read(cx);
            (
                s.bounds,
                s.zoom,
                s.pan,
                s.collapsed.clone(),
                s.is_modified(),
            )
        };
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let theme = cx.theme();
        let hit_reach = if theme.touch_mode { 24.0 } else { 18.0 };
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let font_px = pixels(theme.text_size(TextSize::Xs));
        let label_room = pixels(sizes.label);
        let mut layout = compute_layout(
            self.root.as_ref(),
            self.mode,
            self.horizontal,
            (width, height),
            pixels(sizes.inset),
            label_room,
            &collapsed,
        );
        for node in &mut layout.nodes {
            (node.x, node.y) = transform_point((node.x, node.y), zoom, pan);
        }
        for link in &mut layout.links {
            link.from = transform_point(link.from, zoom, pan);
            link.ctrl1 = transform_point(link.ctrl1, zoom, pan);
            link.ctrl2 = transform_point(link.ctrl2, zoom, pan);
            link.to = transform_point(link.to, zoom, pan);
        }
        let hover = state.read(cx).pointed(layout.nodes.len());

        let tooltip = hover.map(|ix| {
            let node = &layout.nodes[ix];
            let ink = if node.depth == 0 {
                colors.fg
            } else {
                tint(&colors, node.branch)
            };
            let children_value = if node.is_collapsed {
                format!("{} (collapsed)", node.total_children)
            } else {
                node.total_children.to_string()
            };
            let card = ChartTooltip::new(node.path.join(" › "))
                .row(Some(ink), "Weight", compact(node.weight))
                .row(None, "Children", children_value);
            anchored((node.x, node.y), node.x > width * 0.55, card)
        });

        let radial = matches!(
            self.mode,
            TreeLayoutMode::RadialTree | TreeLayoutMode::RadialCluster
        );
        let (fg, fg_muted) = (colors.fg, colors.fg_muted);
        let labels: Vec<Div> = select_labels(
            &layout.nodes,
            radial,
            hover,
            (width, height),
            font_px,
            label_room,
        )
        .into_iter()
        .map(|lbl| {
            let word = div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if lbl.depth == 0 { fg } else { fg_muted })
                .child(lbl.text);
            let anchor = div()
                .absolute()
                .left(Pixels::from(lbl.ax))
                .top(Pixels::from(lbl.ay))
                .h_0()
                .flex()
                .items_center();
            match lbl.align {
                LabelAlign::Start => anchor.child(word),
                LabelAlign::End => anchor.w_0().justify_end().child(word),
                LabelAlign::Center => anchor.w_0().justify_center().child(word),
            }
        })
        .collect();

        let hit_nodes: Rc<Vec<HitNode>> = Rc::new(
            layout
                .nodes
                .iter()
                .map(|n| HitNode {
                    pos: (n.x, n.y),
                    path_key: n.path_key.clone(),
                    expandable: n.total_children > 0,
                })
                .collect(),
        );
        let paint_nodes = layout.nodes;
        let paint_links = layout.links;
        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);

        let reset_button = modified.then(|| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "reset"), "Reset")
                .debug_selector(|| "tree-reset".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    reset.update(cx, |s, cx| {
                        s.reset();
                        cx.notify();
                    });
                });
            div().absolute().top_0().right_0().child(button)
        });

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let (pressed, moved, released, released_out, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (down_hits, move_hits, up_hits) = (hit_nodes.clone(), hit_nodes.clone(), hit_nodes);

        let viewport = div()
            .id((self.id.clone(), "tree"))
            .relative()
            .size_full()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_point(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |s, cx| {
                    if event.click_count >= 2 {
                        s.reset();
                        s.hover = None;
                        cx.notify();
                        return;
                    }
                    s.drag_start = Some(pos);
                    s.drag_last = Some(pos);
                    s.dragged = false;
                    s.hover = hit_node_index(&down_hits, pos, hit_reach);
                    cx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_point(event.position, moved.read(cx).bounds);
                moved.update(cx, |s, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let (Some(start), Some(last)) = (s.drag_start, s.drag_last)
                    {
                        let dist = (pos.0 - start.0).hypot(pos.1 - start.1);
                        if s.dragged || dist > 4.0 {
                            s.pan.0 += pos.0 - last.0;
                            s.pan.1 += pos.1 - last.1;
                            s.drag_last = Some(pos);
                            if !s.dragged {
                                s.dragged = true;
                                s.hover = None;
                            }
                            cx.notify();
                        }
                        return;
                    }
                    let next = hit_node_index(&move_hits, pos, hit_reach);
                    if s.hover != next {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let pos = local_point(event.position, released.read(cx).bounds);
                released.update(cx, |s, cx| {
                    let was_dragged = s.dragged;
                    let had_press = s.drag_start.take().is_some();
                    s.drag_last = None;
                    s.dragged = false;
                    if had_press
                        && !was_dragged
                        && event.click_count < 2
                        && let Some(ix) = hit_node_index(&up_hits, pos, hit_reach)
                        && let Some(node) = up_hits.get(ix)
                        && node.expandable
                    {
                        if !s.collapsed.remove(&node.path_key) {
                            s.collapsed.insert(node.path_key.clone());
                        }
                        s.hover = Some(ix);
                        cx.notify();
                    } else if had_press {
                        cx.notify();
                    }
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                released_out.update(cx, |s, cx| {
                    if s.drag_start.take().is_some() || s.dragged {
                        s.drag_last = None;
                        s.dragged = false;
                        cx.notify();
                    }
                });
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |s, cx| {
                        if s.hover.take().is_some() {
                            cx.notify();
                        }
                    });
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if ev.source == ScrollSource::Touch {
                    if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                        scrolled.update(cx, |s, cx| {
                            s.pan.0 += dx;
                            s.pan.1 += dy;
                            s.hover = None;
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                } else if dy.abs() >= 0.01 {
                    let pointer = local_point(ev.position, scrolled.read(cx).bounds);
                    let scale = (dy * 0.002).exp().clamp(0.1, 10.0);
                    scrolled.update(cx, |s, cx| {
                        s.zoom_at(scale, pointer);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                let pointer = local_point(ev.position, pinched.read(cx).bounds);
                pinched.update(cx, |s, cx| {
                    s.drag_start = None;
                    s.drag_last = None;
                    s.dragged = false;
                    if ev.phase != TouchPhase::Cancelled && ev.delta.is_finite() {
                        let scale = ev.delta.exp().clamp(0.1, 10.0);
                        if (scale - 1.0).abs() > 1e-4 {
                            s.zoom_at(scale, pointer);
                            cx.notify();
                        }
                    }
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for link in &paint_links {
                            let active = hover == Some(link.parent) || hover == Some(link.child);
                            let link_w = if active { stroke * 0.85 } else { hairline };
                            let mut path = PathBuilder::stroke(link_w);
                            path.move_to(at(origin, link.from));
                            path.cubic_bezier_to(
                                at(origin, link.to),
                                at(origin, link.ctrl1),
                                at(origin, link.ctrl2),
                            );
                            let color = if active {
                                tint(&colors, link.branch)
                            } else if hover.is_some() {
                                colors.border.opacity(0.45)
                            } else {
                                colors.border_strong.opacity(0.75)
                            };
                            finish(path, color, window);
                        }
                        for (ix, node) in paint_nodes.iter().enumerate() {
                            let lit = hover == Some(ix);
                            let ink = if node.depth == 0 {
                                colors.fg
                            } else {
                                tint(&colors, node.branch)
                            };
                            let r = Pixels::from(if lit { node.radius + 1.5 } else { node.radius });
                            let center = at(origin, (node.x, node.y));
                            let edge = if lit { colors.fg } else { ink };
                            if node.is_collapsed {
                                let outer_w = if lit {
                                    stroke
                                } else {
                                    stroke.max(hairline * 2.5)
                                };
                                ring(
                                    center,
                                    r + Pixels::from(0.75),
                                    (colors.bg, edge),
                                    outer_w,
                                    window,
                                );
                                ring(center, r * 0.48, (ink, ink), Pixels::ZERO, window);
                            } else {
                                let inside = if node.total_children == 0 {
                                    ink
                                } else {
                                    colors.bg
                                };
                                ring(
                                    center,
                                    r,
                                    (inside, edge),
                                    if lit { stroke * 0.85 } else { hairline * 1.5 },
                                    window,
                                );
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels);

        root.child(viewport)
            .children(tooltip)
            .children(reset_button)
            .child(measure(state, |s| &mut s.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> Slice {
        Slice::new("Root", 0.0).children([
            Slice::new("Alpha", 10.0),
            Slice::new("Beta", 0.0).children([
                Slice::new("B1", 6.0),
                Slice::new("B2", 0.0).children([Slice::new("B2a", 4.0)]),
            ]),
        ])
    }

    #[test]
    fn tree_depths_and_cluster_leaves_match_spec() {
        let tree = sample_tree();

        // Orthogonal Tree: Alpha (depth 1) sits at 1/3, B2a (depth 3) sits at 1.0.
        let ortho = compute_layout(
            Some(&tree),
            TreeLayoutMode::Orthogonal,
            true,
            (400.0, 240.0),
            8.0,
            64.0,
            &HashSet::new(),
        );
        let alpha = ortho
            .nodes
            .iter()
            .find(|n| n.name == "Alpha")
            .expect("Alpha");
        let b2 = ortho.nodes.iter().find(|n| n.name == "B2").expect("B2");
        let b2a = ortho.nodes.iter().find(|n| n.name == "B2a").expect("B2a");
        assert!(alpha.x < b2.x && b2.x < b2a.x);

        // Cluster dendrogram: ALL leaves (Alpha, B1, B2a) share the rightmost x coordinate.
        let cluster = compute_layout(
            Some(&tree),
            TreeLayoutMode::Cluster,
            true,
            (400.0, 240.0),
            8.0,
            64.0,
            &HashSet::new(),
        );
        let c_alpha = cluster
            .nodes
            .iter()
            .find(|n| n.name == "Alpha")
            .expect("Alpha");
        let c_b1 = cluster.nodes.iter().find(|n| n.name == "B1").expect("B1");
        let c_b2a = cluster.nodes.iter().find(|n| n.name == "B2a").expect("B2a");
        assert!((c_alpha.x - c_b1.x).abs() < 1e-4);
        assert!((c_b1.x - c_b2a.x).abs() < 1e-4);

        // RadialCluster: ALL leaves lie on the same outermost radius.
        let radial_c = compute_layout(
            Some(&tree),
            TreeLayoutMode::RadialCluster,
            true,
            (300.0, 300.0),
            8.0,
            64.0,
            &HashSet::new(),
        );
        let r_alpha = radial_c
            .nodes
            .iter()
            .find(|n| n.name == "Alpha")
            .expect("Alpha");
        let r_b2a = radial_c
            .nodes
            .iter()
            .find(|n| n.name == "B2a")
            .expect("B2a");
        let dist = |n: &PlacedNode| (n.x - 150.0).hypot(n.y - 150.0);
        assert!((dist(r_alpha) - dist(r_b2a)).abs() < 1e-3);
    }

    #[test]
    fn csv_outputs_depth_path_and_weight() {
        let chart = TreeChart::new("org", sample_tree());
        let csv = chart.csv();
        assert_eq!(
            csv,
            "depth,path,value\n0,Root,20\n1,Root › Alpha,10\n1,Root › Beta,10\n2,Root › Beta › B1,6\n2,Root › Beta › B2,4\n3,Root › Beta › B2 › B2a,4"
        );
    }

    #[test]
    fn collapse_hides_descendants_and_zoom_at_anchors_pointer() {
        let tree = sample_tree();
        let mut collapsed = HashSet::new();
        collapsed.insert("Root\u{1f}Beta".to_string());
        let layout = compute_layout(
            Some(&tree),
            TreeLayoutMode::Orthogonal,
            true,
            (400.0, 240.0),
            8.0,
            64.0,
            &collapsed,
        );
        assert_eq!(layout.nodes.len(), 3);
        let beta = layout
            .nodes
            .iter()
            .find(|n| n.name == "Beta")
            .expect("Beta");
        assert!(beta.is_collapsed);
        assert!(beta.children.is_empty());
        assert_eq!(beta.total_children, 2);
        assert_eq!(beta.weight, 10.0);

        let mut state = TreeState::default();
        state.zoom_at(2.0, (120.0, 80.0));
        assert!((state.zoom - 2.0).abs() < 1e-6);
        assert!((state.pan.0 - (-120.0)).abs() < 1e-6);
        assert!((state.pan.1 - (-80.0)).abs() < 1e-6);
        let anchored_pt = transform_point((120.0, 80.0), state.zoom, state.pan);
        assert!((anchored_pt.0 - 120.0).abs() < 1e-4);
        assert!((anchored_pt.1 - 80.0).abs() < 1e-4);
        state.zoom_at(10.0, (120.0, 80.0));
        assert!((state.zoom - 6.0).abs() < 1e-6);
        state.reset();
        assert!(!state.is_modified());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn tree_chart_renders_exports_and_opens_tooltip_on_hover(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, Window, px};

        struct TreeHarness(TreeLayoutMode);
        impl Render for TreeHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(400.0)).h(px(280.0)).child(
                    TreeChart::new("tree-test", sample_tree())
                        .mode(self.0)
                        .horizontal(true),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            for mode in [
                TreeLayoutMode::Orthogonal,
                TreeLayoutMode::Cluster,
                TreeLayoutMode::RadialTree,
                TreeLayoutMode::RadialCluster,
            ] {
                let chart = TreeChart::new("export", sample_tree()).mode(mode);
                let svg = chart.svg(360.0, 260.0, cx);
                assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
                assert_eq!(svg.matches("<path").count(), 5);
                assert_eq!(svg.matches("<circle").count(), 6);
            }
        });

        let (_, vc) = cx.add_window_view(|_, _| TreeHarness(TreeLayoutMode::RadialTree));
        settle(vc);
        let root = vc.debug_bounds("chart-root").expect("tree root bounds");
        // In RadialTree, the root node sits directly at the center of chart-root.
        vc.simulate_mouse_move(root.center(), None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn tree_chart_collapses_expands_pans_zooms_and_resets(cx: &mut gpui::TestAppContext) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, ScrollDelta, Window, point, px};

        struct TreeHarness;
        impl Render for TreeHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(400.0)).h(px(280.0)).child(
                    TreeChart::new("tree-interact", sample_tree())
                        .mode(TreeLayoutMode::Orthogonal)
                        .horizontal(true)
                        .h(px(280.0)),
                )
            }
        }

        cx.update(Theme::init);
        let (_, vc) = cx.add_window_view(|_, _| TreeHarness);
        settle(vc);

        let root = vc.debug_bounds("chart-root").expect("tree root bounds");
        assert!(vc.debug_bounds("tree-reset").is_none());

        let (inset, label_room) = vc.update(|window, cx| {
            let theme = cx.theme();
            let sizes = theme.chart();
            let rem = window.rem_size();
            (
                f32::from(sizes.inset.to_pixels(rem)),
                f32::from(sizes.label.to_pixels(rem)),
            )
        });
        let layout = compute_layout(
            Some(&sample_tree()),
            TreeLayoutMode::Orthogonal,
            true,
            (f32::from(root.size.width), f32::from(root.size.height)),
            inset,
            label_room,
            &HashSet::new(),
        );
        let root_node = layout
            .nodes
            .iter()
            .find(|n| n.name == "Root")
            .expect("Root");
        let beta = layout
            .nodes
            .iter()
            .find(|n| n.name == "Beta")
            .expect("Beta");
        let b2a = layout.nodes.iter().find(|n| n.name == "B2a").expect("B2a");

        let at_node = |n: &PlacedNode| root.origin + point(px(n.x), px(n.y));

        // Clicking internal node Beta collapses its descendants.
        let beta_pt = at_node(beta);
        vc.simulate_mouse_down(beta_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(beta_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_some());

        // Hidden descendant B2a is no longer hit-testable when Beta is collapsed.
        vc.simulate_mouse_move(at_node(b2a), None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_none());

        // Clicking Beta again expands its descendants using its collapsed layout coordinates.
        let mut collapsed = HashSet::new();
        collapsed.insert(beta.path_key.clone());
        let collapsed_layout = compute_layout(
            Some(&sample_tree()),
            TreeLayoutMode::Orthogonal,
            true,
            (f32::from(root.size.width), f32::from(root.size.height)),
            inset,
            label_room,
            &collapsed,
        );
        let collapsed_beta = collapsed_layout
            .nodes
            .iter()
            .find(|n| n.name == "Beta")
            .expect("collapsed Beta");
        let collapsed_beta_pt = at_node(collapsed_beta);
        vc.simulate_mouse_down(collapsed_beta_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(collapsed_beta_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_none());

        // B2a is visible and interactive again.
        vc.simulate_mouse_move(at_node(b2a), None, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        // Wheel scroll zooms around pointer and shows Reset button.
        let zoom_center = root.center();
        vc.simulate_event(ScrollEvent {
            position: zoom_center,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Pointer,
        });
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_some());

        // Dragging > 4px pans without toggling Beta even if drag starts on a node.
        let scale = (-120.0_f32 * 0.002).exp().clamp(0.1, 10.0);
        let cx_local = f32::from(zoom_center.x - root.origin.x);
        let cy_local = f32::from(zoom_center.y - root.origin.y);
        let zoom_pan = (cx_local * (1.0 - scale), cy_local * (1.0 - scale));
        let beta_zoomed = transform_point((beta.x, beta.y), scale, zoom_pan);
        let beta_zoomed_pt = root.origin + point(px(beta_zoomed.0), px(beta_zoomed.1));
        vc.simulate_mouse_down(beta_zoomed_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_move(
            beta_zoomed_pt + point(px(16.0), px(10.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            beta_zoomed_pt + point(px(16.0), px(10.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(vc);

        // Touch scroll also pans.
        vc.simulate_event(ScrollEvent {
            position: zoom_center,
            delta: ScrollDelta::Pixels(point(px(8.0), px(-6.0))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
            source: ScrollSource::Touch,
        });
        settle(vc);

        // Verify Root moved to its zoomed + panned screen position and B2a stayed expanded.
        let expected_pan = (zoom_pan.0 + 16.0 + 8.0, zoom_pan.1 + 10.0 - 6.0);
        let root_moved = transform_point((root_node.x, root_node.y), scale, expected_pan);
        vc.simulate_mouse_move(
            root.origin + point(px(root_moved.0), px(root_moved.1)),
            None,
            Modifiers::none(),
        );
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());
        let b2a_moved = transform_point((b2a.x, b2a.y), scale, expected_pan);
        vc.simulate_mouse_move(
            root.origin + point(px(b2a_moved.0), px(b2a_moved.1)),
            None,
            Modifiers::none(),
        );
        settle(vc);
        assert!(vc.debug_bounds("chart-tooltip").is_some());

        // Clicking Reset restores pan, zoom, and expanded state.
        let reset_btn = vc.debug_bounds("tree-reset").expect("reset button bounds");
        vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_none());

        // Pinch zooms, and double-click resets.
        vc.simulate_event(PinchEvent {
            position: zoom_center,
            delta: 0.35,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_some());

        vc.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: zoom_center,
            modifiers: Modifiers::none(),
            click_count: 2,
            first_mouse: false,
        });
        vc.simulate_event(MouseUpEvent {
            button: MouseButton::Left,
            position: zoom_center,
            modifiers: Modifiers::none(),
            click_count: 2,
        });
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_none());
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn tree_chart_touch_mode_uses_24px_hit_radius_clears_drag_on_pinch_and_flips_tooltip(
        cx: &mut gpui::TestAppContext,
    ) {
        use crate::charts::tests::settle;
        use crate::theme::Theme;
        use gpui::{Context, Modifiers, Render, Window, point, px, size};

        struct TreeTouchHarness;
        impl Render for TreeTouchHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(260.0)).h(px(260.0)).child(
                    TreeChart::new("tree-touch", sample_tree())
                        .mode(TreeLayoutMode::Orthogonal)
                        .horizontal(true)
                        .h(px(260.0)),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });
        let (_, vc) = cx.add_window_view(|_, _| TreeTouchHarness);
        vc.simulate_resize(size(px(260.0), px(260.0)));
        settle(vc);

        let root = vc.debug_bounds("chart-root").expect("tree root bounds");
        let (inset, label_room) = vc.update(|window, cx| {
            let sizes = cx.theme().chart();
            let rem = window.rem_size();
            (
                f32::from(sizes.inset.to_pixels(rem)),
                f32::from(sizes.label.to_pixels(rem)),
            )
        });
        let layout = compute_layout(
            Some(&sample_tree()),
            TreeLayoutMode::Orthogonal,
            true,
            (f32::from(root.size.width), f32::from(root.size.height)),
            inset,
            label_room,
            &HashSet::new(),
        );
        let beta = layout
            .nodes
            .iter()
            .find(|n| n.name == "Beta")
            .expect("Beta");
        let b2a = layout.nodes.iter().find(|n| n.name == "B2a").expect("B2a");

        // Tapping 20px away from Beta hits it in touch_mode (>= 24px) and collapses its branch.
        let near_beta = root.origin + point(px(beta.x), px(beta.y + 20.0));
        vc.simulate_mouse_down(near_beta, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(near_beta, MouseButton::Left, Modifiers::none());
        settle(vc);
        let reset_btn = vc
            .debug_bounds("tree-reset")
            .expect("tree reset after collapse");
        vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_none());

        // Pinching after mouse_down clears drag_start/drag_last so move does not pan or toggle Beta.
        let beta_pt = root.origin + point(px(beta.x), px(beta.y));
        vc.simulate_mouse_down(beta_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_event(PinchEvent {
            position: root.center(),
            delta: 0.35,
            phase: TouchPhase::Moved,
            modifiers: Modifiers::none(),
        });
        vc.simulate_mouse_move(
            beta_pt + point(px(40.0), px(20.0)),
            Some(MouseButton::Left),
            Modifiers::none(),
        );
        vc.simulate_mouse_up(
            beta_pt + point(px(40.0), px(20.0)),
            MouseButton::Left,
            Modifiers::none(),
        );
        settle(vc);
        let reset_btn = vc
            .debug_bounds("tree-reset")
            .expect("tree reset after pinch");

        // B2a (> 0.55 * width) stayed expanded and its zoomed tooltip flips to the left.
        let scale = (0.35_f32).exp().clamp(0.1, 10.0);
        let c_local = (
            f32::from(root.size.width) * 0.5,
            f32::from(root.size.height) * 0.5,
        );
        let zoom_pan = (c_local.0 * (1.0 - scale), c_local.1 * (1.0 - scale));
        let b2a_zoomed = transform_point((b2a.x, b2a.y), scale, zoom_pan);
        let b2a_pt = root.origin + point(px(b2a_zoomed.0), px(b2a_zoomed.1));
        vc.simulate_mouse_down(b2a_pt, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(b2a_pt, MouseButton::Left, Modifiers::none());
        settle(vc);
        let tip = vc.debug_bounds("chart-tooltip").expect("B2a tooltip");
        assert!(tip.center().x < b2a_pt.x);

        vc.simulate_mouse_down(reset_btn.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(reset_btn.center(), MouseButton::Left, Modifiers::none());
        settle(vc);
        assert!(vc.debug_bounds("tree-reset").is_none());
    }
}
