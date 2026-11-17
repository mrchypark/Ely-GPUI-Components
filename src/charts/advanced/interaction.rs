//! Interaction helpers for panning, zooming, and pin selection.

use crate::charts::advanced::axis::AxisScale;
use crate::charts::advanced::geom::{Point, ScreenPoint, ScreenRect};
use crate::charts::advanced::series::SeriesId;
use crate::charts::advanced::transform::{Transform, forward_scale, inverse_scale};
use crate::charts::advanced::view::{Range, Viewport};

/// Interaction hit regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HitRegion {
    /// Plot data area.
    Plot,
    /// X axis area.
    XAxis,
    /// Y axis area.
    YAxis,
    /// Outside of the plot.
    Outside,
}

/// Screen regions for hit testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlotRegions {
    /// Plot data area.
    pub(crate) plot: ScreenRect,
    /// X axis area.
    pub(crate) x_axis: ScreenRect,
    /// Y axis area.
    pub(crate) y_axis: ScreenRect,
}

impl PlotRegions {
    /// Determine which region contains the point.
    pub(crate) fn hit_test(&self, point: ScreenPoint) -> HitRegion {
        if contains(self.plot, point) {
            HitRegion::Plot
        } else if contains(self.x_axis, point) {
            HitRegion::XAxis
        } else if contains(self.y_axis, point) {
            HitRegion::YAxis
        } else {
            HitRegion::Outside
        }
    }
}

/// Pin binding to a stable point identity. Pins are stable references to a specific series and point index, allowing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    /// Series identifier.
    pub series_id: SeriesId,
    /// Point index within the series; absolute sample index for a sample window.
    pub point_index: usize,
}

/// Toggle a pin in the list. Returns true if added, false if removed.
pub(crate) fn toggle_pin(pins: &mut Vec<Pin>, pin: Pin) -> bool {
    if let Some(index) = pins.iter().position(|existing| *existing == pin) {
        pins.swap_remove(index);
        false
    } else {
        pins.push(pin);
        true
    }
}

/// Pan a viewport by a pixel delta.
pub(crate) fn pan_viewport(
    viewport: Viewport,
    delta_pixels: ScreenPoint,
    transform: &Transform,
) -> Option<Viewport> {
    let p0 = transform.screen_to_data(ScreenPoint::new(0.0, 0.0))?;
    let p1 = transform.screen_to_data(ScreenPoint::new(delta_pixels.x, delta_pixels.y))?;
    let x_scale = transform.x_scale();
    let y_scale = transform.y_scale();
    let x = pan_axis(viewport.x, p0.x, p1.x, x_scale)?;
    let y = pan_axis(viewport.y, p0.y, p1.y, y_scale)?;
    Some(Viewport::new(x, y))
}

fn pan_axis(range: Range, origin: f64, shifted: f64, scale: AxisScale) -> Option<Range> {
    let d = forward_scale(shifted, scale)? - forward_scale(origin, scale)?;
    let min = inverse_scale(forward_scale(range.min, scale)? - d, scale)?;
    let max = inverse_scale(forward_scale(range.max, scale)? - d, scale)?;
    Some(Range::new(min, max))
}

pub(crate) fn zoom_viewport_with_scales(
    viewport: Viewport,
    center: Point,
    factor_x: f64,
    factor_y: f64,
    x_scale: AxisScale,
    y_scale: AxisScale,
) -> Viewport {
    let zoom_axis = |range: Range, c: f64, factor: f64, scale: AxisScale| -> Range {
        if let (Some(uc), Some(u0), Some(u1)) = (
            forward_scale(c, scale),
            forward_scale(range.min, scale),
            forward_scale(range.max, scale),
        ) && let (Some(min), Some(max)) = (
            inverse_scale(uc + (u0 - uc) * factor, scale),
            inverse_scale(uc + (u1 - uc) * factor, scale),
        ) {
            return Range::new(min, max);
        }
        let min = c + (range.min - c) * factor;
        let max = c + (range.max - c) * factor;
        Range::new(min, max)
    };
    Viewport::new(
        zoom_axis(viewport.x, center.x, factor_x, x_scale),
        zoom_axis(viewport.y, center.y, factor_y, y_scale),
    )
}

/// Convert a zoom rectangle into a new viewport.
pub(crate) fn zoom_to_rect(
    viewport: Viewport,
    rect: ScreenRect,
    transform: &Transform,
) -> Option<Viewport> {
    if rect.width().abs() < 2.0 || rect.height().abs() < 2.0 {
        return Some(viewport);
    }
    let data_min = transform.screen_to_data(rect.min)?;
    let data_max = transform.screen_to_data(rect.max)?;
    Some(Viewport::new(
        Range::new(data_min.x, data_max.x),
        Range::new(data_min.y, data_max.y),
    ))
}

/// Compute a zoom factor from a drag delta and axis length.
pub(crate) fn zoom_factor_from_drag(delta_pixels: f32, axis_pixels: f32) -> f64 {
    if axis_pixels <= 0.0 {
        return 1.0;
    }
    let normalized = delta_pixels as f64 / axis_pixels as f64;
    (1.0 - normalized).clamp(0.1, 10.0)
}

fn contains(rect: ScreenRect, point: ScreenPoint) -> bool {
    point.x >= rect.min.x && point.x <= rect.max.x && point.y >= rect.min.y && point.y <= rect.max.y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_regions() {
        let regions = PlotRegions {
            plot: ScreenRect::new(ScreenPoint::new(0.0, 0.0), ScreenPoint::new(10.0, 10.0)),
            x_axis: ScreenRect::new(ScreenPoint::new(0.0, 10.0), ScreenPoint::new(10.0, 12.0)),
            y_axis: ScreenRect::new(ScreenPoint::new(-2.0, 0.0), ScreenPoint::new(0.0, 10.0)),
        };
        assert_eq!(
            regions.hit_test(ScreenPoint::new(5.0, 5.0)),
            HitRegion::Plot
        );
        assert_eq!(
            regions.hit_test(ScreenPoint::new(5.0, 11.0)),
            HitRegion::XAxis
        );
        assert_eq!(
            regions.hit_test(ScreenPoint::new(-1.0, 5.0)),
            HitRegion::YAxis
        );
    }
}
