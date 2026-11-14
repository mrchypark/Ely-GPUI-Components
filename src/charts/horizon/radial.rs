use super::*;

/// Polar coordinate cyclic line and area chart with optional min/max envelope band.
#[derive(IntoElement)]
pub struct RadialLineChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) labels: Vec<SharedString>,
    pub(super) series: Vec<Series>,
    pub(super) range_band: Option<(Vec<f64>, Vec<f64>)>,
    pub(super) filled: bool,
    pub(super) format: Format,
}

impl RadialLineChart {
    pub fn new(
        id: impl Into<ElementId>,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            base: div(),
            id: id.into(),
            labels: labels.into_iter().map(Into::into).collect(),
            series: Vec::new(),
            range_band: None,
            filled: false,
            format: Rc::new(compact),
        }
    }

    /// Append a cyclic data series (must provide one value per label).
    pub fn series(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.labels.len(),
            "radial series {} needs a value per label",
            series.name
        );
        self.series.push(series);
        self
    }

    /// Optional min/max envelope band around the cycle.
    pub fn range_band(
        mut self,
        low_values: impl IntoIterator<Item = f64>,
        high_values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let low: Vec<f64> = low_values.into_iter().collect();
        let high: Vec<f64> = high_values.into_iter().collect();
        assert_eq!(
            low.len(),
            self.labels.len(),
            "radial range_band low values need one entry per label"
        );
        assert_eq!(
            high.len(),
            self.labels.len(),
            "radial range_band high values need one entry per label"
        );
        assert!(
            low.iter().chain(&high).all(|v| drawable(*v)),
            "radial range_band values must be finite, within charts::LIMIT"
        );
        self.range_band = Some((low, high));
        self
    }

    /// Fill each series loop down to the inner ring when true.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Format numeric values on rings and in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export series (and range band, if present) as CSV.
    pub fn csv(&self) -> String {
        if let Some((low, high)) = &self.range_band {
            let mut all = self.series.clone();
            all.push(Series::new("Low", low.iter().copied()));
            all.push(Series::new("High", high.iter().copied()));
            series_csv(&self.labels, &all)
        } else {
            series_csv(&self.labels, &self.series)
        }
    }

    /// Export the chart as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let room = pixels(sizes.foot);
        let center = (width * 0.5, height * 0.5);
        let outer = (width.min(height) * 0.5 - room).max(1.0);
        let refs: Vec<&Series> = self.series.iter().collect();
        let layout = compute_radial(
            self.labels.len(),
            &refs,
            self.range_band.as_ref(),
            center,
            outer,
            0.0,
        );
        let bg_hex = svg_hex(colors.bg);
        let grid_hex = svg_hex(colors.border);
        let label_hex = svg_hex(colors.fg_subtle);
        let text_px = pixels(theme.text_size(TextSize::Xs));
        let stroke_px = pixels(sizes.stroke);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-size="{text_px}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for &(r, _) in &layout.rings {
            write!(
                out,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{r:.1}" fill="none" stroke="{grid_hex}" stroke-opacity="0.6" stroke-width="1"/>"#,
                center.0, center.1
            )
            .expect("svg write");
        }
        let n = self.labels.len();
        let every = if n == 0 {
            1
        } else {
            ((n as f32 * pixels(sizes.label) / (TAU * outer).max(1.0)).ceil() as usize).max(1)
        };
        for ix in (0..n).step_by(every) {
            let a = radial_angle(ix, n, 0.0);
            let p0 = (
                center.0 + layout.inner * a.cos(),
                center.1 + layout.inner * a.sin(),
            );
            let p1 = (center.0 + outer * a.cos(), center.1 + outer * a.sin());
            write!(
                out,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{grid_hex}" stroke-opacity="0.5" stroke-width="1"/>"#,
                p0.0, p0.1, p1.0, p1.1
            )
            .expect("svg write");
            let lx = center.0 + (outer + 10.0) * a.cos();
            let ly = center.1 + (outer + 10.0) * a.sin() + text_px * 0.35;
            let anchor = if a.cos() > 0.3 {
                "start"
            } else if a.cos() < -0.3 {
                "end"
            } else {
                "middle"
            };
            write!(
                out,
                r#"<text x="{lx:.1}" y="{ly:.1}" text-anchor="{anchor}" fill="{label_hex}">{}</text>"#,
                svg_escaped(&self.labels[ix])
            )
            .expect("svg write");
        }
        if let Some((ref low_pts, ref high_pts)) = layout.band
            && low_pts.len() >= 2
        {
            let band_hex = svg_hex(tint(colors, 0));
            let mut d = String::new();
            for (ix, &(px, py)) in high_pts.iter().enumerate() {
                let cmd = if ix == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
            }
            write!(d, "L{:.1},{:.1} ", high_pts[0].0, high_pts[0].1).expect("svg write");
            write!(d, "L{:.1},{:.1} ", low_pts[0].0, low_pts[0].1).expect("svg write");
            for &(px, py) in low_pts.iter().rev() {
                write!(d, "L{px:.1},{py:.1} ").expect("svg write");
            }
            d.push('Z');
            write!(
                out,
                r#"<path d="{d}" fill="{band_hex}" fill-opacity="0.18"/>"#
            )
            .expect("svg write");
        }
        for (s_ix, pts) in layout.series_pts.iter().enumerate() {
            if pts.len() < 2 {
                continue;
            }
            let hex = svg_hex(tint(colors, s_ix));
            let mut d = String::new();
            for (ix, &(px, py)) in pts.iter().enumerate() {
                let cmd = if ix == 0 { 'M' } else { 'L' };
                write!(d, "{cmd}{px:.1},{py:.1} ").expect("svg write");
            }
            d.push('Z');
            let fill_attr = if self.filled {
                format!(r#"fill="{hex}" fill-opacity="0.18""#)
            } else {
                r#"fill="none""#.to_string()
            };
            write!(
                out,
                r#"<path d="{d}" {fill_attr} stroke="{hex}" stroke-width="{stroke_px:.1}" stroke-linejoin="round"/>"#
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

pub(super) type PointRing = Vec<(f32, f32)>;

pub(super) struct RadialLayout {
    pub(super) inner: f32,
    pub(super) rings: Vec<(f32, f64)>,
    pub(super) band: Option<(PointRing, PointRing)>,
    pub(super) series_pts: Vec<PointRing>,
}

pub(super) fn radial_angle(ix: usize, count: usize, rotation: f32) -> f32 {
    -FRAC_PI_2 + rotation + TAU * ix as f32 / count.max(1) as f32
}

pub(super) fn radial_spoke_at(
    count: usize,
    center: (f32, f32),
    at_pos: (f32, f32),
    rotation: f32,
) -> usize {
    let (angle, _) = bearing(center, at_pos);
    let norm = (angle + FRAC_PI_2 - rotation).rem_euclid(TAU);
    ((norm / (TAU / count.max(1) as f32)).round() as usize) % count.max(1)
}

pub(super) fn compute_radial(
    n: usize,
    series: &[&Series],
    range_band: Option<&(Vec<f64>, Vec<f64>)>,
    center: (f32, f32),
    outer: f32,
    rotation: f32,
) -> RadialLayout {
    let inner = (outer * 0.16).max(0.0);
    if n == 0 || outer <= 0.0 {
        return RadialLayout {
            inner,
            rings: Vec::new(),
            band: None,
            series_pts: Vec::new(),
        };
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for s in series {
        for &v in &s.values {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if let Some((low, high)) = range_band {
        for &v in low.iter().chain(high) {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        lo = 0.0;
        hi = 1.0;
    }
    let ((dom_min, dom_max), ticks) = nice(lo, hi, 4);
    let radius_of = |v: f64| -> f32 {
        let share = if dom_max > dom_min {
            ((v - dom_min) / (dom_max - dom_min)).clamp(0.0, 1.0) as f32
        } else {
            0.5
        };
        inner + (outer - inner) * share
    };
    let pt_at = |ix: usize, v: f64| -> (f32, f32) {
        let r = radius_of(v);
        let a = radial_angle(ix, n, rotation);
        (center.0 + r * a.cos(), center.1 + r * a.sin())
    };
    let rings = ticks.into_iter().map(|t| (radius_of(t), t)).collect();
    let band = range_band.map(|(low, high)| {
        let low_pts = (0..n).map(|ix| pt_at(ix, low[ix].min(high[ix]))).collect();
        let high_pts = (0..n).map(|ix| pt_at(ix, low[ix].max(high[ix]))).collect();
        (low_pts, high_pts)
    });
    let series_pts = series
        .iter()
        .map(|s| (0..n).map(|ix| pt_at(ix, s.values[ix])).collect())
        .collect();
    RadialLayout {
        inner,
        rings,
        band,
        series_pts,
    }
}

pub(super) fn paint_closed_loop(path: &mut PathBuilder, pts: &[(f32, f32)], origin: Point<Pixels>) {
    if let Some(&first) = pts.first() {
        path.move_to(at(origin, first));
        for &pt in &pts[1..] {
            path.line_to(at(origin, pt));
        }
        path.close();
    }
}

impl Styled for RadialLineChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
