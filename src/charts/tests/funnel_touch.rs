use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, Styled, TestAppContext, Window, point, px,
};

use super::settle;
use crate::charts::FunnelChart;
use crate::theme::Theme;

/// A funnel of two stages, in a box big enough for the bars to sit in.
struct Funnel;

impl Render for Funnel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FunnelChart::new("funnel")
            .stage("a", 3.0)
            .stage("b", 1.0)
            .w(px(400.0))
            .h(px(400.0))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_funnel_tooltip(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Funnel);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("funnel-tooltip").is_none(),
        "no stage is hovered yet"
    );

    let root = cx.debug_bounds("chart-root").expect("the funnel draws");
    let row = f32::from(root.size.height) / 2.0;
    let at = point(
        root.center().x,
        gpui::Pixels::from(f32::from(root.origin.y) + row * 0.5),
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("funnel-tooltip").is_some(),
        "a press alone opens the funnel tooltip"
    );
}
