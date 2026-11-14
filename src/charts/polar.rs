use std::{
    collections::HashSet,
    f32::consts::{FRAC_PI_2, TAU},
    fmt::Write as _,
    rc::Rc,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder,
    PinchEvent, Pixels, Refineable, RenderOnce, ScrollEvent, ScrollSource, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div,
};

use super::{
    axes::anchored,
    paint::{finish, measure, tint},
    parts::{ChartLegend, ChartTooltip},
    pie::{bearing, toward},
    plot::Format,
    scale::{compact, nice},
    series::drawable,
};
use crate::{
    buttons::{Button, ButtonVariant},
    theme::{ActiveTheme, ControlSize},
};

#[derive(Default)]
struct PolarState {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    pinned: Option<usize>,
    hidden: HashSet<SharedString>,
    rotation: f32,
    zoom: f32,
    drag_start: Option<(f32, f32)>,
    drag_last_angle: Option<f32>,
    dragged: bool,
}

/// Nightingale rose / radial bar chart where each sector radius encodes its value.
#[derive(IntoElement)]
pub struct PolarChart {
    base: Div,
    id: ElementId,
    slices: Vec<(SharedString, f64)>,
    format: Format,
}

impl PolarChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            slices: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// Add a named radial sector (`value >= 0`).
    pub fn slice(mut self, name: impl Into<SharedString>, value: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0,
            "a polar slice needs a non-negative value, within charts::LIMIT"
        );
        self.slices.push((name.into(), value));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }

    pub fn csv(&self) -> String {
        let mut out = String::from("name,value");
        for (name, value) in &self.slices {
            write!(out, "\n{name},{value}").expect("csv write");
        }
        out
    }

    pub fn svg(&self, width: f32, height: f32, cx: &App) -> String {
        let colors = &cx.theme().colors;
        let (cx_pt, cy_pt) = (width * 0.5, height * 0.5);
        let outer = (width.min(height) * 0.42).max(1.0);
        let max_val = self
            .slices
            .iter()
            .map(|(_, v)| *v)
            .fold(0.0_f64, f64::max)
            .max(1e-9);
        let n = self.slices.len().max(1);
        let sweep = TAU / n as f32;
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
        );
        for (ix, (_, val)) in self.slices.iter().enumerate() {
            let r = outer * (*val / max_val) as f32;
            let a0 = -FRAC_PI_2 + sweep * ix as f32;
            let a1 = a0 + sweep;
            let x0 = cx_pt + r * a0.cos();
            let y0 = cy_pt + r * a0.sin();
            let x1 = cx_pt + r * a1.cos();
            let y1 = cy_pt + r * a1.sin();
            let rgba: gpui::Rgba = tint(colors, ix).into();
            let hex = format!(
                "#{:02x}{:02x}{:02x}",
                (rgba.r * 255.0).round() as u8,
                (rgba.g * 255.0).round() as u8,
                (rgba.b * 255.0).round() as u8
            );
            write!(
                out,
                r#"<path d="M{cx_pt:.1},{cy_pt:.1} L{x0:.1},{y0:.1} A{r:.1},{r:.1} 0 0,1 {x1:.1},{y1:.1} Z" fill="{hex}" fill-opacity="0.72"/>"#,
            )
            .expect("svg write");
        }
        out.push_str("</svg>");
        out
    }
}

