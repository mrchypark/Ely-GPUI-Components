use super::*;

/// Isometric / perspective 3D surface mesh plot for 2D scalar grids z = f(x, y).
#[derive(IntoElement)]
pub struct SurfaceChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) cols: usize,
    pub(super) rows: usize,
    pub(super) x_range: (f64, f64),
    pub(super) y_range: (f64, f64),
    pub(super) values: Vec<f64>,
    pub(super) azimuth_deg: f32,
    pub(super) elevation_deg: f32,
    pub(super) wireframe: bool,
    pub(super) format: Format,
}

impl SurfaceChart {
    /// Build a surface mesh from a row-major cols * rows scalar grid.
    pub fn from_grid(
        id: impl Into<ElementId>,
        cols: usize,
        rows: usize,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            cols * rows,
            "surface grid values length must equal cols * rows"
        );
        assert!(
            values.iter().all(|v| drawable(*v)),
            "surface grid values must be finite and within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            cols,
            rows,
            x_range: (0.0, cols.saturating_sub(1) as f64),
            y_range: (0.0, rows.saturating_sub(1) as f64),
            values,
            azimuth_deg: -38.0,
            elevation_deg: 28.0,
            wireframe: true,
            format: Rc::new(compact),
        }
    }

    /// Sample a 2D function f(x, y) across cols * rows points over x_range and y_range.
    pub fn from_fn(
        id: impl Into<ElementId>,
        cols: usize,
        rows: usize,
        x_range: (f64, f64),
        y_range: (f64, f64),
        f: impl Fn(f64, f64) -> f64,
    ) -> Self {
        assert!(
            drawable(x_range.0)
                && drawable(x_range.1)
                && drawable(y_range.0)
                && drawable(y_range.1),
            "surface ranges must be finite and within charts::LIMIT"
        );
        let mut values = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            let ty = if rows > 1 {
                r as f64 / (rows - 1) as f64
            } else {
                0.5
            };
            let y = y_range.0 + (y_range.1 - y_range.0) * ty;
            for c in 0..cols {
                let tx = if cols > 1 {
                    c as f64 / (cols - 1) as f64
                } else {
                    0.5
                };
                let x = x_range.0 + (x_range.1 - x_range.0) * tx;
                let z = f(x, y);
                assert!(
                    drawable(z),
                    "surface function values must be finite and within charts::LIMIT"
                );
                values.push(z);
            }
        }
        Self {
            base: div(),
            id: id.into(),
            cols,
            rows,
            x_range,
            y_range,
            values,
            azimuth_deg: -38.0,
            elevation_deg: 28.0,
            wireframe: true,
            format: Rc::new(compact),
        }
    }

    /// Set camera azimuth and elevation angles in degrees (default (-38.0, 28.0)).
    pub fn angles(mut self, azimuth_deg: f32, elevation_deg: f32) -> Self {
        assert!(
            azimuth_deg.is_finite() && elevation_deg.is_finite(),
            "surface angles must be finite"
        );
        self.azimuth_deg = azimuth_deg;
        self.elevation_deg = elevation_deg;
        self
    }

    /// Toggle wireframe cell edges over surface quads (default true).
    pub fn wireframe(mut self, wireframe: bool) -> Self {
        self.wireframe = wireframe;
        self
    }

    /// Format numeric z values in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export the sampled grid as CSV (col,row,x,y,z).
    pub fn csv(&self) -> String {
        let mut out = String::from("col,row,x,y,z");
        for r in 0..self.rows {
            let ty = if self.rows > 1 {
                r as f64 / (self.rows - 1) as f64
            } else {
                0.5
            };
            let y = self.y_range.0 + (self.y_range.1 - self.y_range.0) * ty;
            for c in 0..self.cols {
                let tx = if self.cols > 1 {
                    c as f64 / (self.cols - 1) as f64
                } else {
                    0.5
                };
                let x = self.x_range.0 + (self.x_range.1 - self.x_range.0) * tx;
                let z = self.values[r * self.cols + c];
                write!(out, "\n{c},{r},{x},{y},{z}").expect("csv write");
            }
        }
        out
    }

    /// Export the 3D surface as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (sizes, rem, colors) = (theme.chart(), theme.base_rem(), &theme.colors);
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let rect = frame(
            (width, height),
            pixels(sizes.inset),
            pixels(sizes.inset),
            pixels(sizes.inset),
        );
        let scene = build_surface_scene(
            self.cols,
            self.rows,
            &self.values,
            self.azimuth_deg,
            self.elevation_deg,
            1.0,
            rect,
        );
        let (bg_hex, _) = svg_hex(colors.bg);
        let (axis_hex, _) = svg_hex(colors.border);
        let (wire_hex, _) = svg_hex(colors.fg);
        let (label_hex, _) = svg_hex(colors.fg_subtle);
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");
        for &(a, b) in &scene.axes {
            write!(
                out,
                r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{axis_hex}" stroke-opacity="0.7" stroke-width="1"/>"#,
                a.0, a.1, b.0, b.1
            )
            .expect("svg write");
        }
        for quad in &scene.quads {
            let fill = colors.chart[1].mix(&colors.chart[0], quad.t);
            let (fill_hex, _) = svg_hex(fill);
            let stroke_attr = if self.wireframe {
                format!(r#" stroke="{wire_hex}" stroke-opacity="0.22" stroke-width="0.75""#)
            } else {
                String::new()
            };
            write!(
                out,
                r#"<path d="M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} Z" fill="{fill_hex}" fill-opacity="0.92"{stroke_attr}/>"#,
                quad.pts[0].0,
                quad.pts[0].1,
                quad.pts[1].0,
                quad.pts[1].1,
                quad.pts[2].0,
                quad.pts[2].1,
                quad.pts[3].0,
                quad.pts[3].1,
            )
            .expect("svg write");
        }
        for ((lx, ly), label) in &scene.axis_labels {
            write!(
                out,
                r#"<text x="{lx:.1}" y="{ly:.1}" text-anchor="middle" fill="{label_hex}" font-size="11">{}</text>"#,
                svg_escape(label)
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

impl Styled for SurfaceChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
