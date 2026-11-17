use gpui::{Bounds, Pixels, Point};

pub(crate) fn fraction(value: f64, min: f64, max: f64) -> f32 {
    ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
}

/// The value at `fraction` of `min..=max`, on the `step` grid.
pub(crate) fn value_at(fraction: f32, min: f64, max: f64, step: f64) -> f64 {
    let raw = min + f64::from(fraction.clamp(0.0, 1.0)) * (max - min);
    (min + ((raw - min) / step).round() * step).clamp(min, max)
}

/// Where a key moves `value`: arrows step, Page keys jump ten, Home and End go to the ends.
pub(crate) fn keyed(key: &str, value: f64, min: f64, max: f64, step: f64) -> Option<f64> {
    let next = match key {
        "left" | "down" => value - step,
        "right" | "up" => value + step,
        "pagedown" => value - step * 10.0,
        "pageup" => value + step * 10.0,
        "home" => min,
        "end" => max,
        _ => return None,
    };
    Some(next.clamp(min, max))
}

/// How far along a track `at` is, from its start: left, or bottom when vertical.
pub(crate) fn along(bounds: Bounds<Pixels>, at: Point<Pixels>, vertical: bool) -> f32 {
    if vertical {
        (bounds.bottom() - at.y) / bounds.size.height
    } else {
        (at.x - bounds.left()) / bounds.size.width
    }
}
