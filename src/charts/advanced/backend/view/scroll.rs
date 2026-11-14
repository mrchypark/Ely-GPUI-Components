use gpui::{Context, Pixels, Point, ScrollEvent, ScrollSource, Window};

use super::super::state::PlotUiState;
use crate::charts::advanced::geom::{Point as DataPoint, ScreenPoint, ScreenRect};
use crate::charts::advanced::interaction::{
    HitRegion, pan_viewport, zoom_factor_from_drag, zoom_viewport_with_scales,
};
use crate::charts::advanced::plot::Plot;
use crate::charts::advanced::transform::Transform;
use crate::charts::advanced::view::Viewport;

use super::super::state::PinToggle;
use super::{PlotEvent, PlotView};

impl PlotView {
    pub(super) fn on_scroll(
        &mut self,
        ev: &ScrollEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pos = screen_point(ev.position);
        let mut changed_view = None;
        let mut handled = false;
        {
            let mut state = self.state.write().expect("plot state lock");
            if state.legend_hit(pos).is_some() {
                return;
            }
            let region = state.regions.hit_test(pos);
            let Some(transform) = state.transform.clone() else {
                return;
            };

            let line_height = Pixels::from(16.0);
            let delta = ev.delta.pixel_delta(line_height);
            if ev.source == ScrollSource::Touch {
                state.hover = None;
                let dx = f32::from(delta.x);
                let dy = f32::from(delta.y);
                if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                    if let Ok(mut plot) = self.plot.write()
                        && let Some(viewport) = plot.viewport()
                        && let Some(rect) = state.plot_rect
                    {
                        match region {
                            HitRegion::Plot => {
                                if let Some(next) =
                                    pan_viewport(viewport, ScreenPoint::new(dx, dy), &transform)
                                {
                                    self.apply_manual_view_with_link(
                                        &mut plot, &mut state, rect, next,
                                    );
                                    changed_view = Some(next);
                                }
                                handled = true;
                            }
                            HitRegion::XAxis => {
                                let factor = zoom_factor_from_drag(dx, rect.width().max(1.0));
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
                                handled = true;
                            }
                            HitRegion::YAxis => {
                                let factor = zoom_factor_from_drag(-dy, rect.height().max(1.0));
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
                                handled = true;
                            }
                            HitRegion::Outside => {}
                        }
                    }
                } else if region != HitRegion::Outside {
                    handled = true;
                }
            } else {
                let factor = scroll_zoom_factor(f32::from(delta.y));
                if factor == 1.0 {
                    return;
                }

                if let Ok(mut plot) = self.plot.write()
                    && let Some(viewport) = plot.viewport()
                {
                    let center = transform
                        .screen_to_data(pos)
                        .unwrap_or_else(|| viewport.center());
                    let (factor_x, factor_y) = match region {
                        HitRegion::XAxis => (factor, 1.0),
                        HitRegion::YAxis => (1.0, factor),
                        HitRegion::Plot => (factor, factor),
                        HitRegion::Outside => (1.0, 1.0),
                    };
                    if (factor_x != 1.0 || factor_y != 1.0)
                        && let Some(rect) = state.plot_rect
                    {
                        let next = zoom_viewport_with_scales(
                            viewport,
                            center,
                            factor_x,
                            factor_y,
                            transform.x_scale(),
                            transform.y_scale(),
                        );
                        self.apply_manual_view_with_link(&mut plot, &mut state, rect, next);
                        changed_view = Some(next);
                        handled = true;
                    }
                }
            }
        }

        if handled {
            cx.stop_propagation();
        }
        if let Some(next) = changed_view {
            cx.emit(PlotEvent::ViewportChanged(Some(next)));
            self.defer_viewport(Some(next), window, cx);
        }
        if handled || changed_view.is_some() {
            cx.notify();
        }
    }
}

pub(super) fn revert_pin_toggle(plot: &mut Plot, toggle: PinToggle) {
    let pins = plot.pins_mut();
    if toggle.added {
        if let Some(index) = pins.iter().position(|pin| *pin == toggle.pin) {
            pins.swap_remove(index);
        }
    } else if !pins.contains(&toggle.pin) {
        pins.push(toggle.pin);
    }
}

pub(super) fn scroll_zoom_factor(delta_y: f32) -> f64 {
    if delta_y.abs() < 0.01 {
        return 1.0;
    }
    (1.0 - (delta_y as f64 * 0.002)).clamp(0.1, 10.0)
}

pub(super) fn pinch_zoom_scale(delta: f32) -> f32 {
    if !delta.is_finite() || delta.abs() < 0.0001 {
        return 1.0;
    }
    delta.exp().clamp(0.1, 10.0)
}

pub(super) fn screen_point(point: Point<Pixels>) -> ScreenPoint {
    ScreenPoint::new(f32::from(point.x), f32::from(point.y))
}

pub(super) fn apply_manual_view(
    plot: &mut Plot,
    state: &mut PlotUiState,
    rect: ScreenRect,
    viewport: Viewport,
) {
    plot.set_manual_view(viewport);
    state.viewport = Some(viewport);
    state.transform =
        Transform::with_scales(viewport, rect, plot.x_axis().scale(), plot.y_axis().scale());
}

pub(super) trait ViewportCenter {
    fn center(&self) -> DataPoint;
    fn x_center(&self) -> DataPoint;
    fn y_center(&self) -> DataPoint;
}

impl ViewportCenter for Viewport {
    fn center(&self) -> DataPoint {
        DataPoint::new(
            (self.x.min + self.x.max) * 0.5,
            (self.y.min + self.y.max) * 0.5,
        )
    }

    fn x_center(&self) -> DataPoint {
        self.center()
    }

    fn y_center(&self) -> DataPoint {
        self.center()
    }
}
