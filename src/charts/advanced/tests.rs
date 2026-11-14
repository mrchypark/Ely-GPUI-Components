use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    AppContext as _, Context, IntoElement, MouseButton, ParentElement, Render, ScrollDelta,
    ScrollEvent, ScrollSource, Styled, TestAppContext, TouchPhase, VisualTestContext, Window, div,
    point, px,
};

use super::{
    AdvancedChart, AxisConfig, LineStyle, Plot, PlotView, Point, Series, SeriesKind, Theme,
    Viewport, series_color,
};
use crate::theme::{ActiveTheme, Mode, Theme as ElyTheme};

struct ChartHarness {
    selected: Rc<Cell<Option<f64>>>,
    viewport: Rc<RefCell<Option<Viewport>>>,
    revision: u64,
    series: Series,
    reset_token: u64,
}

impl Render for ChartHarness {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.clone();
        let viewport = self.viewport.clone();
        div().w(px(640.0)).h(px(320.0)).child(
            AdvancedChart::new("adv-test")
                .series(self.series.handle())
                .x_axis(AxisConfig::builder().title("Time").build())
                .y_axis(AxisConfig::builder().title("Power").build())
                .revision(self.revision)
                .selected_x(self.selected.get())
                .reset_token(self.reset_token)
                .on_select(cx.listener(move |this, x, _, cx| {
                    selected.set(Some(*x));
                    this.selected.set(Some(*x));
                    cx.notify();
                }))
                .on_viewport_change(move |vp, _, _| {
                    *viewport.borrow_mut() = *vp;
                })
                .h(px(280.0)),
        )
    }
}

