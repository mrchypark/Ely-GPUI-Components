use gpui::{Bounds, Pixels, Point, ScrollHandle, point, size};

/// Insets in logical window coordinates, before keyboard avoidance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewportInsets {
    pub top: Pixels,
    pub right: Pixels,
    pub bottom: Pixels,
    pub left: Pixels,
}

/// `keyboard_bottom` is occlusion from the original viewport bottom, including the safe area.
/// Pass zero when the host has already resized the viewport for the keyboard.
pub fn usable_bounds(
    viewport: Bounds<Pixels>,
    safe: ViewportInsets,
    keyboard_bottom: Pixels,
) -> Bounds<Pixels> {
    let width = viewport.size.width.max(Pixels::ZERO);
    let height = viewport.size.height.max(Pixels::ZERO);
    let left = safe.left.max(Pixels::ZERO).min(width);
    let top = safe.top.max(Pixels::ZERO).min(height);
    let right = safe.right.max(Pixels::ZERO).min(width - left);
    let bottom = safe
        .bottom
        .max(keyboard_bottom)
        .max(Pixels::ZERO)
        .min(height - top);
    Bounds::new(
        viewport.origin + point(left, top),
        size(width - left - right, height - top - bottom),
    )
}

/// The caller chooses its breakpoint in logical pixels, independently of the operating system.
pub fn is_compact(available: Bounds<Pixels>, breakpoint: Pixels) -> bool {
    available.size.width < breakpoint
}

/// Translation for a painted field to fit; an oversized field reveals its leading edge.
pub fn reveal_offset(field: Bounds<Pixels>, visible: Bounds<Pixels>) -> Point<Pixels> {
    fn shift(start: Pixels, end: Pixels, first: Pixels, last: Pixels) -> Pixels {
        if start < first {
            first - start
        } else if end > last {
            (last - end).max(first - start)
        } else {
            Pixels::ZERO
        }
    }
    point(
        shift(field.left(), field.right(), visible.left(), visible.right()),
        shift(field.top(), field.bottom(), visible.top(), visible.bottom()),
    )
}

/// Call after layout when focus or occlusion changes; request a frame when this returns true.
/// Both bounds are painted window coordinates; the owner supplies a focused field or caret.
/// The host must resize the scroll viewport or add keyboard padding to make the field reachable.
/// Movement is clamped to the laid-out scroll range; true does not guarantee full visibility.
pub fn reveal_in_bounds(
    scroll: &ScrollHandle,
    field: Bounds<Pixels>,
    visible: Bounds<Pixels>,
) -> bool {
    let frame = scroll.bounds();
    let left = frame.left().max(visible.left());
    let top = frame.top().max(visible.top());
    let right = frame.right().min(visible.right());
    let bottom = frame.bottom().min(visible.bottom());
    if right <= left || bottom <= top {
        return false;
    }
    let delta = reveal_offset(
        field,
        Bounds::new(point(left, top), size(right - left, bottom - top)),
    );
    if delta == Point::default() {
        return false;
    }
    let before = scroll.offset();
    let maximum = scroll.max_offset();
    let proposed = before + delta;
    let after = point(
        proposed.x.clamp(-maximum.x, Pixels::ZERO),
        proposed.y.clamp(-maximum.y, Pixels::ZERO),
    );
    if after == before {
        return false;
    }
    scroll.set_offset(after);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn keyboard_overlaps_safe_bottom_and_preserves_origin() {
        let safe = ViewportInsets {
            top: px(20.),
            bottom: px(30.),
            ..Default::default()
        };
        assert_eq!(
            usable_bounds(rect(10., 15., 400., 800.), safe, px(300.)),
            rect(10., 35., 400., 480.)
        );
        assert_eq!(
            usable_bounds(rect(0., 0., 400., 800.), safe, px(10.)),
            rect(0., 20., 400., 750.)
        );
    }

    #[test]
    fn empty_and_excessive_insets_do_not_invert_bounds() {
        let view = rect(4., 5., 100., 80.);
        assert_eq!(
            usable_bounds(view, ViewportInsets::default(), Pixels::ZERO),
            view
        );
        let safe = ViewportInsets {
            left: px(200.),
            top: px(90.),
            right: px(20.),
            bottom: px(20.),
        };
        assert_eq!(usable_bounds(view, safe, px(100.)), rect(104., 85., 0., 0.));
        assert!(is_compact(view, px(101.)));
        assert!(!is_compact(view, px(100.)));
    }

    #[test]
    fn reveal_is_minimal_and_prioritizes_oversized_field_start() {
        let visible = rect(10., 20., 100., 100.);
        assert_eq!(
            reveal_offset(rect(20., 30., 20., 20.), visible),
            Point::default()
        );
        assert_eq!(
            reveal_offset(rect(90., 110., 40., 30.), visible),
            point(px(-20.), px(-20.))
        );
        assert_eq!(
            reveal_offset(rect(0., 10., 200., 200.), visible),
            point(px(10.), px(10.))
        );
        assert_eq!(
            reveal_offset(rect(20., 30., 200., 200.), visible),
            point(px(-10.), px(-10.))
        );
    }
}
