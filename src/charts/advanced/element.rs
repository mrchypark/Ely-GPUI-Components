use std::rc::Rc;

use gpui::{
    App, AppContext as _, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    Refineable, RenderOnce, StyleRefinement, Styled, Window, div,
};

use super::axis::AxisConfig;
use super::backend::{PlotHandlers, PlotLinkGroup, PlotLinkOptions, PlotView, PlotViewConfig};
use super::plot::Plot;
use super::series::{Series, SeriesId};
use super::style::Theme as PlotTheme;
use super::view::{View, Viewport};
use crate::theme::ActiveTheme;

type SelectHandler = Rc<dyn Fn(&f64, &mut Window, &mut App)>;
type CursorHandler = Rc<dyn Fn(&Option<f64>, &mut Window, &mut App)>;
type ViewportHandler = Rc<dyn Fn(&Option<Viewport>, &mut Window, &mut App)>;

struct ChartSlot {
    view: Entity<PlotView>,
    revision: Option<u64>,
    series_ids: Vec<SeriesId>,
    generations: Vec<u64>,
    link: Option<(PlotLinkGroup, PlotLinkOptions)>,
    last_selected_prop: Option<Option<f64>>,
    reset_token: Option<u64>,
}

struct ExternalSlot {
    last_selected_prop: Option<Option<f64>>,
    reset_token: Option<u64>,
}

fn push_series_preserving_id(dst: &mut Vec<Series>, series: &Series) {
    if dst.iter().any(|existing| existing.id() == series.id()) {
        dst.push(series.share());
    } else {
        dst.push(series.handle());
    }
}

fn same_link(
    current: &Option<(PlotLinkGroup, PlotLinkOptions)>,
    next: &Option<(PlotLinkGroup, PlotLinkOptions)>,
) -> bool {
    match (current, next) {
        (None, None) => true,
        (Some((g1, o1)), Some((g2, o2))) => g1.same_group(g2) && o1 == o2,
        _ => false,
    }
}

/// High-throughput interactive continuous and time-series plot with pan, zoom, hover, pins, and min-max decimation.
#[derive(IntoElement)]
pub struct AdvancedChart {
    base: Div,
    id: ElementId,
    external_view: Option<Entity<PlotView>>,
    series: Vec<Series>,
    x_axis: Option<AxisConfig>,
    y_axis: Option<AxisConfig>,
    view_mode: Option<View>,
    theme: Option<PlotTheme>,
    revision: Option<u64>,
    selected_x: Option<Option<f64>>,
    show_legend: bool,
    show_hover: bool,
    reset_token: Option<u64>,
    link: Option<(PlotLinkGroup, PlotLinkOptions)>,
    on_select: Option<SelectHandler>,
    on_cursor: Option<CursorHandler>,
    on_viewport_change: Option<ViewportHandler>,
}

