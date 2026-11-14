use super::super::axes::PAD;
use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};
use jiff::civil::date;

use super::super::*;
use super::settle;
use crate::{layout::tests::narrow_width, theme::Theme};

#[gpui::test]
fn every_chart_fills_a_column_its_block_measures_by_content(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let two = || Series::new("s", [1.0, 2.0]);
    let pairs = [vec![0.0, 1.0], vec![1.0, 0.0]];
    let widths = [
        narrow_width(cx, "chart-root", move |_, _| {
            LineChart::new("line", ["a", "b"])
                .series(two())
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            AreaChart::new("area", ["a", "b"])
                .series(two())
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            BarChart::new("bar", ["a", "b"])
                .series(two())
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            let points = Points::new("p", [(0.0, 1.0), (1.0, 2.0)]);
            ScatterChart::new("scatter")
                .points(points)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            Histogram::new("histogram", [1.0, 2.0, 2.0, 3.0]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            WaterfallChart::new("waterfall")
                .step("a", 2.0)
                .step("b", -1.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            BoxPlot::new("box")
                .group("a", [1.0, 2.0, 3.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            ViolinPlot::new("violin")
                .group("a", [1.0, 2.0, 3.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            CalendarHeatmap::new("days", date(2026, 1, 5), [1.0; 14]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            SankeyChart::new("sankey", ["a", "b"])
                .link(0, 1, 5.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            FunnelChart::new("funnel")
                .stage("a", 2.0)
                .stage("b", 1.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            NetworkGraph::new("network")
                .node("a", 0)
                .node("b", 0)
                .edge(0, 1)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            ParallelCoordinates::new("parallel", ["a", "b"])
                .record("r", [1.0, 2.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            PieChart::new("pie")
                .slice("a", 1.0)
                .slice("b", 2.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            RadarChart::new("radar", ["a", "b", "c"])
                .series(Series::new("s", [1.0, 2.0, 3.0]))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            RealtimeChart::new("live", [1.0, 2.0], 2).into_any_element()
        }),
        narrow_width(cx, "chart-root", move |_, _| {
            ChordDiagram::new("chord", ["a", "b"], pairs.to_vec()).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            GanttChart::new("gantt")
                .task(Task::new("t", date(2026, 3, 1), date(2026, 3, 9)))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            Sunburst::new("sun", [Slice::new("a", 1.0)]).into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            BulletChart::new("bullets")
                .bullet(Bullet::new("b", 5.0, 8.0))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            Treemap::new("treemap")
                .tile("a", 1.0)
                .tile("b", 2.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            HeatmapChart::new("heatmap", ["a"])
                .row("a", [1.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            HexbinChart::new("hexbin")
                .points([(1.0, 2.0), (1.5, 2.5), (8.0, 9.0)])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            ContourChart::from_points("contour", [(1.0, 2.0), (2.0, 3.0), (4.0, 5.0)])
                .grid_size(10, 10)
                .levels(3)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            RidgelineChart::new("ridgeline")
                .group("a", [1.0, 2.0, 3.0])
                .group("b", [2.0, 3.0, 4.0])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            DifferenceChart::new("diff", ["a", "b"])
                .primary(Series::new("p", [3.0, 1.0]))
                .baseline(Series::new("b", [1.0, 3.0]))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            PolarChart::new("polar")
                .slice("a", 4.0)
                .slice("b", 8.0)
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            CirclePacking::new("pack", [Slice::new("a", 2.0), Slice::new("b", 4.0)])
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            HorizonChart::new("horizon", ["a", "b", "c"])
                .series(Series::new("s", [-4.0, 2.0, 8.0]))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            RadialLineChart::new("radial-line", ["Jan", "Apr", "Jul", "Oct"])
                .series(Series::new("Temp", [4.0, 14.0, 26.0, 12.0]))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            TreeChart::new(
                "tree",
                Slice::new("root", 0.0).children([Slice::new("a", 1.0), Slice::new("b", 2.0)]),
            )
            .mode(TreeLayoutMode::RadialTree)
            .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            GeoChart::new("geo")
                .projection(GeoProjection::Orthographic)
                .region(GeoRegion::new(
                    "Block",
                    10.0,
                    [(-10.0, -10.0), (20.0, -10.0), (20.0, 20.0), (-10.0, 20.0)],
                ))
                .point(GeoPoint::new("Seoul", 126.97, 37.56, 9.5))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            SurfaceChart::from_grid("surface", 4, 4, (0..16).map(|i| (i as f64).sin()))
                .into_any_element()
        }),
        narrow_width(cx, "chart-root", |_, _| {
            ParallelSetsChart::new("parsets", ["Class", "Survived"])
                .ribbon(["First", "Yes"], 120.0)
                .ribbon(["Third", "No"], 240.0)
                .into_any_element()
        }),
    ];
    assert_eq!(
        widths,
        [px(240.0); 34],
        "each chart spans the card inside its padding"
    );
}

#[gpui::test]
fn log_symlog_streamgraph_and_voronoi_render_and_export(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let log_line = LineChart::new("log", ["1", "2", "3"])
            .series(Series::new("Traffic", [10.0, 100.0, 1000.0]))
            .y_log(10.0);
        assert!(log_line.svg(320.0, 200.0, cx).contains("<path"));

        let symlog_line = LineChart::new("symlog", ["A", "B", "C"])
            .series(Series::new("Balance", [-100.0, 0.0, 250.0]))
            .y_symlog(1.0);
        assert!(symlog_line.svg(320.0, 200.0, cx).contains("<path"));

        let stream = AreaChart::new("stream", ["Q1", "Q2", "Q3"])
            .series(Series::new("A", [10.0, 20.0, 15.0]))
            .series(Series::new("B", [15.0, 10.0, 25.0]))
            .stream()
            .smooth();
        assert!(stream.svg(320.0, 200.0, cx).contains("<path"));

        let voronoi = ScatterChart::new("voronoi")
            .points(Points::new(
                "Sites",
                [(1.0, 10.0), (10.0, 100.0), (100.0, 1000.0)],
            ))
            .x_log(10.0)
            .y_log(10.0)
            .voronoi();
        let vsvg = voronoi.svg(320.0, 200.0, cx);
        assert!(vsvg.contains("<circle") && vsvg.contains("<line"));

        for curve in [
            Curve::Step,
            Curve::StepBefore,
            Curve::StepAfter,
            Curve::Basis,
            Curve::Cardinal,
        ] {
            let c_line = LineChart::new("curve", ["A", "B", "C", "D"])
                .series(Series::new("S", [1.0, 4.0, 2.0, 5.0]))
                .curve(curve);
            assert!(c_line.svg(300.0, 180.0, cx).contains("<path"));
        }

        let symbols = ScatterChart::new("symbols")
            .points(Points::new("D", [(1.0, 2.0), (3.0, 4.0)]).symbol(Symbol::Diamond))
            .points(Points::new("S", [(2.0, 5.0)]).symbol(Symbol::Star));
        assert!(symbols.svg(300.0, 180.0, cx).contains("<path"));
    });
}

/// A sideways bar chart `wide` across with one band named `name`.
struct Sideways(String, f32);

impl Render for Sideways {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(self.1)).child(
            BarChart::new("sideways", [self.0.clone()])
                .series(Series::new("s", [3.0]))
                .horizontal(),
        )
    }
}

#[gpui::test]
fn a_long_band_name_widens_the_gutter_to_half_the_chart(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let long = "x".repeat(200);
    for (name, wide) in [
        ("claude-sonnet-4-5-20250929", 400.0),
        (&long, 400.0),
        (&long, 80.0),
    ] {
        let (_, cx) = cx.add_window_view(|_, _| Sideways(name.to_string(), wide));
        settle(cx);
        let root = cx.debug_bounds("chart-root").expect("the chart draws");
        let band = cx.debug_bounds("chart-band").expect("the band is named");
        let half = root.left() + px(wide / 2.0);
        let pad = cx.update(|window, _| PAD.to_pixels(window.rem_size()));
        assert!(
            band.left() >= root.left(),
            "{wide}: the name's box starts in the chart"
        );
        assert!(
            band.right() + pad <= half,
            "{wide}: the gutter takes half at most"
        );
        if wide > 200.0 {
            assert!(band.size.width > px(48.0), "past the theme gutter");
        }
    }
}

/// A note, a link, an edge and a wait that name what their chart lacks.
struct Dangling;

impl Render for Dangling {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let line = LineChart::new("line", ["a", "b"])
            .series(Series::new("s", [1.0, 2.0]))
            .note(7, "late");
        let sankey = SankeyChart::new("sankey", ["a", "b"])
            .link(0, 9, 5.0)
            .link(0, 1, 5.0);
        let network = NetworkGraph::new("network")
            .node("a", 0)
            .node("b", 0)
            .edge(0, 9);
        let gantt = GanttChart::new("gantt")
            .task(Task::new("t", date(2026, 3, 1), date(2026, 3, 9)).after(4));
        div()
            .w(px(400.0))
            .child(line)
            .child(sankey)
            .child(network)
            .child(gantt)
    }
}

#[gpui::test]
fn entries_past_their_lists_are_skipped(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Dangling);
    settle(cx);
}
