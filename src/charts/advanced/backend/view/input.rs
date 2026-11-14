use std::time::Duration;

use gpui::{Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Window};
use web_time::Instant;

use crate::charts::advanced::geom::{ScreenPoint, ScreenRect};
use crate::charts::advanced::interaction::{
    HitRegion, pan_viewport, toggle_pin, zoom_factor_from_drag, zoom_to_rect,
    zoom_viewport_with_scales,
};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::view::{Range, Viewport};

use super::super::constants::DOUBLE_CLICK_PIN_GRACE_MS;
use super::super::geometry::{distance_sq, normalized_rect};
use super::super::hover::{compute_hover_target, hover_target_within_threshold};
use super::super::state::{ClickState, DragMode, DragState, PinToggle, PlotUiState};

#[derive(Clone, Copy)]
pub(super) struct PlotDrag {
    pub(super) owner: gpui::EntityId,
}

pub(super) fn reset_link_state(state: &mut PlotUiState) {
    state.link_view_seq = 0;
    state.link_cursor_seq = 0;
    state.link_brush_seq = 0;
    state.linked_cursor_x = None;
    state.linked_brush_x = None;
}
use super::scroll::revert_pin_toggle;
use super::{PlotEvent, PlotView, ViewportCenter, apply_manual_view, screen_point};

