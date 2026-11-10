use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window, div, point,
    prelude::*, px,
};

use super::settle;
use crate::charts::NetworkGraph;
use crate::theme::Theme;

/// A graph of one node, in a box the node settles in the middle of.
struct Graph;

impl Render for Graph {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(NetworkGraph::new("network").node("a", 0))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_network_tooltip_over_a_node(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|_, _| Graph);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_none(),
        "no node is pointed at yet"
    );

    // A single node settles in the middle of the graph's box.
    let root = cx.debug_bounds("chart-root").expect("the graph draws");
    let at = root.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_some(),
        "a press alone opens the network tooltip over a node"
    );
}

#[gpui::test]
fn a_press_on_a_node_leaves_it_pointed_at_after_the_finger_lifts(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|_, _| Graph);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    let root = cx.debug_bounds("chart-root").expect("the graph draws");
    let at = root.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_some(),
        "the node stays pointed at, so its tooltip stays open"
    );
}

#[gpui::test]
fn a_press_away_from_every_node_leaves_nothing_pointed_at(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|_, _| Graph);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    let root = cx.debug_bounds("chart-root").expect("the graph draws");
    let at = point(root.left() + px(2.0), root.top() + px(2.0));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_none(),
        "a press on nothing points at nothing"
    );
}
