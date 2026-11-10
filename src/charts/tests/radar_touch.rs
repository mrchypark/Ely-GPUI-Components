use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, Styled, TestAppContext, Window, point, px,
};

use super::settle;
use crate::charts::{RadarChart, Series};
use crate::theme::Theme;

/// A radar of three spokes, in a box big enough for the shape to sit in.
struct Radar;

impl Render for Radar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        RadarChart::new("radar", ["a", "b", "c"])
            .series(Series::new("s", [1.0, 2.0, 3.0]))
            .w(px(400.0))
            .h(px(400.0))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_radar_tooltip(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Radar);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("radar-tooltip").is_none(),
        "no spoke is hovered yet"
    );

    let plot = cx.debug_bounds("radar-plot").expect("the radar draws");
    let reach = (f32::from(plot.size.height).min(f32::from(plot.size.width)) / 2.0).max(1.0);
    let center = plot.center();
    let at = point(
        center.x,
        gpui::Pixels::from(f32::from(center.y) - reach * 0.5),
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("radar-tooltip").is_some(),
        "a press alone opens the radar tooltip"
    );
}
