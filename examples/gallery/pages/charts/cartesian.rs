use super::probe;

use std::time::Duration;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    charts::{
        AdvancedChart, AreaChart, BarChart, ChartTooltip, Curve, HorizonChart, LineChart, Points,
        RadialLineChart, RealtimeChart, ScatterChart, Series, Symbol,
        advanced::{
            AxisConfig, LineStyle, Point as PlotPoint, Series as PlotSeries, SeriesKind,
            series_color,
        },
    },
    theme::ActiveTheme,
    typography::Caption,
};
use gpui::{App, ClipboardItem, IntoElement, ParentElement, SharedString, Styled, Window, div, px};

use crate::ui::{keep, live, noise, row, section, set};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub fn lines(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let turn = keep("lines-turn", || 0usize, window, cx);
    let (now, next) = (*turn.read(cx), turn.clone());
    let copied = keep("lines-copied", || None::<&'static str>, window, cx);
    let (told, tell) = (*copied.read(cx), copied.clone());
    let mut rng = noise(21 + now as u64);
    let revenue: Vec<f64> = (0..12)
        .map(|month| 40.0 + month as f64 * 4.0 + rng() * 12.0)
        .collect();
    let costs: Vec<f64> = (0..12)
        .map(|month| 30.0 + month as f64 * 2.5 + rng() * 8.0)
        .collect();
    let chart = || {
        LineChart::new("revenue", MONTHS)
            .series(Series::new("Revenue", revenue.clone()))
            .series(Series::new("Costs", costs.clone()))
            .smooth()
            .rule(70.0, "Target")
            .note(8, "Launch")
            .zoom()
            .format(|value| {
                if value == 0.0 {
                    "$0".into()
                } else {
                    format!("${value:.0}k")
                }
            })
    };
    let (for_svg, for_csv) = (chart(), chart());
    section(
        "LineChart / ChartAxis / ChartGrid / ChartLegend / ChartTooltip / ChartCrosshair / ChartAnnotation / ChartZoom / ChartExport",
        "Hover for a crosshair and each series' value. Press a name in the legend to hide it. Drag across the chart to zoom into a span; a double press shows the whole year. The target and the launch are annotations. Copy the chart as SVG or its data as CSV.",
        cx,
    )
    .child(probe("revenue", div().w(px(720.)).max_w_full().child(chart())))
    .child(
        row()
            .child(Button::new("lines-next", "New data").variant(ButtonVariant::Ghost).on_click(move |_, _, cx| set(&next, now + 1, cx)))
            .child(Button::new("lines-svg", "Copy SVG").variant(ButtonVariant::Ghost).on_click({
                let tell = tell.clone();
                move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(for_svg.svg(720.0, 240.0, cx)));
                    set(&tell, Some("SVG"), cx)
                }
            }))
            .child(Button::new("lines-csv", "Copy CSV").variant(ButtonVariant::Ghost).on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(for_csv.csv()));
                set(&tell, Some("CSV"), cx)
            })),
    )
    .children(told.map(|what| Caption::new(format!("Copied the chart as {what}."))))
}

