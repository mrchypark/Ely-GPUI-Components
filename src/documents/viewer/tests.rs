use gpui::px;

use super::{page_at, step};

#[test]
fn the_page_in_view_is_the_one_under_the_middle() {
    let bottoms = [px(100.0), px(220.0), px(340.0)];
    assert_eq!(page_at(&bottoms, px(50.0)), 0);
    assert_eq!(page_at(&bottoms, px(150.0)), 1);
    assert_eq!(page_at(&bottoms, px(500.0)), 2, "past the end is the last");
    assert_eq!(page_at(&[], px(50.0)), 0, "no layout yet");
}

#[test]
fn zoom_steps_walk_the_scale_and_stop_at_its_ends() {
    assert_eq!(step(1.0, true), Some(1.25));
    assert_eq!(step(1.0, false), Some(0.75));
    assert_eq!(
        step(1.1, false),
        Some(1.0),
        "between steps goes to the nearer side"
    );
    assert_eq!(step(4.0, true), None);
    assert_eq!(step(0.25, false), None);
}
