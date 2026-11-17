use super::*;

/// Geographic map chart supporting choropleth regions, scaled point markers, graticules, and 3D rotation.
#[derive(IntoElement)]
pub struct GeoChart {
    pub(super) base: Div,
    pub(super) id: ElementId,
    pub(super) projection: GeoProjection,
    pub(super) rotate: (f64, f64),
    pub(super) graticule_step: f64,
    pub(super) show_graticule: bool,
    pub(super) regions: Vec<GeoRegion>,
    pub(super) points: Vec<GeoPoint>,
    pub(super) format: Format,
}

impl GeoChart {
    /// Create a geographic chart with Equirectangular projection and 30° graticule enabled.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            projection: GeoProjection::Equirectangular,
            rotate: (0.0, 0.0),
            graticule_step: 30.0,
            show_graticule: true,
            regions: Vec::new(),
            points: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// Select the map projection.
    pub fn projection(mut self, projection: GeoProjection) -> Self {
        self.projection = projection;
        self
    }

    /// Apply 3D spherical rotation `(yaw_deg, pitch_deg)` before projecting.
    pub fn rotate(mut self, yaw_deg: f64, pitch_deg: f64) -> Self {
        assert!(
            yaw_deg.is_finite() && pitch_deg.is_finite(),
            "geo rotation angles must be finite"
        );
        self.rotate = (yaw_deg, pitch_deg);
        self
    }

    /// Set the graticule grid spacing in degrees and enable it.
    pub fn graticule(mut self, step_deg: f64) -> Self {
        assert!(
            step_deg.is_finite() && step_deg > 0.0 && step_deg <= 180.0,
            "graticule step must be finite in (0, 180]"
        );
        self.graticule_step = step_deg;
        self.show_graticule = true;
        self
    }

    /// Show or hide the latitude/longitude graticule.
    pub fn show_graticule(mut self, show: bool) -> Self {
        self.show_graticule = show;
        self
    }

    /// Add a choropleth region.
    pub fn region(mut self, region: GeoRegion) -> Self {
        self.regions.push(region);
        self
    }

    /// Add multiple choropleth regions.
    pub fn regions(mut self, regions: impl IntoIterator<Item = GeoRegion>) -> Self {
        self.regions.extend(regions);
        self
    }

    /// Add a point marker.
    pub fn point(mut self, point: GeoPoint) -> Self {
        self.points.push(point);
        self
    }

    /// Add multiple point markers.
    pub fn points(mut self, points: impl IntoIterator<Item = GeoPoint>) -> Self {
        self.points.extend(points);
        self
    }

    /// Format numeric values in tooltips.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    /// Export regions and points as CSV (`kind,name,lon,lat,value`).
    pub fn csv(&self) -> String {
        let mut out = String::from("kind,name,lon,lat,value");
        for region in &self.regions {
            let (lon, lat) = region
                .rings
                .first()
                .and_then(|ring| centroid_deg(ring))
                .unwrap_or((0.0, 0.0));
            write!(
                out,
                "\nregion,{},{lon:.4},{lat:.4},{}",
                csv_field(&region.name),
                region.value
            )
            .expect("csv write");
        }
        for pt in &self.points {
            write!(
                out,
                "\npoint,{},{},{},{}",
                csv_field(&pt.name),
                pt.lon,
                pt.lat,
                pt.value
            )
            .expect("csv write");
        }
        out
    }

    /// Export the projected map as a standalone SVG document.
    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let theme = cx.theme();
        let (colors, sizes, rem) = (&theme.colors, theme.chart(), theme.base_rem());
        let pad = f32::from(sizes.inset.to_pixels(rem));
        let scene = build_scene(
            self,
            (width, height),
            pad,
            GeoView {
                rotate: self.rotate,
                zoom: 1.0,
                pan: (0.0, 0.0),
            },
        );
        let bg_hex = svg_hex(colors.bg);
        let border_hex = svg_hex(colors.border);
        let region_hex = svg_hex(tint(colors, 0));
        let point_hex = svg_hex(if self.regions.is_empty() {
            tint(colors, 0)
        } else {
            tint(colors, 1)
        });
        let (low, high) = region_extent(&self.regions);

        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        write!(
            out,
            r#"<rect width="{width}" height="{height}" fill="{bg_hex}"/>"#
        )
        .expect("svg write");

        if let Some((cx_px, cy_px, r_px)) = scene.globe {
            write!(
                out,
                r#"<circle cx="{cx_px:.1}" cy="{cy_px:.1}" r="{r_px:.1}" fill="none" stroke="{border_hex}" stroke-opacity="0.7" stroke-width="1"/>"#,
            )
            .expect("svg write");
        } else if scene.frame_w > 0.0 && scene.frame_h > 0.0 {
            write!(
                out,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="none" stroke="{border_hex}" stroke-opacity="0.55" stroke-width="1"/>"#,
                scene.frame_x, scene.frame_y, scene.frame_w, scene.frame_h
            )
            .expect("svg write");
        }

        if !scene.graticule.is_empty() {
            let mut d = String::new();
            for line in &scene.graticule {
                write_polyline_svg(&mut d, line);
            }
            if !d.is_empty() {
                write!(
                    out,
                    r#"<path d="{d}" fill="none" stroke="{border_hex}" stroke-opacity="0.45" stroke-width="0.75"/>"#,
                )
                .expect("svg write");
            }
        }

        for region in &scene.regions {
            let alpha = strength(region.value, low, high);
            let mut d = String::new();
            for ring in &region.rings {
                if ring.len() >= 3 {
                    write_polygon_svg(&mut d, ring);
                }
            }
            if !d.is_empty() {
                write!(
                    out,
                    r#"<path d="{d}" fill="{region_hex}" fill-opacity="{alpha:.2}" stroke="{bg_hex}" stroke-opacity="0.8" stroke-width="1"/>"#,
                )
                .expect("svg write");
            }
        }

        for pt in &scene.points {
            write!(
                out,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{point_hex}" fill-opacity="0.75" stroke="{point_hex}" stroke-width="1"/>"#,
                pt.center.0, pt.center.1, pt.radius
            )
            .expect("svg write");
        }

        out.push_str("</svg>");
        out
    }
}

impl Styled for GeoChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
