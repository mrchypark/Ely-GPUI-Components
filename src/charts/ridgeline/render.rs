use super::*;

impl Styled for RidgelineChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for RidgelineChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<RidgeState> =
            window.use_keyed_state((self.id.clone(), "ridgeline"), cx, |_, _| {
                RidgeState::default()
            });
        let (bounds, hover_raw, pinned_raw, hidden, x_domain) = {
            let s = state.read(cx);
            (s.bounds, s.hover, s.pinned, s.hidden.clone(), s.x_domain)
        };
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let visible_pairs: Vec<(usize, (SharedString, Vec<f64>))> = self
            .groups
            .iter()
            .enumerate()
            .filter(|(_, (name, _))| !hidden.contains(name))
            .map(|(ix, g)| (ix, g.clone()))
            .collect();
        let visible_groups: Vec<(SharedString, Vec<f64>)> =
            visible_pairs.iter().map(|(_, g)| g.clone()).collect();
        let visible_colors: Vec<usize> = visible_pairs.iter().map(|(ix, _)| *ix).collect();
        let text_size = theme.text_size(TextSize::Xs).to_pixels(rem);
        let widest = visible_groups
            .iter()
            .map(|(label, _)| f32::from(text_width(label, text_size, window)))
            .fold(0.0, f32::max);
        let gutter = (widest + pixels(PAD) + 4.0)
            .max(pixels(sizes.gutter))
            .min(box_size.0 / 2.0);
        let rect = frame(box_size, gutter, pixels(sizes.foot), pixels(sizes.inset));
        let layout = compute_ridges(&visible_groups, rect, self.samples, self.overlap, x_domain);
        let active_slot = pinned_raw
            .or(hover_raw)
            .filter(|&ix| ix < visible_groups.len());
        let format = self.format.clone();
        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every_x = ((layout.x_ticks.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        let x_labels: Vec<_> = layout
            .x_ticks
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every_x == 0)
            .map(|(_, (at_x, val))| below(*at_x, rect, tabular(text(format(*val).into()))))
            .collect();
        let y_labels: Vec<_> = layout
            .ridges
            .iter()
            .zip(&visible_groups)
            .map(|(ridge, (label, _))| {
                let name = div()
                    .min_w_0()
                    .text_right()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(Ellipsis::new(label.clone()));
                beside(ridge.baseline, Pixels::from(gutter), name)
            })
            .collect();

        let tooltip = active_slot.map(|ix| {
            let ridge = &layout.ridges[ix];
            let ink = tint(&colors, visible_colors[ix]);
            let card = ChartTooltip::new(visible_groups[ix].0.clone())
                .row(Some(ink), "Median", format(ridge.median))
                .row(
                    None,
                    "IQR",
                    format!("{} – {}", format(ridge.q1), format(ridge.q3)),
                );
            let anchor_x = rect.x + rect.w * 0.5;
            anchored(
                (anchor_x, ridge.baseline),
                anchor_x > box_size.0 * 0.55,
                card,
            )
        });

        let legend = (self.groups.len() > 1).then(|| {
            let st = state.clone();
            let leg = self.groups.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "ridge-legend")),
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

        let show_reset = x_domain.is_some() || pinned_raw.is_some();
        let reset_btn = show_reset.then(|| {
            let st = state.clone();
            let btn = Button::new((self.id.clone(), "show-all"), "Show all")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    st.update(cx, |s, cx| {
                        s.x_domain = None;
                        s.pinned = None;
                        cx.notify();
                    });
                });
            div().absolute().top_0().right_0().child(btn)
        });

        let count = visible_groups.len();
        let row_h = rect.h / count.max(1) as f32;
        let base_dom = layout.base_domain;
        let act_dom = layout.active_domain;
        let ridges = layout.ridges;
        let (stroke, hairline, bg) = (sizes.stroke.to_pixels(rem), sizes.hairline, colors.bg);
        let baseline_color = colors.border.opacity(0.6);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let pick_row = move |pos: (f32, f32)| -> Option<usize> {
            (rect.contains(pos) && count > 0 && row_h > 0.0)
                .then(|| (((pos.1 - rect.y) / row_h) as usize).min(count - 1))
        };
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
            .id((self.id.clone(), "ridgeline"))
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |ev: &MouseDownEvent, _, cx| {
                let off = ev.position - s_down.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                s_down.update(cx, |s, cx| {
                    if ev.click_count >= 2 {
                        s.x_domain = None;
                        s.pinned = None;
                        s.drag_start = None;
                        s.drag_last = None;
                        cx.notify();
                        return;
                    }
                    s.hover = pick_row(pos);
                    if rect.contains(pos) {
                        s.drag_start = Some(pos);
                        s.drag_last = Some(pos);
                        s.dragged = false;
                    }
                    cx.notify();
                });
            })
            .on_mouse_move(move |ev: &MouseMoveEvent, _, cx| {
                let off = ev.position - s_move.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                s_move.update(cx, |s, cx| {
                    if ev.pressed_button == Some(MouseButton::Left)
                        && let (Some(start), Some(last)) = (s.drag_start, s.drag_last)
                    {
                        if (pos.0 - start.0).hypot(pos.1 - start.1) > 4.0 {
                            s.dragged = true;
                        }
                        if s.dragged && rect.w > 1.0 {
                            let cur = s.x_domain.unwrap_or(base_dom);
                            let shift = (cur.1 - cur.0) * f64::from(-(pos.0 - last.0) / rect.w);
                            s.x_domain = Some((cur.0 + shift, cur.1 + shift));
                            s.drag_last = Some(pos);
                            cx.notify();
                            return;
                        }
                    }
                    let next = pick_row(pos);
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
                        let hit = pick_row(pos);
                        s.pinned = if s.pinned == hit { None } else { hit };
                    }
                    s.drag_start = None;
                    s.drag_last = None;
                    s.dragged = false;
                    cx.notify();
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                s_up_out.update(cx, |s, _| {
                    s.drag_start = None;
                    s.drag_last = None;
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
                let off = ev.position - s_scroll.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                if !rect.contains(pos) || rect.w <= 1.0 {
                    return;
                }
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                let base_span = (base_dom.1 - base_dom.0).abs().max(1e-6);
                if ev.source == ScrollSource::Touch {
                    if dx.abs() > dy.abs() && dx.abs() >= 0.2 {
                        s_scroll.update(cx, |s, cx| {
                            let cur = s.x_domain.unwrap_or(act_dom);
                            let shift = (cur.1 - cur.0) * f64::from(-dx / rect.w);
                            s.x_domain = Some((cur.0 + shift, cur.1 + shift));
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                    return;
                }
                if dy.abs() >= 0.05 {
                    s_scroll.update(cx, |s, cx| {
                        let cur = s.x_domain.unwrap_or(act_dom);
                        let factor = (1.0 - f64::from(dy) * 0.002).clamp(0.2, 5.0);
                        let t = f64::from(((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0));
                        let anchor = cur.0 + (cur.1 - cur.0) * t;
                        let n0 = anchor + (cur.0 - anchor) * factor;
                        let n1 = anchor + (cur.1 - anchor) * factor;
                        let span = (n1 - n0).abs();
                        if span >= base_span * 1e-4 && span <= base_span * 10.0 {
                            s.x_domain = Some((n0, n1));
                            cx.notify();
                        }
                    });
                    cx.stop_propagation();
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                let off = ev.position - s_pinch.read(cx).bounds.origin;
                let pos = (f32::from(off.x), f32::from(off.y));
                if !rect.contains(pos) || rect.w <= 1.0 {
                    return;
                }
                let scale = ev.delta.exp().clamp(0.25, 4.0);
                if (scale - 1.0).abs() > 1e-4 {
                    s_pinch.update(cx, |s, cx| {
                        s.drag_start = None;
                        s.drag_last = None;
                        s.dragged = false;
                        let cur = s.x_domain.unwrap_or(act_dom);
                        let base_span = (base_dom.1 - base_dom.0).abs().max(1e-6);
                        let factor = 1.0 / f64::from(scale);
                        let t = f64::from(((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0));
                        let anchor = cur.0 + (cur.1 - cur.0) * t;
                        let n0 = anchor + (cur.0 - anchor) * factor;
                        let n1 = anchor + (cur.1 - anchor) * factor;
                        let span = (n1 - n0).abs();
                        if span >= base_span * 1e-4 && span <= base_span * 10.0 {
                            s.x_domain = Some((n0, n1));
                        }
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for (ix, ridge) in ridges.iter().enumerate() {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, ridge.baseline)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                baseline_color,
                            ));
                            if ridge.points.len() < 2 {
                                continue;
                            }
                            let ink = tint(&colors, visible_colors[ix]);
                            let mut fill_path = PathBuilder::fill();
                            fill_path.move_to(at(origin, (ridge.points[0].0, ridge.baseline)));
                            for &pt in &ridge.points {
                                fill_path.line_to(at(origin, pt));
                            }
                            if let Some(last) = ridge.points.last() {
                                fill_path.line_to(at(origin, (last.0, ridge.baseline)));
                            }
                            fill_path.close();
                            finish(fill_path, bg.opacity(0.88), window);

                            let mut tint_path = PathBuilder::fill();
                            tint_path.move_to(at(origin, (ridge.points[0].0, ridge.baseline)));
                            for &pt in &ridge.points {
                                tint_path.line_to(at(origin, pt));
                            }
                            if let Some(last) = ridge.points.last() {
                                tint_path.line_to(at(origin, (last.0, ridge.baseline)));
                            }
                            tint_path.close();
                            let fill_alpha = if active_slot == Some(ix) { 0.48 } else { 0.28 };
                            finish(tint_path, ink.opacity(fill_alpha), window);

                            let mut line_path = PathBuilder::stroke(stroke);
                            trace(&mut line_path, &ridge.points, true, origin);
                            finish(line_path, ink, window);
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(x_labels)
            .children(y_labels)
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
