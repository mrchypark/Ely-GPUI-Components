use super::*;

#[derive(Clone, Debug)]
pub(super) struct FlatNode {
    pub(super) name: SharedString,
    pub(super) path: Vec<SharedString>,
    pub(super) path_key: String,
    pub(super) depth: usize,
    pub(super) parent: Option<usize>,
    pub(super) children: Vec<usize>,
    pub(super) total_children: usize,
    pub(super) is_collapsed: bool,
    pub(super) branch: usize,
    pub(super) weight: f64,
}

#[derive(Clone, Debug)]
pub(super) struct PlacedNode {
    pub(super) name: SharedString,
    pub(super) path: Vec<SharedString>,
    pub(super) path_key: String,
    pub(super) depth: usize,
    pub(super) parent: Option<usize>,
    pub(super) children: Vec<usize>,
    pub(super) total_children: usize,
    pub(super) is_collapsed: bool,
    pub(super) branch: usize,
    pub(super) weight: f64,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) radius: f32,
    pub(super) theta: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LinkCurve {
    pub(super) parent: usize,
    pub(super) child: usize,
    pub(super) from: (f32, f32),
    pub(super) ctrl1: (f32, f32),
    pub(super) ctrl2: (f32, f32),
    pub(super) to: (f32, f32),
    pub(super) branch: usize,
}

pub(super) struct TreeLayout {
    pub(super) nodes: Vec<PlacedNode>,
    pub(super) links: Vec<LinkCurve>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LabelAlign {
    Start,
    End,
    Center,
}

#[derive(Clone, Debug)]
pub(super) struct PlacedLabel {
    pub(super) text: SharedString,
    pub(super) depth: usize,
    pub(super) ax: f32,
    pub(super) ay: f32,
    pub(super) align: LabelAlign,
    pub(super) rect: (f32, f32, f32, f32),
}

#[derive(Clone, Debug)]
pub(super) struct HitNode {
    pub(super) pos: (f32, f32),
    pub(super) path_key: String,
    pub(super) expandable: bool,
}

pub(super) fn node_path_key(path: &[SharedString]) -> String {
    let mut key = String::new();
    for (ix, seg) in path.iter().enumerate() {
        if ix > 0 {
            key.push('\u{1f}');
        }
        key.push_str(seg.as_ref());
    }
    key
}

pub(super) fn flatten_tree(root: Option<&Slice>, collapsed: &HashSet<String>) -> Vec<FlatNode> {
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

pub(super) fn compute_layout(
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

pub(super) fn select_labels(
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
