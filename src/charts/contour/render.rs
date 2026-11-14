use super::*;

impl Styled for ContourChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ContourChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<ContourState> =
            window.use_keyed_state((self.id.clone(), "contour"), cx, |_, _| {
                ContourState::default()
            });
        let (bounds, pointer_at, domain) = {
            let s = state.read(cx);
            (s.bounds, s.hover, s.domain)
        };
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let base_domain = self.base_domain();
        let active_domain = domain.unwrap_or(base_domain);
        let field = Rc::new(self.build_field(domain));
        let has_data = field.cols >= 2 && field.rows >= 2;
        let x_scale = Linear::new(active_domain.0, (rect.x, rect.x + rect.w));
        let y_scale = Linear::new(active_domain.1, (rect.y + rect.h, rect.y));
        let bands = build_bands(
            &field,
            x_scale,
            y_scale,
            self.levels,
            self.thresholds.as_deref(),
        );
        let (x_tick_vals, y_tick_vals) = if domain.is_some() {
            (
                domain_ticks(active_domain.0.0, active_domain.0.1),
                domain_ticks(active_domain.1.0, active_domain.1.1),
            )
        } else {
            (
                nice(active_domain.0.0, active_domain.0.1, 5).1,
                nice(active_domain.1.0, active_domain.1.1, 5).1,
            )
        };
        let hover_dims = (field.cols.max(24), field.rows.max(24));
        let format = self.format.clone();
        let ink = tint(&colors, 0);
        let (x_name, y_name) = self.axes.clone();

        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every_x = ((x_tick_vals.len() as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil()
            as usize)
            .max(1);
        let x_labels: Vec<_> = x_tick_vals
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every_x == 0)
            .map(|(_, val)| below(x_scale.at(*val), rect, tabular(text(format(*val).into()))))
            .collect();
        let y_labels: Vec<_> = y_tick_vals
            .iter()
            .map(|val| {
                beside(
                    y_scale.at(*val),
                    sizes.gutter.to_pixels(rem),
                    tabular(text(format(*val).into())),
                )
            })
            .collect();

        let tooltip = pointer_at.and_then(|key_id| {
            if field.cols < 2 || field.rows < 2 {
                return None;
            }
            let c = key_id % hover_dims.0;
            let r = key_id / hover_dims.0;
            if r >= hover_dims.1 {
                return None;
            }
            let tx = c as f64 / (hover_dims.0 - 1) as f64;
            let ty = r as f64 / (hover_dims.1 - 1) as f64;
            let x = active_domain.0.0 + (active_domain.0.1 - active_domain.0.0) * tx;
            let y = active_domain.1.0 + (active_domain.1.1 - active_domain.1.0) * ty;
            let val = sample_field_at(&field, x, y)?;
            let px = x_scale.at(x);
            let py = y_scale.at(y);
            let card = ChartTooltip::new("Contour")
                .row(Some(ink), "Level", format(val))
                .row(None, x_name.clone(), format(x))
                .row(None, y_name.clone(), format(y));
            Some(anchored((px, py), px > box_size.0 * 0.55, card))
        });

        let y_ticks_px: Vec<f32> = y_tick_vals.iter().map(|v| y_scale.at(*v)).collect();
        let (hairline, grid_color) = (sizes.hairline, colors.border.opacity(0.6));
        let legend = key(steps(ink), ("Low", "High"), cx);
        let field_hit = field.clone();
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let find_hover = Rc::new(move |pos: (f32, f32)| {
            if !rect.contains(pos) || field_hit.cols < 2 || field_hit.rows < 2 {
                return None;
            }
            let x = x_scale.value(pos.0);
            let y = y_scale.value(pos.1);
            let _ = sample_field_at(&field_hit, x, y)?;
            let u = ((pos.0 - rect.x) / rect.w.max(1.0)).clamp(0.0, 1.0);
            let v = ((rect.y + rect.h - pos.1) / rect.h.max(1.0)).clamp(0.0, 1.0);
            let c = (u * (hover_dims.0 - 1) as f32).round() as usize;
            let r = (v * (hover_dims.1 - 1) as f32).round() as usize;
            Some(r.min(hover_dims.1 - 1) * hover_dims.0 + c.min(hover_dims.0 - 1))
        });
        let show_all = domain.map(|_| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "show-all"), "Show all")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .debug_selector(|| "show-all".into())
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |s, cx| {
                        s.domain = None;
                        s.drag_last = None;
                        s.dragged = false;
                        s.hover = None;
                        cx.notify();
                    })
                }))
        });
        let (pressed, moved, released, released_out, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (down_hit, move_hit) = (find_hover.clone(), find_hover);
        let end_drag = move |target: &Entity<ContourState>, cx: &mut App| {
            target.update(cx, |s, cx| {
                s.drag_last = None;
                if s.dragged {
                    s.dragged = false;
                    cx.notify();
                }
            });
        };

        let chart = div()
            .id((self.id.clone(), "contour"))
            .debug_selector(|| "contour-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                if event.click_count == 2 {
                    pressed.update(cx, |s, cx| {
                        s.domain = None;
                        s.drag_last = None;
                        s.dragged = false;
                        s.hover = None;
                        cx.notify();
                    });
                    return;
                }
                let pos = local_pos(event.position, pressed.read(cx).bounds);
                let next = down_hit(pos);
                pressed.update(cx, |s, cx| {
                    s.drag_last = Some(pos);
                    s.dragged = false;
                    s.hover = next;
                    cx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, moved.read(cx).bounds);
                moved.update(cx, |s, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(last) = s.drag_last
                    {
                        let dx = pos.0 - last.0;
                        let dy = pos.1 - last.1;
                        if !s.dragged && dx * dx + dy * dy <= 9.0 {
                            return;
                        }
                        if !has_data || rect.w <= 0.0 || rect.h <= 0.0 {
                            return;
                        }
                        s.dragged = true;
                        s.drag_last = Some(pos);
                        s.hover = None;
                        let cur = s.domain.unwrap_or(base_domain);
                        s.domain = Some(pan_domain(cur, rect, dx, dy));
                        cx.notify();
                        return;
                    }
                    let was_dragging = s.drag_last.is_some() || s.dragged;
                    s.drag_last = None;
                    s.dragged = false;
                    let next = move_hit(pos);
                    if s.hover != next || was_dragging {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| end_drag(&released, cx))
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                end_drag(&released_out, cx)
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |s, cx| {
                        s.hover = None;
                        cx.notify();
                    });
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                if !has_data || rect.w <= 0.0 || rect.h <= 0.0 {
                    return;
                }
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                let pos = local_pos(ev.position, scrolled.read(cx).bounds);
                if ev.source == ScrollSource::Touch {
                    if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                        scrolled.update(cx, |s, cx| {
                            let cur = s.domain.unwrap_or(base_domain);
                            s.domain = Some(pan_domain(cur, rect, dx, dy));
                            s.hover = None;
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                } else {
                    let factor = (1.0 - f64::from(dy) * 0.002).clamp(0.1, 10.0);
                    if (factor - 1.0).abs() > 1e-6 {
                        scrolled.update(cx, |s, cx| {
                            let cur = s.domain.unwrap_or(base_domain);
                            let anchor = cursor_data(cur, rect, pos);
                            s.domain = Some(zoom_domain(cur, base_domain, anchor, factor));
                            s.hover = None;
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                if !ev.delta.is_finite() || ev.phase == TouchPhase::Cancelled {
                    pinched.update(cx, |s, _| {
                        s.drag_last = None;
                        s.dragged = false;
                    });
                    return;
                }
                let scale = ev.delta.exp().clamp(0.1, 10.0);
                let pos = local_pos(ev.position, pinched.read(cx).bounds);
                let factor = 1.0 / f64::from(scale);
                pinched.update(cx, |s, cx| {
                    s.drag_last = None;
                    s.dragged = false;
                    if !has_data || rect.w <= 0.0 || rect.h <= 0.0 || (scale - 1.0).abs() <= 1e-5 {
                        return;
                    }
                    let cur = s.domain.unwrap_or(base_domain);
                    let anchor = cursor_data(cur, rect, pos);
                    s.domain = Some(zoom_domain(cur, base_domain, anchor, factor));
                    s.hover = None;
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let mask = ContentMask {
                            bounds: place(origin, rect),
                        };
                        for &y_px in &y_ticks_px {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, y_px)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                grid_color,
                            ));
                        }
                        window.with_content_mask(Some(mask), |window| {
                            let count = bands.len().max(1);
                            for (lvl_ix, band) in bands.iter().enumerate() {
                                let _ = band.threshold;
                                let alpha = 0.12 + 0.52 * ((lvl_ix + 1) as f32 / count as f32);
                                let fill_color = ink.opacity(alpha);
                                for poly in &band.polygons {
                                    if poly.len() < 3 {
                                        continue;
                                    }
                                    let mut path = PathBuilder::fill();
                                    path.move_to(at(origin, poly[0]));
                                    for &pt in &poly[1..] {
                                        path.line_to(at(origin, pt));
                                    }
                                    path.close();
                                    finish(path, fill_color, window);
                                }
                                let stroke_color = ink.opacity(0.65);
                                for &(a, b) in &band.isolines {
                                    let mut line = PathBuilder::stroke(hairline);
                                    line.move_to(at(origin, a));
                                    line.line_to(at(origin, b));
                                    finish(line, stroke_color, window);
                                }
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(x_labels)
            .children(y_labels)
            .children(tooltip)
            .children(show_all)
            .child(measure(state, |state| &mut state.bounds));

        root.flex().flex_col().gap_2().child(chart).child(legend)
    }
}
