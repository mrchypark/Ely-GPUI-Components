use super::*;

/// Compact multi-band horizon chart stacking positive and negative deviations across rows.
#[derive(IntoElement)]
pub struct HorizonChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) labels: Vec<SharedString>,
    pub(super) series: Vec<Series>,
    pub(super) bands: usize,
    pub(super) baseline: f64,
    pub(super) mirror: bool,
    pub(super) format: Format,
}

impl HorizonChart {
    pub fn new(
        id: impl Into<ElementId>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            bands: 3,
            baseline: 0.0,
            mirror: true,
            format: Rc::new(compact),
        }
    }

    /// Append a series row (must provide one value per label).
    pub fn series(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.labels.len(),
            "horizon series {} needs a value per label",
            series.name
        );
        self.series.push(series);
        self
    }

    /// Number of folded amplitude bands per row, clamped to 1..=6 (default 3).
    pub fn bands(mut self, bands: usize) -> Self {
        self.bands = bands.clamp(1, 6);
        self
    }

    /// Reference baseline value (default 0.0).
    pub fn baseline(mut self, baseline: f64) -> Self {
        assert!(
            drawable(baseline),
            "a horizon baseline needs a finite value, within charts::LIMIT"
        );
        self.baseline = baseline;
        self
    }

    /// Mirror negative deviations upward from the row baseline when true (default), or hang downward when false.
    pub fn mode_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    /// Format numeric values in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export series rows as CSV.
    pub fn csv(&self) -> String {
        series_csv(&self.labels, &self.series)
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
        let refs: Vec<&Series> = self.series.iter().collect();
        let layout = compute_horizon(
            &refs,
            (0, self.labels.len().saturating_sub(1)),
            rect,
            self.bands,
            self.baseline,
            self.mirror,
        );
        let bg_hex = svg_hex(colors.bg);
        let pos_hex = svg_hex(colors.chart[0]);
        let neg_hex = svg_hex(colors.danger);
        let grid_hex = svg_hex(colors.border);
        let label_hex = svg_hex(colors.fg_subtle);
        let text_px = pixels(theme.text_size(TextSize::Xs));
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-size="{text_px}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for (ix, row) in layout.rows.iter().enumerate() {
            for patch in &row.patches {
                if patch.points.len() < 3 {
                    continue;
                }
                let mut d = String::new();
                for (p_ix, &(px, py)) in patch.points.iter().enumerate() {
                    let cmd = if p_ix == 0 { 'M' } else { 'L' };
                    write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
                }
                d.push('Z');
                let fill_hex = if patch.positive { &pos_hex } else { &neg_hex };
                let alpha = band_alpha(patch.band, self.bands);
                write!(
                    out,
                    r#"<path d="{d}" fill="{fill_hex}" fill-opacity="{alpha:.2}"/>"#
                )
                .expect("svg write");
            }
            let y_line = row.rect.y + row.rect.h;
            write!(
                out,
                r#"<line x1="{:.1}" y1="{y_line:.1}" x2="{:.1}" y2="{y_line:.1}" stroke="{grid_hex}" stroke-width="1"/>"#,
                row.rect.x,
                row.rect.x + row.rect.w
            )
            .expect("svg write");
            write!(
                out,
                r#"<text x="{:.1}" y="{:.1}" text-anchor="end" fill="{label_hex}">{}</text>"#,
                rect.x - 8.0,
                row.rect.y + row.rect.h * 0.5 + text_px * 0.35,
                svg_escaped(&self.series[ix].name)
            )
            .expect("svg write");
        }
        let every = ((self.labels.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        for (ix, &x_px) in layout.xs.iter().enumerate() {
            if ix % every == 0 {
                write!(
                    out,
                    r#"<text x="{x_px:.1}" y="{:.1}" text-anchor="middle" fill="{label_hex}">{}</text>"#,
                    rect.y + rect.h + text_px * 1.6,
                    svg_escaped(&self.labels[ix])
                )
                .expect("svg write");
            }
        }
        out.push_str("</svg>");
        out
    }
}

impl Styled for HorizonChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