#[gpui::test]
fn advanced_chart_renders_selects_zooms_and_updates_theme(cx: &mut TestAppContext) {
    let selected = Rc::new(Cell::new(None));
    let viewport = Rc::new(RefCell::new(None));
    let series = Series::from_iter_points(
        "panel-01",
        (0..1000).map(|i| Point::new(i as f64, (i as f64 * 0.05).sin() * 50.0 + 50.0)),
        SeriesKind::Line(LineStyle::default()),
    );
    let view = cx.add_window(|_, cx| {
        ElyTheme::init(cx);
        ChartHarness {
            selected: selected.clone(),
            viewport: viewport.clone(),
            revision: 1,
            series,
            reset_token: 0,
        }
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    cx.run_until_parked();

    let center = point(px(340.0), px(140.0));
    cx.simulate_mouse_down(center, MouseButton::Left, Default::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Default::default());
    cx.run_until_parked();
    assert!(selected.get().is_some(), "click should emit on_select");

    cx.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    cx.run_until_parked();
    let zoomed = (*viewport.borrow()).expect("wheel scroll should zoom viewport");
    assert!(zoomed.x.span() < 1000.0);

    cx.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(40.0), px(0.0))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    cx.run_until_parked();
    let panned = (*viewport.borrow()).expect("touch scroll should pan viewport");
    assert!(panned.x.min < zoomed.x.min);

    cx.update(|_, cx| {
        ElyTheme::set_mode_now(Mode::Dark, cx);
        let plot_theme = Theme::from_ely(cx.theme());
        assert_eq!(plot_theme.background, cx.theme().colors.bg.into());
        let c0 = series_color(0, cx);
        assert_eq!(c0, cx.theme().colors.chart[0].into());
    });
}

#[gpui::test]
fn rectangle_zoom_tracks_a_right_drag_outside_the_chart(cx: &mut TestAppContext) {
    let selected = Rc::new(Cell::new(None));
    let viewport = Rc::new(RefCell::new(None));
    let series = Series::from_iter_y("panel", 0..1000, SeriesKind::Line(LineStyle::default()));
    let view = cx.add_window(|_, cx| {
        ElyTheme::init(cx);
        ChartHarness {
            selected,
            viewport: viewport.clone(),
            revision: 1,
            series,
            reset_token: 0,
        }
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    cx.simulate_resize(gpui::size(px(900.0), px(500.0)));
    cx.run_until_parked();
    let start = point(px(200.0), px(80.0));
    let inside = point(px(300.0), px(140.0));
    let outside = point(px(700.0), px(200.0));
    cx.simulate_mouse_down(start, MouseButton::Right, Default::default());
    cx.simulate_mouse_move(inside, MouseButton::Right, Default::default());
    cx.simulate_mouse_move(outside, MouseButton::Right, Default::default());
    cx.simulate_mouse_up(outside, MouseButton::Right, Default::default());
    cx.run_until_parked();
    let zoomed = viewport
        .borrow()
        .expect("outside release commits rectangle zoom");
    assert!(
        zoomed.x.span() > 500.0,
        "the outside drag extends the selected range"
    );
}

#[gpui::test]
fn pan_tracks_a_left_drag_outside_the_chart(cx: &mut TestAppContext) {
    let viewport = Rc::new(RefCell::new(None));
    let view = cx.add_window(|_, cx| {
        ElyTheme::init(cx);
        ChartHarness {
            selected: Rc::new(Cell::new(None)),
            viewport: viewport.clone(),
            revision: 1,
            series: Series::from_iter_y("panel", 0..1000, SeriesKind::Line(LineStyle::default())),
            reset_token: 0,
        }
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    cx.simulate_resize(gpui::size(px(900.0), px(500.0)));
    cx.run_until_parked();
    let start = point(px(200.0), px(80.0));
    let inside = point(px(300.0), px(140.0));
    let outside = point(px(700.0), px(200.0));
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(inside, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(outside, MouseButton::Left, Default::default());
    cx.simulate_mouse_up(outside, MouseButton::Left, Default::default());
    cx.run_until_parked();
    let panned = viewport.borrow().expect("outside drag pans the viewport");
    assert!(panned.x.min < -500.0, "outside movement continues the pan");
}

#[gpui::test]
fn advanced_chart_from_view_wraps_existing_plot_view(cx: &mut TestAppContext) {
    struct Wrapped(gpui::Entity<PlotView>);
    impl Render for Wrapped {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.0)).h(px(280.0)).child(
                AdvancedChart::from_view("wrapped", self.0.clone())
                    .show_legend(false)
                    .show_hover(false),
            )
        }
    }

    let window = cx.add_window(|_, cx| {
        ElyTheme::init(cx);
        let mut plot = Plot::new();
        plot.add_series(&Series::from_iter_y(
            "s",
            [1.0, 4.0, 2.0, 8.0],
            SeriesKind::Line(LineStyle::default()),
        ));
        Wrapped(cx.new(|_| PlotView::new(plot)))
    });
    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
}

#[gpui::test]
fn advanced_chart_preserves_series_id_and_handles_replace_append_and_reset(
    cx: &mut TestAppContext,
) {
    struct DynamicHarness {
        series_list: Vec<Series>,
        revision: u64,
        custom_theme: Option<Theme>,
        x_title: &'static str,
        viewport: Rc<RefCell<Option<Viewport>>>,
        reset_token: u64,
    }

    impl Render for DynamicHarness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let viewport = self.viewport.clone();
            let mut chart = AdvancedChart::new("dyn-test")
                .series_iter(self.series_list.iter().map(Series::handle))
                .x_axis(AxisConfig::builder().title(self.x_title).build())
                .revision(self.revision)
                .reset_token(self.reset_token)
                .on_viewport_change(move |vp, _, _| {
                    *viewport.borrow_mut() = *vp;
                })
                .h(px(260.0));
            if let Some(theme) = &self.custom_theme {
                chart = chart.plot_theme(theme.clone());
            }
            div().w(px(320.0)).h(px(280.0)).child(chart)
        }
    }

    let viewport = Rc::new(RefCell::new(None));
    let first = Series::from_iter_y(
        "First",
        [1.0, 2.0, 3.0, 4.0, 5.0],
        SeriesKind::Line(LineStyle::default()),
    );
    let first_id = first.id();
    let replacement = Series::from_iter_y(
        "Replacement",
        [10.0, 20.0, 30.0],
        SeriesKind::Line(LineStyle::default()),
    );
    let replacement_id = replacement.id();

    let window = cx.add_window(|_, cx| {
        ElyTheme::init(cx);
        DynamicHarness {
            series_list: vec![first],
            revision: 1,
            custom_theme: Some(Theme::dark()),
            x_title: "Initial",
            viewport: viewport.clone(),
            reset_token: 0,
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Zoom manually so we can verify external append preserves manual viewport.
    cx.simulate_event(ScrollEvent {
        position: point(px(160.0), px(130.0)),
        delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    cx.run_until_parked();
    let zoomed = (*viewport.borrow()).expect("zoom should produce manual viewport");

    // External append + rerender must preserve zoomed viewport and update bounds.
    window
        .update(&mut cx, |harness, _window, cx| {
            harness.series_list[0].extend_y([6.0, 7.0, 8.0]).unwrap();
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(*viewport.borrow(), Some(zoomed));
    window
        .read_with(&cx, |harness, _| {
            let bounds = harness.series_list[0].bounds().unwrap();
            assert_eq!(bounds.x.max, 7.0);
            assert_eq!(harness.series_list[0].id(), first_id);
        })
        .unwrap();

    // Replacing series and clearing custom theme on revision bump updates cleanly.
    window
        .update(&mut cx, |harness, _window, cx| {
            harness.series_list = vec![replacement];
            harness.revision = 2;
            harness.custom_theme = None;
            harness.x_title = "Updated";
            harness.reset_token = 1;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        *viewport.borrow(),
        None,
        "reset_token should clear manual viewport"
    );
    assert_ne!(first_id, replacement_id);
}

#[gpui::test]
fn advanced_chart_supports_log_and_symlog_axes_during_zoom_and_pan(cx: &mut TestAppContext) {
    struct ScaleHarness {
        viewport: Rc<RefCell<Option<Viewport>>>,
        log_series: Series,
    }

    impl Render for ScaleHarness {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let vp = self.viewport.clone();
            div().w(px(480.0)).h(px(280.0)).child(
                AdvancedChart::new("scale-adv")
                    .series(self.log_series.handle())
                    .x_axis(AxisConfig::builder().log(10.0).build())
                    .y_axis(AxisConfig::builder().symlog(1.0).build())
                    .on_viewport_change(move |v, _, _| *vp.borrow_mut() = *v)
                    .h(px(240.0)),
            )
        }
    }

    let viewport = Rc::new(RefCell::new(None));
    let log_series = Series::from_iter_points(
        "decades",
        [
            Point::new(1.0, -100.0),
            Point::new(10.0, 0.0),
            Point::new(100.0, 10.0),
            Point::new(1000.0, 1000.0),
        ],
        SeriesKind::Line(LineStyle::default()),
    );
    let view = cx.add_window(|_, cx| {
        ElyTheme::init(cx);
        ScaleHarness {
            viewport: viewport.clone(),
            log_series,
        }
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    cx.run_until_parked();

    let center = point(px(240.0), px(120.0));
    cx.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Pointer,
    });
    cx.run_until_parked();
    let zoomed = (*viewport.borrow()).expect("log/symlog zoom produces viewport");
    assert!(
        zoomed.x.min > 0.0,
        "log x range must stay strictly positive"
    );
    assert!(zoomed.x.span() < 999.0);

    cx.simulate_event(ScrollEvent {
        position: center,
        delta: ScrollDelta::Pixels(point(px(30.0), px(0.0))),
        modifiers: Default::default(),
        touch_phase: TouchPhase::Moved,
        source: ScrollSource::Touch,
    });
    cx.run_until_parked();
    let panned = (*viewport.borrow()).expect("touch pan produces viewport");
    assert!(panned.x.min > 0.0 && panned.x.min < zoomed.x.min);
}
