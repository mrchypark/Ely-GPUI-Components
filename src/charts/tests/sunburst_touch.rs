use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, Styled, TestAppContext, Window, point, px,
};

use super::settle;
use crate::charts::{Slice, Sunburst};
use crate::theme::Theme;

/// A sunburst of one slice, in a box big enough for the ring to sit in.
struct Sun;

impl Render for Sun {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Sunburst::new("sun", [Slice::new("a", 1.0)])
            .w(px(400.0))
            .h(px(400.0))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_sunburst_reading(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Sun);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("sunburst-tooltip").is_none(),
        "no arc is hovered yet"
    );

    let root = cx.debug_bounds("chart-root").expect("the sunburst draws");
    let outer = (f32::from(root.size.width).min(f32::from(root.size.height)) / 2.0).max(1.0);
    let center = root.center();
    let at = point(
        center.x,
        gpui::Pixels::from(f32::from(center.y) - outer * 0.6),
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("sunburst-tooltip").is_some(),
        "a press alone reads the sunburst's arc"
    );
}