pub fn areas_and_bars(cx: &mut App) -> impl IntoElement + use<> {
    let quarters = ["Q1", "Q2", "Q3", "Q4"];
    let mut rng = noise(5);
    let mut series = |name: &'static str, base: f64| {
        Series::new(
            name,
            (0..4)
                .map(|q| base + q as f64 * 6.0 + rng() * 10.0)
                .collect::<Vec<_>>(),
        )
    };
    let (web, ios, android) = (
        series("Web", 30.0),
        series("iOS", 22.0),
        series("Android", 16.0),
    );
    section(
        "AreaChart / StackedArea / BarChart",
        "Areas stack each platform on the ones before it. Bars stand grouped, stacked, or run sideways.",
        cx,
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(div().w(px(360.)).max_w_full().child(AreaChart::new("areas", quarters).series(web.clone()).series(ios.clone()).series(android.clone()).stacked().smooth()))
            .child(probe("bars", div().w(px(360.)).max_w_full().child(BarChart::new("bars", quarters).series(web.clone()).series(ios.clone()).series(android.clone())))),
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(div().w(px(360.)).max_w_full().child(BarChart::new("stacked", quarters).series(web.clone()).series(ios.clone()).series(android.clone()).stacked()))
            .child(div().w(px(360.)).max_w_full().child(BarChart::new("sideways", ["Search", "Direct", "Social", "Email"]).series(Series::new("Visits", [412.0, 288.0, 164.0, 91.0])).horizontal())),
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(div().w(px(360.)).max_w_full().child(AreaChart::new("stream", quarters).series(web).series(ios).series(android).stream().smooth()))
            .child(div().w(px(360.)).max_w_full().child(LineChart::new("log-growth", quarters).series(Series::new("Requests", [12.0, 140.0, 1600.0, 18000.0])).curve(Curve::StepAfter).y_log(10.0))),
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(
                div().w(px(360.)).max_w_full().child(
                    HorizonChart::new("horizon-cpu", MONTHS)
                        .series(Series::new("Cluster A", [-18.0, -6.0, 8.0, 22.0, 35.0, 48.0, 41.0, 19.0, -4.0, -15.0, 12.0, 29.0]))
                        .series(Series::new("Cluster B", [14.0, 28.0, 19.0, -8.0, -24.0, -12.0, 9.0, 31.0, 44.0, 26.0, 7.0, -9.0]))
                        .bands(3),
                ),
            )
            .child(
                div().w(px(360.)).max_w_full().child(
                    RadialLineChart::new("radial-temp", MONTHS)
                        .series(Series::new("Mean °C", [-2.0, 1.0, 7.0, 13.0, 18.0, 23.0, 26.0, 25.0, 20.0, 14.0, 6.0, 0.0]))
                        .range_band(
                            [-6.0, -3.0, 2.0, 8.0, 13.0, 18.0, 22.0, 21.0, 15.0, 9.0, 2.0, -4.0],
                            [2.0, 5.0, 12.0, 18.0, 23.0, 28.0, 30.0, 29.0, 25.0, 19.0, 11.0, 4.0],
                        )
                        .filled(true),
                ),
            ),
    )
}

pub fn scatter(cx: &mut App) -> impl IntoElement + use<> {
    let mut rng = noise(9);
    let cloud = |name: &'static str, cx0: f64, cy0: f64, rng: &mut dyn FnMut() -> f64| {
        Points::new(
            name,
            (0..28)
                .map(|_| (cx0 + (rng() - 0.5) * 40.0, cy0 + (rng() - 0.5) * 30.0))
                .collect::<Vec<_>>(),
        )
    };
    let (a, b) = (
        cloud("Trial", 40.0, 60.0, &mut rng),
        cloud("Paid", 75.0, 35.0, &mut rng),
    );
    let cities = Points::new(
        "Cities",
        [
            (12.0, 70.0),
            (35.0, 52.0),
            (58.0, 81.0),
            (80.0, 44.0),
            (94.0, 66.0),
        ],
    )
    .sizes([3.2, 8.9, 1.4, 12.5, 5.6]);
    section(
        "ScatterChart / BubbleChart",
        "Points placed by two values; sizes turn them into bubbles whose areas compare.",
        cx,
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .child(
                div().w(px(360.)).max_w_full().child(
                    ScatterChart::new("scatter")
                        .points(a.symbol(Symbol::Diamond))
                        .points(b.symbol(Symbol::Triangle))
                        .voronoi()
                        .axes("Sessions", "Retention"),
                ),
            )
            .child(probe(
                "bubbles",
                div().w(px(360.)).max_w_full().child(
                    ScatterChart::new("bubbles")
                        .points(cities)
                        .axes("Price", "Rating"),
                ),
            )),
    )
}

