use std::rc::Rc;
use std::sync::{Arc, RwLock};

use gpui::prelude::*;
use gpui::{
    App, Bounds, DragMoveEvent, EmptyView, EventEmitter, MouseButton, PinchEvent, Pixels, Point,
    SharedString, Window, canvas, div,
};

use crate::charts::advanced::interaction::zoom_viewport_with_scales;
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::style::Theme as PlotTheme;
use crate::charts::advanced::transform::Transform;
use crate::charts::advanced::view::{Range, Viewport};
use crate::theme::{ActiveTheme, Theme as ElyTheme};

use super::config::PlotViewConfig;
use super::frame::build_frame;
use super::link::{LinkBinding, PlotLinkGroup, PlotLinkOptions, ViewSyncKind};
use super::paint::{paint_frame, to_hsla};
use super::state::PlotUiState;

mod input;
mod scroll;
#[cfg(test)]
mod tests;

use input::{PlotDrag, reset_link_state};
use scroll::{ViewportCenter, apply_manual_view, screen_point};

/// Events emitted by PlotView when the user interacts with a chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlotEvent {
    /// Pointer hover X position in data coordinates, or None when cleared.
    CursorX(Option<f64>),
    /// Primary click/tap X position in data coordinates.
    SelectedX(f64),
    /// Visible data viewport changed by pan, zoom, or reset (None).
    ViewportChanged(Option<Viewport>),
}

/// Callback invoked when a data X position is clicked or tapped.
pub type SelectHandler = Rc<dyn Fn(&f64, &mut Window, &mut App)>;

/// Callback invoked when the pointer cursor moves across the plot.
pub type CursorHandler = Rc<dyn Fn(&Option<f64>, &mut Window, &mut App)>;

/// Callback invoked when the visible viewport changes.
pub type ViewportHandler = Rc<dyn Fn(&Option<Viewport>, &mut Window, &mut App)>;

/// Optional window-deferred callbacks attached by declarative wrappers.
#[derive(Clone, Default)]
pub struct PlotHandlers {
    pub on_select: Option<SelectHandler>,
    pub on_cursor: Option<CursorHandler>,
    pub on_viewport_change: Option<ViewportHandler>,
}

/// Interactive GPUI view that renders a Plot with pan, zoom, hover, and pins.
#[derive(Clone)]
pub struct PlotView {
    pub(super) plot: Arc<RwLock<Plot>>,
    pub(super) state: Arc<RwLock<PlotUiState>>,
    pub(super) config: PlotViewConfig,
    pub(super) link: Option<LinkBinding>,
    pub(super) handlers: PlotHandlers,
}

impl EventEmitter<PlotEvent> for PlotView {}

impl PlotView {
    /// Create a new GPUI plot view with default configuration.
    pub fn new(plot: Plot) -> Self {
        Self {
            plot: Arc::new(RwLock::new(plot)),
            state: Arc::new(RwLock::new(PlotUiState::default())),
            config: PlotViewConfig::default(),
            link: None,
            handlers: PlotHandlers::default(),
        }
    }

    /// Create a new GPUI plot view with a custom configuration.
    pub fn with_config(plot: Plot, config: PlotViewConfig) -> Self {
        Self {
            plot: Arc::new(RwLock::new(plot)),
            state: Arc::new(RwLock::new(PlotUiState::default())),
            config,
            link: None,
            handlers: PlotHandlers::default(),
        }
    }

    /// Attach this view to a multi-plot link group.
    pub fn with_link_group(mut self, group: PlotLinkGroup, options: PlotLinkOptions) -> Self {
        self.link = Some(LinkBinding {
            member_id: group.register_member(),
            group,
            options,
        });
        self
    }

    /// Update or clear the multi-plot link group at runtime.
    pub fn set_link_group(
        &mut self,
        group: Option<(PlotLinkGroup, PlotLinkOptions)>,
        cx: &mut Context<Self>,
    ) {
        self.link = group.map(|(group, options)| LinkBinding {
            member_id: group.register_member(),
            group,
            options,
        });
        reset_link_state(&mut self.state.write().expect("plot state lock"));
        cx.notify();
    }

    /// Update window-deferred event handlers.
    pub fn set_handlers(&mut self, handlers: PlotHandlers) {
        self.handlers = handlers;
    }

    /// Get a handle for mutating the underlying plot.
    pub fn plot_handle(&self) -> PlotHandle {
        PlotHandle {
            plot: Arc::clone(&self.plot),
        }
    }

    /// Current window-relative view bounds after the latest layout pass.
    pub fn bounds(&self) -> Option<Bounds<Pixels>> {
        self.state.read().expect("plot state lock").view_bounds
    }

    /// Set or clear a persistent vertical marker at data coordinate x.
    pub fn set_selected_x(&mut self, x: Option<f64>, cx: &mut Context<Self>) {
        let next = x.filter(|v| v.is_finite());
        let mut state = self.state.write().expect("plot state lock");
        if state.selected_x != next {
            state.selected_x = next;
            drop(state);
            cx.notify();
        }
    }

