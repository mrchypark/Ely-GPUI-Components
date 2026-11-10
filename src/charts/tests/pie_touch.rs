use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, Styled, TestAppContext, Window, point, px,
};

use super::settle;
use crate::charts::PieChart;
use crate::theme::Theme;

/// A pie of two slices, in a box big enough for the ring to sit in.
struct Pie;

impl Render for Pie {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        PieChart::new("pie")
            .slice("a", 1.0)
            .slice("b", 3.0)
            .w(px(400.0))
            .h(px(400.0))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_pie_tooltip(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Pie);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("pie-tooltip").is_none(),
        "no slice is hovered yet"
    );

    let plot = cx.debug_bounds("pie-plot").expect("the pie draws");
    let outer = (f32::from(plot.size.width).min(f32::from(plot.size.height)) / 2.0).max(1.0);
    let center = plot.center();
    let at = point(
        center.x,
        gpui::Pixels::from(f32::from(center.y) - outer * 0.5),
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("pie-tooltip").is_some(),
        "a press alone opens the pie tooltip"
    );
}