impl AdvancedChart {
    /// Create a declarative advanced plot keyed by id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            external_view: None,
            series: Vec::new(),
            x_axis: None,
            y_axis: None,
            view_mode: None,
            theme: None,
            revision: None,
            selected_x: None,
            show_legend: true,
            show_hover: true,
            reset_token: None,
            link: None,
            on_select: None,
            on_cursor: None,
            on_viewport_change: None,
        }
    }

    /// Wrap an existing PlotView entity in an Ely-styled chart container.
    pub fn from_view(id: impl Into<ElementId>, view: Entity<PlotView>) -> Self {
        let mut chart = Self::new(id);
        chart.external_view = Some(view);
        chart
    }

    /// Add a data series; shares underlying append-only storage without copying points.
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Add multiple data series at once.
    pub fn series_iter(mut self, series: impl IntoIterator<Item = Series>) -> Self {
        self.series.extend(series);
        self
    }

    /// Configure the X axis.
    pub fn x_axis(mut self, axis: AxisConfig) -> Self {
        self.x_axis = Some(axis);
        self
    }

    /// Configure the Y axis.
    pub fn y_axis(mut self, axis: AxisConfig) -> Self {
        self.y_axis = Some(axis);
        self
    }

    /// Set the initial or updated plot view mode.
    pub fn view_mode(mut self, view: View) -> Self {
        self.view_mode = Some(view);
        self
    }

    /// Override the automatic Ely theme with a custom plot theme.
    pub fn plot_theme(mut self, theme: PlotTheme) -> Self {
        self.theme = Some(theme);
        self
    }

    /// Set a revision key so series and axes only rebuild when the revision changes.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Drive a persistent vertical selection line at data coordinate x.
    pub fn selected_x(mut self, x: Option<f64>) -> Self {
        self.selected_x = Some(x);
        self
    }

    /// Show or hide the interactive legend overlay (defaults to true).
    pub fn show_legend(mut self, show: bool) -> Self {
        self.show_legend = show;
        self
    }

    /// Show or hide the hover coordinate readout (defaults to true).
    pub fn show_hover(mut self, show: bool) -> Self {
        self.show_hover = show;
        self
    }

    /// Incrementing this token resets manual pan and zoom to auto-fit.
    pub fn reset_token(mut self, token: u64) -> Self {
        self.reset_token = Some(token);
        self
    }

    /// Synchronize viewport, cursor, and brush with other plots in the same group.
    pub fn link_group(mut self, group: PlotLinkGroup, options: PlotLinkOptions) -> Self {
        self.link = Some((group, options));
        self
    }

    /// Register a callback when the user clicks or taps a data X position.
    pub fn on_select(mut self, handler: impl Fn(&f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Register a callback when the pointer cursor moves over the plot.
    pub fn on_cursor(
        mut self,
        handler: impl Fn(&Option<f64>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_cursor = Some(Rc::new(handler));
        self
    }

    /// Register a callback when pan, zoom, or reset changes the visible viewport.
    pub fn on_viewport_change(
        mut self,
        handler: impl Fn(&Option<Viewport>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_viewport_change = Some(Rc::new(handler));
        self
    }

    fn build_plot(&self) -> Plot {
        let mut builder = Plot::builder();
        if let Some(theme) = &self.theme {
            builder = builder.theme(theme.clone());
        }
        if let Some(x_axis) = &self.x_axis {
            builder = builder.x_axis(x_axis.clone());
        }
        if let Some(y_axis) = &self.y_axis {
            builder = builder.y_axis(y_axis.clone());
        }
        if let Some(view_mode) = self.view_mode {
            builder = builder.view(view_mode);
        }
        let mut plot = builder.build();
        for series in &self.series {
            push_series_preserving_id(plot.series_mut(), series);
        }
        plot
    }

    fn handlers(&self) -> PlotHandlers {
        PlotHandlers {
            on_select: self.on_select.clone(),
            on_cursor: self.on_cursor.clone(),
            on_viewport_change: self.on_viewport_change.clone(),
        }
    }
}

impl Styled for AdvancedChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for AdvancedChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handlers = self.handlers();
        let view = if let Some(external) = self.external_view.take() {
            let slot: Entity<ExternalSlot> =
                window.use_keyed_state((self.id.clone(), "advanced-chart-ext"), cx, |_, _| {
                    ExternalSlot {
                        last_selected_prop: None,
                        reset_token: self.reset_token,
                    }
                });
            let selected_changed = self
                .selected_x
                .is_some_and(|prop| slot.read(cx).last_selected_prop != Some(prop));
            let reset_changed = self
                .reset_token
                .is_some_and(|tok| slot.read(cx).reset_token != Some(tok));
            external.update(cx, |plot_view, cx| {
                plot_view.set_handlers(handlers);
                plot_view.set_show_legend(self.show_legend, cx);
                plot_view.set_show_hover(self.show_hover, cx);
                if selected_changed && let Some(selected) = self.selected_x {
                    plot_view.set_selected_x(selected, cx);
                }
                if reset_changed {
                    plot_view.reset_view_in_window(window, cx);
                }
            });
            if selected_changed {
                slot.update(cx, |s, _| s.last_selected_prop = self.selected_x);
            }
            if reset_changed {
                slot.update(cx, |s, _| s.reset_token = self.reset_token);
            }
            external
        } else {
            let slot: Entity<ChartSlot> =
                window.use_keyed_state((self.id.clone(), "advanced-chart"), cx, |_, cx| {
                    let plot = self.build_plot();
                    let config = PlotViewConfig {
                        show_legend: self.show_legend,
                        show_hover: self.show_hover,
                        ..Default::default()
                    };
                    let mut plot_view = PlotView::with_config(plot, config);
                    if let Some((group, options)) = self.link.clone() {
                        plot_view = plot_view.with_link_group(group, options);
                    }
                    plot_view.set_handlers(handlers.clone());
                    let view = cx.new(|_| plot_view);
                    if let Some(selected) = self.selected_x {
                        view.update(cx, |v, cx| v.set_selected_x(selected, cx));
                    }
                    ChartSlot {
                        view,
                        revision: self.revision,
                        series_ids: self.series.iter().map(Series::id).collect(),
                        generations: self.series.iter().map(Series::generation).collect(),
                        link: self.link.clone(),
                        last_selected_prop: self.selected_x,
                        reset_token: self.reset_token,
                    }
                });

            let series_ids: Vec<SeriesId> = self.series.iter().map(Series::id).collect();
            let generations: Vec<u64> = self.series.iter().map(Series::generation).collect();
            let (needs_rebuild, data_changed, link_changed) = {
                let s = slot.read(cx);
                (
                    s.revision != self.revision || s.series_ids != series_ids,
                    s.generations != generations,
                    !same_link(&s.link, &self.link),
                )
            };
            let view = slot.read(cx).view.clone();
            if needs_rebuild {
                let handle = view.read(cx).plot_handle();
                handle.write(|plot| {
                    if let Some(theme) = &self.theme {
                        plot.set_theme(theme.clone());
                    } else {
                        plot.clear_custom_theme();
                    }
                    plot.set_x_axis(self.x_axis.take().unwrap_or_default());
                    plot.set_y_axis(self.y_axis.take().unwrap_or_default());
                    if let Some(view_mode) = self.view_mode {
                        plot.set_view(view_mode);
                    }
                    let dst = plot.series_mut();
                    dst.clear();
                    for series in &self.series {
                        push_series_preserving_id(dst, series);
                    }
                });
                slot.update(cx, |s, _| {
                    s.revision = self.revision;
                    s.series_ids = series_ids;
                    s.generations = generations;
                });
            } else if data_changed {
                slot.update(cx, |s, _| {
                    s.generations = generations;
                });
            }

            let selected_changed = self
                .selected_x
                .is_some_and(|prop| slot.read(cx).last_selected_prop != Some(prop));
            let reset_changed = self
                .reset_token
                .is_some_and(|tok| slot.read(cx).reset_token != Some(tok));

            view.update(cx, |plot_view, cx| {
                plot_view.set_handlers(handlers);
                plot_view.set_show_legend(self.show_legend, cx);
                plot_view.set_show_hover(self.show_hover, cx);
                if needs_rebuild && link_changed {
                    plot_view.set_link_group(self.link.clone(), cx);
                }
                if selected_changed && let Some(prop) = self.selected_x {
                    plot_view.set_selected_x(prop, cx);
                }
                if reset_changed {
                    plot_view.reset_view_in_window(window, cx);
                }
                if needs_rebuild || data_changed {
                    cx.notify();
                }
            });
            if needs_rebuild && link_changed {
                slot.update(cx, |s, _| s.link = self.link);
            }
            if selected_changed {
                slot.update(cx, |s, _| s.last_selected_prop = self.selected_x);
            }
            if reset_changed {
                slot.update(cx, |s, _| s.reset_token = self.reset_token);
            }
            view
        };

        let mut frame = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(cx.theme().chart().height)
            .overflow_hidden();
        frame.style().refine(self.base.style());
        frame.child(view)
    }
}
