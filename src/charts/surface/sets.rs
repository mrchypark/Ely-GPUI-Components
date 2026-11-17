use super::*;

/// Categorical multi-dimension alluvial / parallel sets diagram.
#[derive(IntoElement)]
pub struct ParallelSetsChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) dimensions: Vec<SharedString>,
    pub(super) ribbons: Vec<(Vec<SharedString>, f64)>,
    pub(super) format: Format,
}

impl ParallelSetsChart {
    /// Create a parallel sets diagram across the ordered dimensions.
    pub fn new(
        id: impl Into<ElementId>,
        dimensions: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            dimensions: dimensions.into_iter().map(Into::into).collect(),
            ribbons: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// Add a categorical path across every dimension carrying count.
    pub fn ribbon(
        mut self,
        path_categories: impl IntoIterator<Item = impl Into<SharedString>>,
        count: f64,
    ) -> Self {
        let path: Vec<SharedString> = path_categories.into_iter().map(Into::into).collect();
        assert_eq!(
            path.len(),
            self.dimensions.len(),
            "a parallel sets ribbon needs one category per dimension"
        );
        assert!(
            drawable(count) && count >= 0.0,
            "a parallel sets ribbon needs a non-negative finite count within charts::LIMIT"
        );
        if count > 0.0 {
            self.ribbons.push((path, count));
        }
        self
    }

    /// Format numeric counts in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export the parallel sets paths as CSV.
    pub fn csv(&self) -> String {
        let mut head: Vec<String> = self.dimensions.iter().map(|d| csv_field(d)).collect();
        head.push("count".to_string());
        let mut out = head.join(",");
        for (path, count) in &self.ribbons {
            out.push('\n');
            for cat in path {
                out.push_str(&csv_field(cat));
                out.push(',');
            }
            write!(out, "{count}").expect("csv write");
        }
        out
    }

    /// Export the parallel sets diagram as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.inset),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let bar_w = (pixels(sizes.inset) * 1.4).max(8.0);
        let layout = compute_parallel_sets(&self.dimensions, &self.ribbons, rect, bar_w);
        let (bg_hex, _) = svg_hex(colors.bg);
        let (bar_hex, _) = svg_hex(colors.fg_muted);
        let (label_hex, _) = svg_hex(colors.fg_subtle);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for seg in &layout.segments {
            let (hex, _) = svg_hex(tint(colors, seg.color_ix));
            let mid = (seg.from.0 + seg.to.0) * 0.5;
            write!(
                out,
                r#"<path d="M{:.1},{:.1} C{mid:.1},{:.1} {mid:.1},{:.1} {:.1},{:.1} L{:.1},{:.1} C{mid:.1},{:.1} {mid:.1},{:.1} {:.1},{:.1} Z" fill="{hex}" fill-opacity="0.45"/>"#,
                seg.from.0,
                seg.from.1,
                seg.from.1,
                seg.to.1,
                seg.to.0,
                seg.to.1,
                seg.to.0,
                seg.to.1 + seg.thick,
                seg.to.1 + seg.thick,
                seg.from.1 + seg.thick,
                seg.from.0,
                seg.from.1 + seg.thick,
            )
            .expect("svg write");
        }
        for bar in &layout.bars {
            let (hex, opacity) = if bar.dim == 0 {
                (svg_hex(tint(colors, bar.cat_ix)).0, 0.9)
            } else {
                (bar_hex.clone(), 0.45)
            };
            write!(
                out,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="{hex}" fill-opacity="{opacity}"/>"#,
                bar.rect.x, bar.rect.y, bar.rect.w, bar.rect.h
            )
            .expect("svg write");
        }
        for (dim, x) in layout.dim_xs.iter().enumerate() {
            write!(
                out,
                r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle" fill="{label_hex}" font-size="11">{}</text>"#,
                rect.y + rect.h + 16.0,
                svg_escape(&self.dimensions[dim])
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

#[derive(Clone, Debug)]
pub(super) struct SetBar {
    pub(super) dim: usize,
    pub(super) cat_ix: usize,
    pub(super) name: SharedString,
    pub(super) rect: Rect,
    pub(super) count: f64,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SetSegment {
    pub(super) ribbon_ix: usize,
    pub(super) color_ix: usize,
    pub(super) dim: usize,
    pub(super) from_cat: usize,
    pub(super) to_cat: usize,
    pub(super) from: (f32, f32),
    pub(super) to: (f32, f32),
    pub(super) thick: f32,
}

impl SetSegment {
    pub(super) fn top(&self, t: f32) -> (f32, f32) {
        let mid = (self.from.0 + self.to.0) * 0.5;
        let u = 1.0 - t;
        let x = u * u * u * self.from.0
            + 3.0 * u * u * t * mid
            + 3.0 * u * t * t * mid
            + t * t * t * self.to.0;
        let y = u * u * u * self.from.1
            + 3.0 * u * u * t * self.from.1
            + 3.0 * u * t * t * self.to.1
            + t * t * t * self.to.1;
        (x, y)
    }

    pub(super) fn holds(&self, (x, y): (f32, f32)) -> bool {
        if x < self.from.0 || x > self.to.0 || self.thick <= 0.0 {
            return false;
        }
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for _ in 0..24 {
            let mid = (lo + hi) * 0.5;
            if self.top(mid).0 < x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let top_y = self.top(lo).1;
        y >= top_y && y <= top_y + self.thick
    }
}

pub(super) struct ParallelSetsLayout {
    pub(super) bars: Vec<SetBar>,
    pub(super) segments: Vec<SetSegment>,
    pub(super) ribbon_anchors: Vec<(f32, f32)>,
    pub(super) dim_xs: Vec<f32>,
    pub(super) total: f64,
}

pub(super) fn compute_parallel_sets(
    dimensions: &[SharedString],
    ribbons: &[(Vec<SharedString>, f64)],
    rect: Rect,
    bar_w: f32,
) -> ParallelSetsLayout {
    let d_count = dimensions.len();
    let total: f64 = ribbons.iter().map(|(_, c)| *c).sum();
    if d_count == 0 || ribbons.is_empty() || total <= 0.0 || rect.w <= bar_w || rect.h <= 1.0 {
        return ParallelSetsLayout {
            bars: Vec::new(),
            segments: Vec::new(),
            ribbon_anchors: Vec::new(),
            dim_xs: Vec::new(),
            total,
        };
    }

    // Collect ordered categories and totals per dimension.
    let mut dim_cats: Vec<Vec<(SharedString, f64)>> = vec![Vec::new(); d_count];
    let mut dim_index: Vec<HashMap<SharedString, usize>> = vec![HashMap::new(); d_count];
    let mut ribbon_cats: Vec<Vec<usize>> = Vec::with_capacity(ribbons.len());
    for (path, count) in ribbons {
        let mut row_cats = Vec::with_capacity(d_count);
        for (d, name) in path.iter().enumerate() {
            let cat_ix = if let Some(&ix) = dim_index[d].get(name) {
                dim_cats[d][ix].1 += *count;
                ix
            } else {
                let ix = dim_cats[d].len();
                dim_index[d].insert(name.clone(), ix);
                dim_cats[d].push((name.clone(), *count));
                ix
            };
            row_cats.push(cat_ix);
        }
        ribbon_cats.push(row_cats);
    }

    let max_cats = dim_cats.iter().map(Vec::len).max().unwrap_or(1);
    let gap = if max_cats > 1 {
        8.0_f32.min((rect.h * 0.42) / (max_cats - 1) as f32)
    } else {
        0.0
    };
    let min_avail_h = dim_cats
        .iter()
        .map(|cats| (rect.h - gap * cats.len().saturating_sub(1) as f32).max(1.0))
        .fold(f32::INFINITY, f32::min);
    let height_of = |count: f64| ((count / total) as f32) * min_avail_h;

    let dim_xs: Vec<f32> = (0..d_count)
        .map(|d| {
            if d_count > 1 {
                rect.x + bar_w * 0.5 + (rect.w - bar_w) * d as f32 / (d_count - 1) as f32
            } else {
                rect.x + rect.w * 0.5
            }
        })
        .collect();

    let mut bars = Vec::new();
    let mut bar_lookup: Vec<Vec<Rect>> = vec![Vec::new(); d_count];
    for (d, cats) in dim_cats.iter().enumerate() {
        let col_h: f32 = cats.iter().map(|(_, c)| height_of(*c)).sum::<f32>()
            + gap * cats.len().saturating_sub(1) as f32;
        let mut y = rect.y + (rect.h - col_h) * 0.5;
        let x = dim_xs[d] - bar_w * 0.5;
        for (cat_ix, (name, count)) in cats.iter().enumerate() {
            let h = height_of(*count);
            let r = Rect { x, y, w: bar_w, h };
            bars.push(SetBar {
                dim: d,
                cat_ix,
                name: name.clone(),
                rect: r,
                count: *count,
            });
            bar_lookup[d].push(r);
            y += h + gap;
        }
    }

    // Build cubic S-curve segments between consecutive dimensions.
    let mut segments = Vec::with_capacity(ribbons.len() * d_count.saturating_sub(1));
    let mut ribbon_anchors = vec![(0.0_f32, 0.0_f32); ribbons.len()];
    for d in 0..d_count.saturating_sub(1) {
        let n_from = dim_cats[d].len();
        let n_to = dim_cats[d + 1].len();
        let mut out_offset = vec![0.0_f32; n_from];
        let mut in_offset = vec![0.0_f32; n_to];

        let mut order: Vec<usize> = (0..ribbons.len()).collect();
        order.sort_by_key(|&rix| {
            (
                ribbon_cats[rix][d],
                ribbon_cats[rix][d + 1],
                ribbon_cats[rix][0],
                rix,
            )
        });
        let mut from_ys = vec![0.0_f32; ribbons.len()];
        for &rix in &order {
            let fc = ribbon_cats[rix][d];
            let thick = height_of(ribbons[rix].1);
            from_ys[rix] = bar_lookup[d][fc].y + out_offset[fc];
            out_offset[fc] += thick;
        }

        order.sort_by_key(|&rix| {
            (
                ribbon_cats[rix][d + 1],
                ribbon_cats[rix][d],
                ribbon_cats[rix][0],
                rix,
            )
        });
        let mut to_ys = vec![0.0_f32; ribbons.len()];
        for &rix in &order {
            let tc = ribbon_cats[rix][d + 1];
            let thick = height_of(ribbons[rix].1);
            to_ys[rix] = bar_lookup[d + 1][tc].y + in_offset[tc];
            in_offset[tc] += thick;
        }

        for rix in 0..ribbons.len() {
            let fc = ribbon_cats[rix][d];
            let tc = ribbon_cats[rix][d + 1];
            let thick = height_of(ribbons[rix].1);
            let seg = SetSegment {
                ribbon_ix: rix,
                color_ix: ribbon_cats[rix][0],
                dim: d,
                from_cat: fc,
                to_cat: tc,
                from: (bar_lookup[d][fc].x + bar_w, from_ys[rix]),
                to: (bar_lookup[d + 1][tc].x, to_ys[rix]),
                thick,
            };
            if d == 0 {
                let mid = seg.top(0.5);
                ribbon_anchors[rix] = (mid.0, mid.1 + thick * 0.5);
            }
            segments.push(seg);
        }
    }
    if d_count == 1 {
        for (rix, cats) in ribbon_cats.iter().enumerate() {
            let r = bar_lookup[0][cats[0]];
            ribbon_anchors[rix] = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        }
    }

    ParallelSetsLayout {
        bars,
        segments,
        ribbon_anchors,
        dim_xs,
        total,
    }
}
