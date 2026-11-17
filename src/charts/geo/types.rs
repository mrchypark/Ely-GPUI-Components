use super::*;

/// Geographic map projections supported by [`GeoChart`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeoProjection {
    #[default]
    Equirectangular,
    Mercator,
    Orthographic,
    Stereographic,
}

/// A geographic polygon region (country, province, or zone) with an associated scalar value.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoRegion {
    pub name: SharedString,
    pub value: f64,
    pub rings: Vec<Vec<(f64, f64)>>,
}

impl GeoRegion {
    /// Create a region from its name, scalar value, and exterior `(lon_deg, lat_deg)` ring.
    pub fn new(
        name: impl Into<SharedString>,
        value: f64,
        exterior_ring: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        assert!(
            drawable(value),
            "a geo region needs a finite value within charts::LIMIT"
        );
        let ring = validate_ring(exterior_ring);
        Self {
            name: name.into(),
            value,
            rings: if ring.is_empty() {
                Vec::new()
            } else {
                vec![ring]
            },
        }
    }

    /// Append an additional ring (an island or interior cutout) in `(lon_deg, lat_deg)`.
    pub fn ring(mut self, extra_ring: impl IntoIterator<Item = (f64, f64)>) -> Self {
        let ring = validate_ring(extra_ring);
        if !ring.is_empty() {
            self.rings.push(ring);
        }
        self
    }
}

/// A geographic point marker at `(lon_deg, lat_deg)` with a scalar value.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoPoint {
    pub name: SharedString,
    pub lon: f64,
    pub lat: f64,
    pub value: f64,
}

impl GeoPoint {
    /// Create a geographic point at `(lon_deg, lat_deg)` in `-180..=180`, `-90..=90`.
    pub fn new(name: impl Into<SharedString>, lon: f64, lat: f64, value: f64) -> Self {
        assert!(
            lon.is_finite()
                && (-180.0..=180.0).contains(&lon)
                && lat.is_finite()
                && (-90.0..=90.0).contains(&lat),
            "geo point coordinates must be within lon -180..=180 and lat -90..=90"
        );
        assert!(
            drawable(value),
            "a geo point needs a finite value within charts::LIMIT"
        );
        Self {
            name: name.into(),
            lon,
            lat,
            value,
        }
    }
}
