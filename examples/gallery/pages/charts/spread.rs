use ely_gpui_component::charts::{
    BoxPlot, ContourChart, DifferenceChart, HexbinChart, Histogram, RidgelineChart, Series,
    ViolinPlot, WaterfallChart,
};
use gpui::{App, IntoElement, ParentElement, Styled, div, px};

use crate::ui::{noise, section};

pub fn distributions(cx: &mut App) -> impl IntoElement + use<> {
    let mut rng = noise(13);
    let mut normal =
        |mean: f64, spread: f64| -> f64 { mean + spread * ((rng() + rng() + rng() + rng()) - 2.0) };
    let times: Vec<f64> = (0..240).map(|_| normal(320.0, 120.0).max(40.0)).collect();
    let hex_points: Vec<(f64, f64)> = (0..400)
        .map(|ix| {
            let (mx, my) = if ix % 3 == 0 {
                (68.0, 72.0)
            } else {
                (34.0, 42.0)
            };
            (
                normal(mx, 14.0).clamp(5.0, 95.0),
                normal(my, 16.0).clamp(5.0, 95.0),
            )
        })
        .collect();
    let groups: Vec<(&str, Vec<f64>)> = [("Europe", 220.0), ("Americas", 310.0), ("Asia", 270.0)]
        .into_iter()
        .map(|(name, mean)| (name, (0..60).map(|_| normal(mean, 90.0)).collect()))
        .collect();
    let boxes = groups
        .iter()
        .fold(BoxPlot::new("boxes"), |plot, (name, values)| {
            plot.group(*name, values.clone())
        });
    let violins = groups
        .iter()
        .fold(ViolinPlot::new("violins"), |plot, (name, values)| {
            plot.group(*name, values.clone())
        });
    let contour_points = hex_points.clone();
    let ridgeline = groups
        .iter()
        .fold(RidgelineChart::new("ridgeline"), |chart, (name, values)| {
            chart.group(*name, values.clone())
        });
    let diff = DifferenceChart::new("diff-temp", ["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
        .primary(Series::new("Actual", [14.0, 18.0, 16.0, 24.0, 28.0, 22.0]))
        .baseline(Series::new("Target", [15.0, 16.0, 19.0, 21.0, 25.0, 26.0]));
    section(
        "Histogram / BoxPlot / ViolinPlot",
        "Response times by count in equal bins, and by region as boxes and as violins that widen where values gather.",
        cx,
    )
    .child(div().w(px(720.)).max_w_full().child(Histogram::new("times", times).bins(12)))
    .child(div().flex().flex_wrap().gap_6().child(div().w(px(360.)).max_w_full().child(boxes)).child(div().w(px(360.)).max_w_full().child(violins)))
    .child(
        div().w(px(720.)).max_w_full().child(
            HexbinChart::new("hexbin-density")
                .points(hex_points)
                .radius(14.0)
                .axes("Latency (ms)", "Queue depth"),
        ),
    )
    .child(
        div().w(px(720.)).max_w_full().child(
            ContourChart::from_points("contour-density", contour_points)
                .levels(6)
                .axes("Latency (ms)", "Queue depth"),
        ),
    )
    .child(div().flex().flex_wrap().gap_6().child(div().w(px(360.)).max_w_full().child(ridgeline)).child(div().w(px(360.)).max_w_full().child(diff)))
}

pub fn waterfall(cx: &mut App) -> impl IntoElement + use<> {
    section(
        "WaterfallChart",
        "How a year's revenue became its profit: each step floats from where the last left off, and the totals stand on zero.",
        cx,
    )
    .child(
        div().w(px(720.)).child(
            WaterfallChart::new("profit")
                .step("Revenue", 420.0)
                .step("Refunds", -36.0)
                .total("Net sales")
                .step("Cost of sales", -148.0)
                .step("Operating", -121.0)
                .step("Other income", 18.0)
                .total("Profit"),
        ),
    )
}
