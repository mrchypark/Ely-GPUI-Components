use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window, div, point,
    prelude::*, px,
};

use super::settle;
use crate::charts::SankeyChart;
use crate::theme::Theme;

/// A flow of two nodes and one link, in a box wide enough for its columns to spread.
struct Flow;

impl Render for Flow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(SankeyChart::new("sankey", ["a", "b"]).link(0, 1, 5.0))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_flow_tooltip_over_a_node(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Flow);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_none(),
        "nothing is pointed at yet"
    );

    // The first column of nodes stands just past the room the labels take.
    let root = cx.debug_bounds("chart-root").expect("the flow draws");
    let at = point(root.left() + px(100.0), root.top() + px(100.0));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_some(),
        "a press alone opens the flow tooltip over a node"
    );
}

#[gpui::test]
fn a_press_alone_opens_the_flow_tooltip_over_a_link(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Flow);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_none(),
        "nothing is pointed at yet"
    );

    // The ribbon runs from the first column to the second, across the middle.
    let root = cx.debug_bounds("chart-root").expect("the flow draws");
    let at = point(root.left() + px(180.0), root.top() + px(100.0));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_some(),
        "a press alone opens the flow tooltip over a link"
    );
}