impl PlotView {
    pub(super) fn publish_manual_view_link(&self, viewport: Viewport) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        link.group.publish_manual_view(
            link.member_id,
            viewport,
            link.options.link_x,
            link.options.link_y,
        );
    }

    pub(super) fn publish_reset_link(&self) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        if link.options.link_reset {
            link.group.publish_reset(link.member_id);
        }
    }

    pub(super) fn publish_cursor_link(&self, x: Option<f64>) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        if link.options.link_cursor {
            link.group.publish_cursor_x(link.member_id, x);
        }
    }

    pub(super) fn publish_brush_link(&self, x_range: Option<Range>) {
        let Some(link) = self.link.as_ref() else {
            return;
        };
        if link.options.link_brush {
            link.group.publish_brush_x(link.member_id, x_range);
        }
    }

    pub(super) fn apply_manual_view_with_link(
        &self,
        plot: &mut Plot,
        state: &mut PlotUiState,
        rect: ScreenRect,
        viewport: Viewport,
    ) {
        apply_manual_view(plot, state, rect, viewport);
        state.linked_brush_x = None;
        self.publish_manual_view_link(viewport);
        self.publish_brush_link(None);
    }

    pub(super) fn on_mouse_down(
        &mut self,
        ev: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = screen_point(ev.position);
        let mut state = self.state.write().expect("plot state lock");
        state.last_cursor = Some(pos);

        if let Some(series_id) = state.legend_hit(pos) {
            if ev.button == MouseButton::Left
                && ev.click_count == 1
                && let Ok(mut plot) = self.plot.write()
                && let Some(series) = plot
                    .series_mut()
                    .iter_mut()
                    .find(|series| series.id() == series_id)
            {
                series.set_visible(!series.is_visible());
            }
            state.clear_interaction();
            state.hover = None;
            state.hover_target = None;
            cx.notify();
            return;
        }

        let region = state.regions.hit_test(pos);
        if ev.button == MouseButton::Left && ev.click_count >= 2 && region == HitRegion::Plot {
            let last_toggle = state.last_pin_toggle.take();
            if let Ok(mut plot) = self.plot.write() {
                if let Some(last_toggle) = last_toggle
                    && last_toggle.at.elapsed() <= Duration::from_millis(DOUBLE_CLICK_PIN_GRACE_MS)
                    && distance_sq(last_toggle.screen_pos, pos)
                        <= self.config.pin_threshold_px.powi(2)
                {
                    revert_pin_toggle(&mut plot, last_toggle);
                }
                plot.reset_view();
                state.linked_brush_x = None;
                self.publish_reset_link();
                self.publish_brush_link(None);
            }
            state.clear_interaction();
            drop(state);
            cx.emit(PlotEvent::ViewportChanged(None));
            self.defer_viewport(None, window, cx);
            cx.notify();
            return;
        }

        state.pending_click = Some(ClickState {
            region,
            button: ev.button,
        });

        match (ev.button, region) {
            (MouseButton::Left, HitRegion::XAxis) => {
                state.drag = Some(DragState::new(DragMode::ZoomX, pos, true));
            }
            (MouseButton::Left, HitRegion::YAxis) => {
                state.drag = Some(DragState::new(DragMode::ZoomY, pos, true));
            }
            (MouseButton::Left, HitRegion::Plot) => {
                state.drag = Some(DragState::new(DragMode::Pan, pos, false));
            }
            (MouseButton::Right, HitRegion::Plot) => {
                state.drag = Some(DragState::new(DragMode::ZoomRect, pos, true));
                state.selection_rect = Some(ScreenRect::new(pos, pos));
            }
            _ => {}
        }

        cx.notify();
    }

    pub(super) fn on_mouse_move(
        &mut self,
        ev: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = screen_point(ev.position);
        let cursor_x;
        let mut changed_view = None;
        {
            let mut state = self.state.write().expect("plot state lock");
            state.last_cursor = Some(pos);

            if state.legend_hit(pos).is_some() {
                state.hover = None;
            } else if state.regions.hit_test(pos) == HitRegion::Plot {
                state.hover = Some(pos);
            } else {
                state.hover = None;
            }
            let linked_cursor_x = state.hover.and_then(|_| {
                state
                    .transform
                    .as_ref()
                    .and_then(|transform| transform.screen_to_data(pos))
                    .map(|point| point.x)
            });
            self.publish_cursor_link(linked_cursor_x);
            cursor_x = linked_cursor_x;

            let Some(mut drag) = state.drag.clone() else {
                drop(state);
                cx.emit(PlotEvent::CursorX(cursor_x));
                self.defer_cursor(cursor_x, window, cx);
                cx.notify();
                return;
            };

            if !is_drag_button_held(drag.mode, ev.pressed_button) {
                state.clear_interaction();
                self.publish_cursor_link(None);
                drop(state);
                cx.emit(PlotEvent::CursorX(None));
                self.defer_cursor(None, window, cx);
                cx.notify();
                return;
            }

            let moved_sq = distance_sq(drag.start, pos);
            if !drag.active && moved_sq > self.config.drag_threshold_px.powi(2) {
                drag.active = true;
            }

            if !drag.active {
                state.drag = Some(drag);
                drop(state);
                cx.emit(PlotEvent::CursorX(cursor_x));
                self.defer_cursor(cursor_x, window, cx);
                cx.notify();
                return;
            }

            let delta = ScreenPoint::new(pos.x - drag.last.x, pos.y - drag.last.y);
            let plot_rect = state.plot_rect;
            let transform = state.transform.clone();

            match drag.mode {
                DragMode::Pan => {
                    if let (Some(rect), Some(transform)) = (plot_rect, transform)
                        && let Ok(mut plot) = self.plot.write()
                        && let Some(viewport) = plot.viewport()
                        && let Some(next) = pan_viewport(viewport, delta, &transform)
                    {
                        self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
                        changed_view = Some(next);
                    }
                }
                DragMode::ZoomRect => {
                    state.selection_rect = Some(ScreenRect::new(drag.start, pos));
                }
                DragMode::ZoomX => {
                    if let (Some(rect), Some(transform)) = (plot_rect, transform) {
                        let axis_pixels = rect.width().max(1.0);
                        let factor = zoom_factor_from_drag(delta.x, axis_pixels);
                        if let Ok(mut plot) = self.plot.write()
                            && let Some(viewport) = plot.viewport()
                        {
                            let center = transform
                                .screen_to_data(pos)
                                .unwrap_or_else(|| viewport.x_center());
                            let next = zoom_viewport_with_scales(
                                viewport,
                                center,
                                factor,
                                1.0,
                                transform.x_scale(),
                                transform.y_scale(),
                            );
                            self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
                            changed_view = Some(next);
                        }
                    }
                }
                DragMode::ZoomY => {
                    if let (Some(rect), Some(transform)) = (plot_rect, transform) {
                        let axis_pixels = rect.height().max(1.0);
                        let factor = zoom_factor_from_drag(-delta.y, axis_pixels);
                        if let Ok(mut plot) = self.plot.write()
                            && let Some(viewport) = plot.viewport()
                        {
                            let center = transform
                                .screen_to_data(pos)
                                .unwrap_or_else(|| viewport.y_center());
                            let next = zoom_viewport_with_scales(
                                viewport,
                                center,
                                1.0,
                                factor,
                                transform.x_scale(),
                                transform.y_scale(),
                            );
                            self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
                            changed_view = Some(next);
                        }
                    }
                }
            }

            drag.last = pos;
            state.drag = Some(drag);
            state.pending_click = None;
        }

        cx.emit(PlotEvent::CursorX(cursor_x));
        self.defer_cursor(cursor_x, window, cx);
        if let Some(next) = changed_view {
            cx.emit(PlotEvent::ViewportChanged(Some(next)));
            self.defer_viewport(Some(next), window, cx);
        }
        cx.notify();
    }

    pub(super) fn on_hover_state_change(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if hovered {
            return;
        }

        let cursor = screen_point(window.mouse_position());
        let mut state = self.state.write().expect("plot state lock");
        let still_inside = state.legend_hit(cursor).is_some()
            || state.regions.hit_test(cursor) != HitRegion::Outside;
        if still_inside {
            return;
        }

        let changed = state.hover.take().is_some() || state.hover_target.take().is_some();
        state.last_cursor = None;
        drop(state);

        self.publish_cursor_link(None);
        if changed {
            cx.emit(PlotEvent::CursorX(None));
            self.defer_cursor(None, window, cx);
            cx.notify();
        }
    }

    pub(super) fn on_mouse_up(
        &mut self,
        ev: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = screen_point(ev.position);
        let mut selected_x = None;
        let mut changed_view = None;
        {
            let mut state = self.state.write().expect("plot state lock");
            let drag = state.drag.clone();

            if let Some(drag_state) = drag.as_ref()
                && drag_state.active
                && drag_state.mode == DragMode::ZoomRect
                && let (Some(rect), Some(transform)) =
                    (state.selection_rect.take(), state.transform.clone())
            {
                let rect = normalized_rect(rect);
                if let Ok(mut plot) = self.plot.write()
                    && let Some(viewport) = plot.viewport()
                    && let Some(next) = zoom_to_rect(viewport, rect, &transform)
                {
                    self.apply_manual_view_with_link(
                        &mut plot,
                        &mut state,
                        transform.screen(),
                        next,
                    );
                    self.publish_brush_link(Some(next.x));
                    changed_view = Some(next);
                }
            }

            let click = state.pending_click.take();
            let should_toggle = click.as_ref().is_some_and(|click| {
                click.button == MouseButton::Left && click.region == HitRegion::Plot
            }) && drag.as_ref().is_none_or(|drag| !drag.active)
                && ev.click_count == 1;

            if should_toggle {
                if let Some(transform) = state.transform.clone() {
                    if let Some(data_point) = transform.screen_to_data(pos) {
                        state.selected_x = Some(data_point.x);
                        selected_x = Some(data_point.x);
                    }
                    if let Ok(mut plot) = self.plot.write() {
                        let target = state
                            .hover_target
                            .filter(|target| {
                                hover_target_within_threshold(target, pos, &self.config)
                            })
                            .or_else(|| {
                                compute_hover_target(
                                    &plot,
                                    &transform,
                                    pos,
                                    state.plot_rect,
                                    self.config.pin_threshold_px,
                                    self.config.unpin_threshold_px,
                                )
                            });

                        if let Some(target) = target {
                            let added = toggle_pin(plot.pins_mut(), target.pin);
                            let now = Instant::now();
                            state.last_pin_toggle = Some(PinToggle {
                                pin: target.pin,
                                added,
                                at: now,
                                screen_pos: target.screen,
                            });
                        }
                    }
                }
            } else if ev.click_count > 1 {
                state.last_pin_toggle = None;
            }

            state.drag = None;
            state.selection_rect = None;
            self.publish_cursor_link(None);
        }

        if let Some(x) = selected_x {
            cx.emit(PlotEvent::SelectedX(x));
            self.defer_select(x, window, cx);
        }
        if let Some(next) = changed_view {
            cx.emit(PlotEvent::ViewportChanged(Some(next)));
            self.defer_viewport(Some(next), window, cx);
        }
        cx.notify();
    }

    pub(super) fn on_mouse_up_out(
        &mut self,
        ev: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_mouse_up(ev, window, cx);
    }
}

pub(super) fn is_drag_button_held(mode: DragMode, pressed_button: Option<MouseButton>) -> bool {
    let expected = match mode {
        DragMode::ZoomRect => MouseButton::Right,
        DragMode::Pan | DragMode::ZoomX | DragMode::ZoomY => MouseButton::Left,
    };
    pressed_button == Some(expected)
}

pub(super) fn track_outside_rect_drag(view: gpui::WeakEntity<PlotView>, window: &mut Window) {
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase != gpui::DispatchPhase::Bubble || event.pressed_button != Some(MouseButton::Right)
        {
            return;
        }
        let _ = view.update(cx, |view, cx| {
            let outside = {
                let state = view.state.read().expect("plot state lock");
                state
                    .drag
                    .as_ref()
                    .is_some_and(|drag| drag.mode == DragMode::ZoomRect)
                    && state
                        .view_bounds
                        .is_some_and(|bounds| !bounds.contains(&event.position))
            };
            if outside {
                view.on_mouse_move(event, window, cx);
            }
        });
    });
}
