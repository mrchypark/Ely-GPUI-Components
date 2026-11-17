use crate::{MobileGallery, probe::Probes};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size};

const LINES: &str = "LineChart / ChartAxis / ChartGrid / ChartLegend / ChartTooltip / ChartCrosshair / ChartAnnotation / ChartZoom / ChartExport";
const AREAS: &str = "areachart-stackedarea-barchart";

fn draw(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
}

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::init(cx);
        ely_gpui_component::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
}

#[gpui::test]
fn mobile_chart_catalog_renders_every_story_at_phone_and_tablet_sizes(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../gallery/stories.json")).unwrap();
    let page = catalog["pages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["slug"] == "charts")
        .unwrap();
    let stories = page["stories"].as_array().unwrap();
    let (view, cx) = cx.add_window_view(MobileGallery::new);
    for story in stories {
        let slug = story["slug"].as_str().unwrap();
        view.update(cx, |view, cx| {
            assert!(view.select_story("charts", slug, cx), "{slug}")
        });
        for (width, height) in [(390., 844.), (768., 1024.)] {
            cx.simulate_resize(size(px(width), px(height)));
            draw(cx);
            assert!(cx.debug_bounds("gallery-story-error").is_none(), "{slug}");
            let bounds = cx.debug_bounds("gallery-story-body").expect("story body");
            assert!(
                bounds.size.width > px(0.) && bounds.size.height > px(0.),
                "{slug}"
            );
        }
    }
}

#[gpui::test]
fn mobile_chart_stories_navigate_and_render_at_phone_and_tablet_widths(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);
    let (view, cx) = cx.add_window_view(MobileGallery::new);

    for (width, height) in [(390., 844.), (768., 1024.)] {
        cx.simulate_resize(size(px(width), px(height)));
        view.update(cx, |view, cx| {
            assert!(view.select_story("charts", LINES, cx))
        });
        draw(cx);
        let revenue = cx.update(|_, cx| Probes::get("revenue", cx).expect("line chart probe"));
        assert!(revenue.size.width > px(0.) && revenue.size.height > px(0.));
        assert!(
            revenue.origin.x + revenue.size.width <= px(width),
            "line chart {revenue:?} overflows at {width}×{height}"
        );
        assert!(cx.debug_bounds("chart-root").is_some());
        assert!(cx.debug_bounds("gallery-story-error").is_none());

        let next = cx
            .debug_bounds("gallery-story-next")
            .expect("next story")
            .center();
        cx.simulate_click(next, Modifiers::none());
        draw(cx);
        view.read_with(cx, |view, _| {
            assert_eq!(view.current_slug(), "charts");
            assert_eq!(view.current_story_slug(), Some(AREAS));
        });
        let bars = cx.update(|_, cx| Probes::get("bars", cx).expect("bar chart probe"));
        assert!(bars.size.width > px(0.) && bars.size.height > px(0.));
        assert!(cx.debug_bounds("gallery-story-error").is_none());
    }
}

#[gpui::test]
fn mobile_line_chart_accepts_tooltip_and_legend_input(cx: &mut TestAppContext) {
    let _files = crate::assets::prepare_test_files();
    init(cx);

    for (width, height) in [(390., 844.), (768., 1024.)] {
        let (view, cx) = cx.add_window_view(MobileGallery::new);
        cx.simulate_resize(size(px(width), px(height)));
        view.update(cx, |view, cx| {
            assert!(view.select_story("charts", LINES, cx))
        });
        draw(cx);
        let chart = cx.update(|_, cx| Probes::get("revenue", cx).expect("line chart probe"));
        let point = chart.origin + point(px(120.), chart.size.height / 2.);
        cx.simulate_mouse_move(point, None, Modifiers::none());
        draw(cx);
        let before = cx
            .debug_bounds("chart-tooltip")
            .expect("tooltip after chart input");

        let legend = cx
            .debug_bounds("chart-legend-entry-0")
            .expect("first chart legend entry")
            .center();
        cx.simulate_mouse_down(legend, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(legend, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(point, None, Modifiers::none());
        draw(cx);
        let after = cx
            .debug_bounds("chart-tooltip")
            .expect("tooltip after legend input");
        assert!(
            after.size.height < before.size.height,
            "legend input did not hide a tooltip row at {width}×{height}"
        );
    }
}