    /// Request a redraw after mutating an underlying shared series externally.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Show or hide the in-plot legend overlay.
    pub fn set_show_legend(&mut self, show: bool, cx: &mut Context<Self>) {
        if self.config.show_legend != show {
            self.config.show_legend = show;
            cx.notify();
        }
    }

    /// Show or hide hover coordinate readouts.
    pub fn set_show_hover(&mut self, show: bool, cx: &mut Context<Self>) {
        if self.config.show_hover != show {
            self.config.show_hover = show;
            cx.notify();
        }
    }

    /// Reset manual pan/zoom, notify any linked views, and emit ViewportChanged(None).
    pub fn reset_view(&mut self, cx: &mut Context<Self>) {
        {
            let mut state = self.state.write().expect("plot state lock");
            if let Ok(mut plot) = self.plot.write() {
                plot.reset_view();
            }
            state.linked_brush_x = None;
            state.clear_interaction();
            self.publish_reset_link();
            self.publish_brush_link(None);
        }
        cx.emit(PlotEvent::ViewportChanged(None));
        cx.notify();
    }

    pub(crate) fn reset_view_in_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.reset_view(cx);
        self.defer_viewport(None, window, cx);
    }

    /// Zoom the viewport by multiplicative scale factors around screen_pos.
    pub fn zoom_by(
        &mut self,
        factor_x: f32,
        factor_y: f32,
        screen_pos: Option<Point<Pixels>>,
        cx: &mut Context<Self>,
    ) {
        self.zoom_by_in_window(factor_x, factor_y, screen_pos, None, cx);
    }

    pub(super) fn zoom_by_in_window(
        &mut self,
        factor_x: f32,
        factor_y: f32,
        screen_pos: Option<Point<Pixels>>,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if !factor_x.is_finite() || !factor_y.is_finite() || factor_x <= 0.0 || factor_y <= 0.0 {
            return;
        }
        let mut changed = None;
        {
            let mut state = self.state.write().expect("plot state lock");
            let Some(transform) = state.transform.clone() else {
                return;
            };
            let Some(rect) = state.plot_rect else {
                return;
            };
            if let Ok(mut plot) = self.plot.write()
                && let Some(viewport) = plot.viewport()
            {
                let center = screen_pos
                    .and_then(|pos| transform.screen_to_data(screen_point(pos)))
                    .unwrap_or_else(|| viewport.center());
                let next = zoom_viewport_with_scales(
                    viewport,
                    center,
                    (1.0 / f64::from(factor_x)).clamp(0.1, 10.0),
                    (1.0 / f64::from(factor_y)).clamp(0.1, 10.0),
                    transform.x_scale(),
                    transform.y_scale(),
                );
                self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
                changed = Some(next);
            }
        }
        if let Some(next) = changed {
            cx.emit(PlotEvent::ViewportChanged(Some(next)));
            if let Some(window) = window {
                self.defer_viewport(Some(next), window, cx);
            }
            cx.notify();
        }
    }

    pub(super) fn defer_select(&self, x: f64, window: &mut Window, cx: &mut App) {
        if let Some(cb) = self.handlers.on_select.clone() {
            window.defer(cx, move |window, cx| cb(&x, window, cx));
        }
    }

    pub(super) fn defer_cursor(&self, x: Option<f64>, window: &mut Window, cx: &mut App) {
        if let Some(cb) = self.handlers.on_cursor.clone() {
            window.defer(cx, move |window, cx| cb(&x, window, cx));
        }
    }

    pub(super) fn defer_viewport(
        &self,
        viewport: Option<Viewport>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(cb) = self.handlers.on_viewport_change.clone() {
            window.defer(cx, move |window, cx| cb(&viewport, window, cx));
        }
    }
}

