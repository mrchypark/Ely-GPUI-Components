//! Coordinate transforms between data and screen space.
use crate::charts::advanced::axis::AxisScale;
use crate::charts::advanced::geom::{Point, ScreenPoint, ScreenRect};
use crate::charts::advanced::view::{Range, Viewport};

const MIN_SPAN: f64 = 1e-12;
const LOG_FLOOR: f64 = 1e-12;

/// Transform from data coordinates into screen coordinates.
#[derive(Debug, Clone)]
pub(crate) struct Transform {
    viewport: Viewport,
    screen: ScreenRect,
    x_scale: AxisScale,
    y_scale: AxisScale,
    x_mapped: Range,
    y_mapped: Range,
}

impl Transform {
    /// Create a transform for the given viewport and screen rectangle.
    #[cfg(test)]
    pub(crate) fn new(viewport: Viewport, screen: ScreenRect) -> Option<Self> {
        Self::with_scales(viewport, screen, AxisScale::Linear, AxisScale::Linear)
    }

    /// Create a transform with explicit X/Y axis scale modes.
    pub(crate) fn with_scales(
        viewport: Viewport,
        screen: ScreenRect,
        x_scale: AxisScale,
        y_scale: AxisScale,
    ) -> Option<Self> {
        if !screen.is_valid() {
            return None;
        }
        let x_axis = map_range(viewport.x.with_min_span(MIN_SPAN), x_scale)?;
        let y_axis = map_range(viewport.y.with_min_span(MIN_SPAN), y_scale)?;
        let x_mapped = mapped_range(x_axis, x_scale)?;
        let y_mapped = mapped_range(y_axis, y_scale)?;
        Some(Self {
            viewport,
            screen,
            x_scale,
            y_scale,
            x_mapped,
            y_mapped,
        })
    }

    /// Access the viewport.
    pub(crate) fn viewport(&self) -> Viewport {
        self.viewport
    }

    /// Access the screen rectangle.
    pub(crate) fn screen(&self) -> ScreenRect {
        self.screen
    }

    pub(crate) fn x_scale(&self) -> AxisScale {
        self.x_scale
    }

    pub(crate) fn y_scale(&self) -> AxisScale {
        self.y_scale
    }

    /// Map a data point into screen space.
    pub(crate) fn data_to_screen(&self, point: Point) -> Option<ScreenPoint> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let px = forward_scale(point.x, self.x_scale)?;
        let py = forward_scale(point.y, self.y_scale)?;
        let x_norm = (px - self.x_mapped.min) / self.x_mapped.span();
        let y_norm = (py - self.y_mapped.min) / self.y_mapped.span();
        let sx = self.screen.min.x as f64 + x_norm * self.screen.width() as f64;
        let sy = self.screen.max.y as f64 - y_norm * self.screen.height() as f64;
        Some(ScreenPoint::new(sx as f32, sy as f32))
    }

    /// Map a screen point into data space.
    pub(crate) fn screen_to_data(&self, point: ScreenPoint) -> Option<Point> {
        let x_norm = (point.x as f64 - self.screen.min.x as f64) / self.screen.width() as f64;
        let y_norm = (self.screen.max.y as f64 - point.y as f64) / self.screen.height() as f64;
        let ux = self.x_mapped.min + x_norm * self.x_mapped.span();
        let uy = self.y_mapped.min + y_norm * self.y_mapped.span();
        let x_axis = inverse_scale(ux, self.x_scale)?;
        let y_axis = inverse_scale(uy, self.y_scale)?;
        Some(Point::new(x_axis, y_axis))
    }
}

pub(crate) fn forward_scale(value: f64, scale: AxisScale) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    let mapped = match scale {
        AxisScale::Linear => value,
        AxisScale::Log(base) => {
            if !base.is_finite() || base <= 0.0 || (base - 1.0).abs() < 1e-12 {
                return None;
            }
            value.max(LOG_FLOOR).log(base)
        }
        AxisScale::Symlog(c) => {
            if !c.is_finite() || c <= 0.0 {
                return None;
            }
            value.signum() * (1.0 + value.abs() / c).ln()
        }
    };
    mapped.is_finite().then_some(mapped)
}

pub(crate) fn inverse_scale(mapped: f64, scale: AxisScale) -> Option<f64> {
    if !mapped.is_finite() {
        return None;
    }
    let value = match scale {
        AxisScale::Linear => mapped,
        AxisScale::Log(base) => base.powf(mapped),
        AxisScale::Symlog(c) => mapped.signum() * c * (mapped.abs().exp() - 1.0),
    };
    value.is_finite().then_some(value)
}

fn map_range(range: Range, scale: AxisScale) -> Option<Range> {
    if !range.is_finite() {
        return None;
    }
    if matches!(scale, AxisScale::Log(_)) {
        let min = range.min.max(LOG_FLOOR);
        let max = range.max.max(min * 1.0001);
        return Some(Range::new(min, max));
    }
    Some(range)
}

fn mapped_range(range: Range, scale: AxisScale) -> Option<Range> {
    let a = forward_scale(range.min, scale)?;
    let b = forward_scale(range.max, scale)?;
    Some(Range::new(a, b).with_min_span(MIN_SPAN))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_roundtrip() {
        let viewport = Viewport::new(Range::new(0.0, 10.0), Range::new(0.0, 10.0));
        let screen = ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(100.0, 100.0));
        let transform = Transform::new(viewport, screen).expect("valid transform");
        let point = Point::new(5.0, 7.5);
        let screen_point = transform.data_to_screen(point).unwrap();
        let roundtrip = transform.screen_to_data(screen_point).unwrap();
        assert!((roundtrip.x - point.x).abs() < 1e-9);
        assert!((roundtrip.y - point.y).abs() < 1e-9);
    }
}