pub fn realtime(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let stream = keep(
        "stream",
        || {
            let mut rng = noise(17);
            (
                (0..40).map(|_| 50.0 + rng() * 20.0).collect::<Vec<f64>>(),
                40u64,
            )
        },
        window,
        cx,
    );
    let ticking = keep("stream-tick", || false, window, cx);
    if !*ticking.read(cx) && !cx.theme().reduced_motion && live("RealtimeChart", cx) {
        set(&ticking, true, cx);
        let stream = stream.clone();
        cx.spawn(async move |cx| {
            let mut rng = noise(29);
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(900))
                    .await;
                cx.update(|cx| {
                    stream.update(cx, |(values, pushed), cx| {
                        let last = values.last().copied().unwrap_or(60.0);
                        values.push((last + (rng() - 0.5) * 10.0).clamp(20.0, 100.0));
                        values.remove(0);
                        *pushed += 1;
                        cx.notify();
                    })
                });
            }
        })
        .detach();
    }
    let (values, pushed) = stream.read(cx).clone();
    let latest = values.last().copied().unwrap_or_default();
    section(
        "RealtimeChart",
        "A new reading every second; the line slides left as each arrives.",
        cx,
    )
    .child(
        div()
            .w(px(720.))
            .max_w_full()
            .child(RealtimeChart::new("stream", values, pushed)),
    )
    .child(div().w(px(200.)).child(ChartTooltip::new("Latest").row(
        Some(cx.theme().colors.chart[0]),
        "Requests per second",
        SharedString::from(format!("{latest:.1}")),
    )))
}

pub fn advanced(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let selected = keep("advanced-selected", || None::<f64>, window, cx);
    let reset = keep("advanced-reset", || 0u64, window, cx);
    let (color_a, color_b) = (series_color(0, cx), series_color(1, cx));
    let primary = keep(
        "advanced-primary",
        || {
            PlotSeries::from_iter_points(
                "Signal A",
                (0..2000).map(|i| {
                    let x = i as f64 * 0.5;
                    PlotPoint::new(x, (x * 0.04).sin() * 24.0 + (x * 0.19).cos() * 6.0 + 50.0)
                }),
                SeriesKind::Line(LineStyle {
                    color: color_a,
                    width: 1.5,
                }),
            )
        },
        window,
        cx,
    );
    let secondary = keep(
        "advanced-secondary",
        || {
            PlotSeries::from_iter_points(
                "Signal B",
                (0..2000).map(|i| {
                    let x = i as f64 * 0.5;
                    PlotPoint::new(x, (x * 0.025).cos() * 18.0 + 46.0)
                }),
                SeriesKind::Line(LineStyle {
                    color: color_b,
                    width: 1.5,
                }),
            )
        },
        window,
        cx,
    );
    let picked = *selected.read(cx);
    let reset_now = *reset.read(cx);
    let select_state = selected.clone();
    let reset_state = reset.clone();
    section(
        "AdvancedChart",
        "Viewport-decimated continuous plot for dense streams. Drag to pan, scroll or pinch to zoom, right-drag for box zoom, click to select X or pin a point, and click legend items to toggle series.",
        cx,
    )
    .child(
        div().w(px(720.)).max_w_full().child(
            AdvancedChart::new("advanced-plot")
                .series(primary.read(cx).handle())
                .series(secondary.read(cx).handle())
                .x_axis(AxisConfig::builder().title("Sample").build())
                .y_axis(AxisConfig::builder().title("Output").build())
                .selected_x(picked)
                .reset_token(reset_now)
                .on_select(move |x, _, cx| set(&select_state, Some(*x), cx)),
        ),
    )
    .child(
        row().child(
            Button::new("advanced-reset-btn", "Reset view")
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, _, cx| set(&reset_state, reset_now + 1, cx)),
        ),
    )
    .children(picked.map(|x| Caption::new(format!("Selected X: {x:.1}"))))
}