impl Render for PlotView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let plot = Arc::clone(&self.plot);
        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let link = self.link.clone();
        let theme = {
            let guard = plot.read().expect("plot lock");
            if guard.has_custom_theme() {
                guard.theme().clone()
            } else {
                resolve_ely_theme(guard.theme().clone(), cx)
            }
        };
        let hover_region_id = Arc::as_ptr(&self.state) as usize;
        let owner = cx.entity_id();
        let right_drag = cx.entity().downgrade();

        div()
            .id(("gpui-plot-view", hover_region_id))
            .size_full()
            .bg(to_hsla(theme.background))
            .child(
                canvas(
                    move |bounds, window, app| {
                        let mut plot = plot.write().expect("plot lock");
                        let mut state = state.write().expect("plot state lock");
                        if !plot.has_custom_theme()
                            && let Some(ely_theme) = resolve_ely_theme_opt(app)
                        {
                            plot.apply_theme(ely_theme);
                        }
                        if let Some(link) = &link {
                            apply_link_updates(link, &mut plot, &mut state);
                        }
                        let font_family: SharedString = if app.has_global::<ElyTheme>() {
                            app.theme().font_family.clone()
                        } else {
                            ".SystemUIFont".into()
                        };
                        build_frame(&mut plot, &mut state, &config, bounds, window, font_family)
                    },
                    move |_, frame, window, cx| {
                        paint_frame(&frame, window, cx);
                        input::track_outside_rect_drag(right_drag.clone(), window);
                    },
                )
                .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev, window, cx| {
                    this.on_mouse_down(ev, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, ev, window, cx| {
                    this.on_mouse_down(ev, window, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, ev, window, cx| {
                this.on_mouse_move(ev, window, cx);
            }))
            .on_drag(PlotDrag { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<PlotDrag>, window, cx| {
                    if event.drag(cx).owner != owner {
                        return;
                    }
                    this.on_mouse_move(&event.event, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_hover(cx.listener(|this, hovered, window, cx| {
                this.on_hover_state_change(*hovered, window, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev, window, cx| {
                    this.on_mouse_up(ev, window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, ev, window, cx| {
                    this.on_mouse_up(ev, window, cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, ev, window, cx| {
                    this.on_mouse_up_out(ev, window, cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Right,
                cx.listener(|this, ev, window, cx| {
                    this.on_mouse_up_out(ev, window, cx);
                }),
            )
            .on_scroll(cx.listener(|this, ev, window, cx| {
                this.on_scroll(ev, window, cx);
            }))
            .on_pinch(cx.listener(|this, ev: &PinchEvent, window, cx| {
                let factor = scroll::pinch_zoom_scale(ev.delta);
                if (factor - 1.0).abs() > f32::EPSILON {
                    this.zoom_by_in_window(factor, factor, Some(ev.position), Some(window), cx);
                    cx.stop_propagation();
                }
            }))
    }
}

fn resolve_ely_theme_opt(cx: &App) -> Option<PlotTheme> {
    cx.has_global::<ElyTheme>()
        .then(|| PlotTheme::from_ely(cx.theme()))
}

fn resolve_ely_theme(base: PlotTheme, cx: &App) -> PlotTheme {
    resolve_ely_theme_opt(cx).unwrap_or(base)
}

/// Handle for mutating a Plot held inside a PlotView.
#[derive(Debug, Clone)]
pub struct PlotHandle {
    plot: Arc<RwLock<Plot>>,
}

impl PlotHandle {
    /// Read the plot state while locked.
    pub fn read<R>(&self, f: impl FnOnce(&Plot) -> R) -> R {
        let plot = self.plot.read().expect("plot lock");
        f(&plot)
    }

    /// Mutate the plot state while locked.
    pub fn write<R>(&self, f: impl FnOnce(&mut Plot) -> R) -> R {
        let mut plot = self.plot.write().expect("plot lock");
        f(&mut plot)
    }
}

fn apply_link_updates(link: &LinkBinding, plot: &mut Plot, state: &mut PlotUiState) {
    if let Some(update) = link.group.latest_view_update()
        && update.seq > state.link_view_seq
    {
        state.link_view_seq = update.seq;
        if update.source != link.member_id {
            match update.kind {
                ViewSyncKind::Reset => {
                    if link.options.link_reset {
                        plot.reset_view();
                        state.viewport = None;
                        state.transform = None;
                        state.linked_brush_x = None;
                    }
                }
                ViewSyncKind::Manual {
                    viewport,
                    sync_x,
                    sync_y,
                } => {
                    let mut next = plot
                        .viewport()
                        .or_else(|| plot.data_bounds())
                        .unwrap_or(viewport);
                    let mut changed = false;
                    if sync_x && link.options.link_x {
                        next.x = viewport.x;
                        changed = true;
                    }
                    if sync_y && link.options.link_y {
                        next.y = viewport.y;
                        changed = true;
                    }
                    if changed {
                        plot.set_manual_view(next);
                        state.viewport = Some(next);
                        if let Some(rect) = state.plot_rect {
                            state.transform = Transform::with_scales(
                                next,
                                rect,
                                plot.x_axis().scale(),
                                plot.y_axis().scale(),
                            );
                        }
                    }
                }
            }
        }
    }

    if let Some(update) = link.group.latest_cursor_update()
        && update.seq > state.link_cursor_seq
    {
        state.link_cursor_seq = update.seq;
        if update.source != link.member_id && link.options.link_cursor {
            state.linked_cursor_x = update.x;
        }
    }

    if let Some(update) = link.group.latest_brush_update()
        && update.seq > state.link_brush_seq
    {
        state.link_brush_seq = update.seq;
        if update.source != link.member_id && link.options.link_brush {
            state.linked_brush_x = update.x_range;
            if let Some(x_range) = update.x_range {
                let y_range = plot
                    .viewport()
                    .or_else(|| plot.data_bounds())
                    .map(|viewport| viewport.y)
                    .unwrap_or_else(|| Range::new(0.0, 1.0));
                let next = Viewport::new(x_range, y_range);
                plot.set_manual_view(next);
                state.viewport = Some(next);
                if let Some(rect) = state.plot_rect {
                    state.transform = Transform::with_scales(
                        next,
                        rect,
                        plot.x_axis().scale(),
                        plot.y_axis().scale(),
                    );
                }
            }
        }
    }
}
