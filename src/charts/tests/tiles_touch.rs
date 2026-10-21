use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window, div, point,
    prelude::*, px,
};

use super::settle;
use crate::charts::{HeatmapChart, Treemap};
use crate::theme::Theme;

/// A treemap of one tile.
struct Map;

impl Render for Map {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(Treemap::new("treemap").tile("a", 5.0))
    }
}

/// A heatmap of one cell.
struct Cells;

impl Render for Cells {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(HeatmapChart::new("heatmap", ["c"]).row("r", [5.0]))
    }
}

#[gpui::test]
fn a_press_alone_opens_the_treemap_tooltip_over_a_tile(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Map);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_none(),
        "no tile is pointed at yet"
    );

    let root = cx.debug_bounds("chart-root").expect("the treemap draws");
    let at = root.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_some(),
        "a press alone opens the treemap tooltip over a tile"
    );
}

#[gpui::test]
fn a_press_alone_opens_the_heatmap_tooltip_over_a_cell(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Cells);
    cx.simulate_resize(gpui::size(px(400.0), px(400.0)));
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_none(),
        "no cell is pointed at yet"
    );

    // The cells sit past the gutter of row labels and under the inset.
    let plot = cx.debug_bounds("heatmap-plot").expect("the heatmap draws");
    let at = point(
        plot.left() + px(48.0) + px(f32::from(plot.size.width) / 2.0),
        plot.top() + px(8.0) + px(f32::from(plot.size.height) / 2.0),
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert!(
        cx.debug_bounds("chart-tooltip").is_some(),
        "a press alone opens the heatmap tooltip over a cell"
    );
}
