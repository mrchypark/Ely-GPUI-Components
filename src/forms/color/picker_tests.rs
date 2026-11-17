use gpui::{Bounds, point, px, size};

use super::{Area, Hsva, pointed};

#[test]
fn pointing_sets_the_part_under_the_pointer() {
    let bounds = Bounds::new(point(px(10.0), px(10.0)), size(px(100.0), px(50.0)));
    let gray = Hsva {
        h: 200.0,
        s: 0.0,
        v: 0.5,
        a: 1.0,
    };
    let plane = pointed(gray, Area::Plane, bounds, point(px(60.0), px(20.0)));
    assert_eq!((plane.h, plane.s, plane.v), (200.0, 0.5, 0.8));
    let hue = pointed(gray, Area::Hue, bounds, point(px(200.0), px(0.0)));
    assert_eq!(hue.h, 360.0);
    let alpha = pointed(gray, Area::Alpha, bounds, point(px(35.0), px(30.0)));
    assert_eq!(alpha.a, 0.25);
}