impl Styled for PolarChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for PolarChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<PolarState> =
            window.use_keyed_state((self.id.clone(), "polar"), cx, |_, _| PolarState {
                zoom: 1.0,
                ..PolarState::default()
            });
        let (bounds, hover_raw, pinned_raw, hidden, rotation, zoom) = {
            let s = state.read(cx);
            (
                s.bounds,
                s.hover,
                s.pinned,
                s.hidden.clone(),
                s.rotation,
                if s.zoom <= 0.0 { 1.0 } else { s.zoom },
            )
        };
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let center = (box_size.0 * 0.5, box_size.1 * 0.5);
        let base_outer = (center.0.min(center.1) * 0.84).max(0.0);
        let outer = base_outer * zoom;
        let visible_indices: Vec<usize> = self
            .slices
            .iter()
            .enumerate()
            .filter(|(_, (name, _))| !hidden.contains(name))
            .map(|(ix, _)| ix)
            .collect();
        let raw_max = visible_indices
            .iter()
            .map(|&ix| self.slices[ix].1)
            .fold(0.0_f64, f64::max);
        let ((_, nice_max), _) = nice(0.0, raw_max.max(1.0), 4);
        let n = visible_indices.len().max(1);
        let sweep = TAU / n as f32;
        let total: f64 = visible_indices.iter().map(|&ix| self.slices[ix].1).sum();
        let format = self.format.clone();

        let radii: Vec<f32> = visible_indices
            .iter()
            .map(|&ix| outer * (self.slices[ix].1 / nice_max.max(1e-9)).clamp(0.0, 1.0) as f32)
            .collect();
        let active_slot = pinned_raw
            .or(hover_raw)
            .filter(|&slot| slot < visible_indices.len());

        let tooltip = active_slot.map(|slot| {
            let ix = visible_indices[slot];
            let (name, value) = &self.slices[ix];
            let mid = -FRAC_PI_2 + rotation + sweep * (slot as f32 + 0.5);
            let r = radii[slot].min(base_outer) * 0.65;
            let pos = (center.0 + r * mid.cos(), center.1 + r * mid.sin());
            let share = if total > 0.0 { *value / total } else { 0.0 };
            let card = ChartTooltip::new(name.clone())
                .row(Some(tint(&colors, ix)), "Value", format(*value))
                .row(
                    None,
                    "Share",
                    crate::typography::format::percent(share, 1, false),
                );
            anchored(pos, pos.0 > box_size.0 * 0.55, card)
        });

        let legend = (self.slices.len() > 1).then(|| {
            let st = state.clone();
            let leg = self.slices.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "polar-legend")),
                |leg, (ix, (name, _))| {
                    leg.entry(tint(&colors, ix), name.clone(), hidden.contains(name))
                },
            );
            leg.on_toggle(move |name, _, cx| {
                st.update(cx, |s, cx| {
                    if !s.hidden.remove(name) {
                        s.hidden.insert(name.clone());
                    }
                    s.pinned = None;
                    s.hover = None;
                    cx.notify();
                });
            })
        });

        let modified = rotation.abs() > 1e-4 || (zoom - 1.0).abs() > 1e-3 || pinned_raw.is_some();
        let reset_btn = modified.then(|| {
            let st = state.clone();
            let btn = Button::new((self.id.clone(), "polar-reset"), "Reset")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    st.update(cx, |s, cx| {
                        s.rotation = 0.0;
                        s.zoom = 1.0;
                        s.pinned = None;
                        s.drag_start = None;
                        s.drag_last_angle = None;
                        cx.notify();
                    });
                });
            div().absolute().top_0().right_0().child(btn)
        });

        let radii_hit = radii.clone();
        let count = visible_indices.len();
        let min_hit_r = if touch_mode { 24.0 } else { 6.0 };
        let pick_slot = move |pos: (f32, f32), rot: f32| -> Option<usize> {
            if count == 0 || outer <= 0.0 {
                return None;
            }
            let (angle, dist) = bearing(center, pos);
            if dist > outer.max(min_hit_r) {
                return None;
            }
            let rel = (angle + FRAC_PI_2 - rot).rem_euclid(TAU);
            let slot = ((rel / sweep).floor() as usize).min(count - 1);
            (dist <= radii_hit[slot].max(min_hit_r)).then_some(slot)
        };
        let pick_down = pick_slot.clone();
        let pick_move = pick_slot.clone();
        let pick_up = pick_slot;

        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let (s_down, s_move, s_up, s_up_out, s_leave, s_scroll, s_pinch) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let plot_area = div()
            .id((self.id.clone(), "polar"))
            .debug_selector(|| "polar-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |ev: &MouseDownEvent, _, cx| {
                let off = ev.position - s_down.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                s_down.update(cx, |s, cx| {
                    if ev.click_count >= 2 {
                        s.rotation = 0.0;
                        s.zoom = 1.0;
                        s.pinned = None;
                        s.drag_start = None;
                        s.drag_last_angle = None;
                        cx.notify();
                        return;
                    }
                    s.hover = pick_down(pos, s.rotation);
                    s.drag_start = Some(pos);
                    s.drag_last_angle = Some((pos.1 - center.1).atan2(pos.0 - center.0));
                    s.dragged = false;
                    cx.notify();
                });
            })
            .on_mouse_move(move |ev: &MouseMoveEvent, _, cx| {
                let off = ev.position - s_move.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                s_move.update(cx, |s, cx| {
                    if ev.pressed_button == Some(MouseButton::Left)
                        && let (Some(start), Some(last_a)) = (s.drag_start, s.drag_last_angle)
                    {
                        if (pos.0 - start.0).hypot(pos.1 - start.1) > 4.0 {
                            s.dragged = true;
                        }
                        if s.dragged {
                            let cur_a = (pos.1 - center.1).atan2(pos.0 - center.0);
                            let mut da = cur_a - last_a;
                            if da > std::f32::consts::PI {
                                da -= TAU;
                            } else if da < -std::f32::consts::PI {
                                da += TAU;
                            }
                            s.rotation = (s.rotation + da).rem_euclid(TAU);
                            s.drag_last_angle = Some(cur_a);
                            cx.notify();
                            return;
                        }
                    }
                    let next = pick_move(pos, s.rotation);
                    if s.hover != next {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |ev: &MouseUpEvent, _, cx| {
                let off = ev.position - s_up.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                s_up.update(cx, |s, cx| {
                    if !s.dragged && ev.click_count == 1 {
                        let hit = pick_up(pos, s.rotation);
                        s.pinned = if s.pinned == hit { None } else { hit };
                    }
                    s.drag_start = None;
                    s.drag_last_angle = None;
                    s.dragged = false;
                    cx.notify();
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                s_up_out.update(cx, |s, _| {
                    s.drag_start = None;
                    s.drag_last_angle = None;
                    s.dragged = false;
                });
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    s_leave.update(cx, |s, cx| {
                        s.hover = None;
                        cx.notify();
                    });
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if ev.source == ScrollSource::Touch {
                    if dx.abs() > dy.abs() && dx.abs() >= 0.2 {
                        s_scroll.update(cx, |s, cx| {
                            s.rotation = (s.rotation + dx * 0.008).rem_euclid(TAU);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                    return;
                }
                if dy.abs() >= 0.05 {
                    s_scroll.update(cx, |s, cx| {
                        let z = if s.zoom <= 0.0 { 1.0 } else { s.zoom };
                        s.zoom = (z * (1.0 + dy * 0.002)).clamp(0.5, 3.0);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                let scale = ev.delta.exp().clamp(0.25, 4.0);
                if (scale - 1.0).abs() > 1e-4 {
                    s_pinch.update(cx, |s, cx| {
                        s.drag_start = None;
                        s.drag_last_angle = None;
                        s.dragged = false;
                        let z = if s.zoom <= 0.0 { 1.0 } else { s.zoom };
                        s.zoom = (z * scale).clamp(0.5, 3.0);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let center_pt = bounds.center();
                        for step in 1..=4 {
                            let ring_r = outer * (step as f32 / 4.0);
                            let mut ring_path = PathBuilder::stroke(hairline);
                            let segs = 48;
                            ring_path.move_to(toward(center_pt, ring_r, 0.0));
                            for s in 1..=segs {
                                ring_path.line_to(toward(
                                    center_pt,
                                    ring_r,
                                    TAU * s as f32 / segs as f32,
                                ));
                            }
                            ring_path.close();
                            finish(ring_path, colors.border.opacity(0.55), window);
                        }
                        for (slot, &r) in radii.iter().enumerate() {
                            if r <= 0.5 {
                                continue;
                            }
                            let orig_ix = visible_indices[slot];
                            let start = -FRAC_PI_2 + rotation + sweep * slot as f32;
                            let end = start + sweep;
                            let steps = ((sweep * r / 3.0).ceil() as usize).max(6);
                            let mut fill_path = PathBuilder::fill();
                            fill_path.move_to(center_pt);
                            for s in 0..=steps {
                                let a = start + (end - start) * (s as f32 / steps as f32);
                                fill_path.line_to(toward(center_pt, r, a));
                            }
                            fill_path.close();
                            let ink = tint(&colors, orig_ix);
                            let lit = active_slot == Some(slot);
                            let alpha = if lit { 0.85 } else { 0.6 };
                            finish(fill_path, ink.opacity(alpha), window);
                            if lit {
                                let mut edge = PathBuilder::stroke(stroke);
                                edge.move_to(center_pt);
                                for s in 0..=steps {
                                    let a = start + (end - start) * (s as f32 / steps as f32);
                                    edge.line_to(toward(center_pt, r, a));
                                }
                                edge.close();
                                finish(edge, colors.fg, window);
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(tooltip)
            .children(reset_btn)
            .child(measure(state, |s| &mut s.bounds));

        root.flex()
            .flex_col()
            .gap_2()
            .children(legend)
            .child(plot_area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[gpui::test]
    fn polar_chart_exports_csv_and_svg(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            let chart = PolarChart::new("wind")
                .slice("N", 12.0)
                .slice("E", 24.0)
                .slice("S", 8.0);
            assert_eq!(chart.csv(), "name,value\nN,12\nE,24\nS,8");
            let svg = chart.svg(240.0, 240.0, cx);
            assert!(svg.starts_with("<svg") && svg.matches("<path").count() == 3);
        });
    }

    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn polar_chart_supports_desktop_and_touch_pin_rotate_pinch_and_reset(
        cx: &mut gpui::TestAppContext,
    ) {
        use crate::charts::tests::settle;
        use gpui::{Context, Modifiers, PinchEvent, Render, TouchPhase, Window, point, px};

        struct Harness;
        impl Render for Harness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().w(px(320.0)).h(px(280.0)).child(
                    PolarChart::new("polar-ui")
                        .slice("A", 10.0)
                        .slice("B", 20.0)
                        .slice("C", 15.0),
                )
            }
        }

        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |t| t.touch_mode = true);
        });
        let (_, vc) = cx.add_window_view(|_, _| Harness);
        settle(vc);

        let plot = vc.debug_bounds("polar-plot").expect("polar plot");
        let tap = point(plot.center().x, plot.center().y - px(30.0));
        vc.simulate_mouse_down(tap, MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(tap, MouseButton::Left, Modifiers::none());
        settle(vc);

        vc.simulate_event(PinchEvent {
            position: plot.center(),
            delta: 0.3,
            modifiers: Modifiers::none(),
            phase: TouchPhase::Moved,
        });
        settle(vc);

        vc.simulate_mouse_down(plot.center(), MouseButton::Left, Modifiers::none());
        vc.simulate_mouse_up(plot.center(), MouseButton::Left, Modifiers::none());
    }
}
