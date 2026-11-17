use super::*;

impl Styled for HexbinChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for HexbinChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<HexState> =
            window.use_keyed_state((self.id.clone(), "hexbin"), cx, |_, _| HexState::default());
        let (bounds, domain) = {
            let s = state.read(cx);
            (s.bounds, s.domain)
        };
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.gutter),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let layout = compute_layout(&self.points, rect, self.radius, domain);
        let hover = state.read(cx).pointed(layout.bins.len());
        let has_data = !self.points.is_empty();
        let total = self.points.len().max(1);
        let ink = tint(&colors, 0);
        let format = self.format.clone();
        let (x_name, y_name) = self.axes.clone();

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
            .y_ticks
            .iter()
            .map(|(at_y, val)| {
                beside(
                    *at_y,
                    sizes.gutter.to_pixels(rem),
                    tabular(text(format(*val).into())),
                )
            })
            .collect();

        let tooltip = hover.map(|ix| {
            let bin = &layout.bins[ix];
            let share = bin.count as f64 / total as f64;
            let color = ink.opacity(strength(bin.count, layout.max_count));
            let card = ChartTooltip::new(format!("{} points", bin.count))
                .row(Some(color), "Share", format::percent(share, 1, false))
                .row(None, x_name.clone(), format(bin.data_center.0))
                .row(None, y_name.clone(), format(bin.data_center.1));
            let pos = (bin.center.0 + layout.radius, bin.center.1);
            anchored(pos, pos.0 > box_size.0 * 0.55, card)
        });

        let bins_for_hit = layout.bins.clone();
        let bins_for_paint = layout.bins;
        let y_ticks_for_paint = layout.y_ticks;
        let base_domain = layout.base_domain;
        let radius = layout.radius;
        let max_count = layout.max_count;
        let (hairline, stroke) = (sizes.hairline, sizes.stroke.to_pixels(rem));
        let (edge, grid_color) = (colors.fg, colors.border.opacity(0.6));
        let legend = key(steps(ink), ("Few", "Many"), cx);

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let find_hover = Rc::new(move |pos: (f32, f32)| {
            if !rect.contains(pos) {
                return None;
            }
            let hit_radius = if touch_mode { radius.max(24.0) } else { radius };
            let limit = hit_radius * hit_radius * 1.05;
            bins_for_hit
                .iter()
                .enumerate()
                .map(|(ix, bin)| {
                    let dx = pos.0 - bin.center.0;
                    let dy = pos.1 - bin.center.1;
                    (ix, dx * dx + dy * dy)
                })
                .filter(|(_, d2)| *d2 <= limit)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(ix, _)| ix)
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
        let end_drag = move |target: &Entity<HexState>, cx: &mut App| {
            target.update(cx, |s, cx| {
                s.drag_last = None;
                if s.dragged {
                    s.dragged = false;
                    cx.notify();
                }
            });
        };

        let chart = div()
            .id((self.id.clone(), "hexbin"))
            .debug_selector(|| "hexbin-plot".into())
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
                        for (y_px, _) in &y_ticks_for_paint {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, *y_px)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                grid_color,
                            ));
                        }
                        window.with_content_mask(Some(mask), |window| {
                            for (ix, bin) in bins_for_paint.iter().enumerate() {
                                let verts = hex_vertices(bin.center, radius);
                                let mut fill_path = PathBuilder::fill();
                                let mut edge_path = PathBuilder::stroke(if hover == Some(ix) {
                                    stroke
                                } else {
                                    hairline
                                });
                                fill_path.move_to(at(origin, verts[0]));
                                edge_path.move_to(at(origin, verts[0]));
                                for v in &verts[1..] {
                                    fill_path.line_to(at(origin, *v));
                                    edge_path.line_to(at(origin, *v));
                                }
                                fill_path.close();
                                edge_path.close();
                                let fill_color = ink.opacity(strength(bin.count, max_count));
                                let border_color = if hover == Some(ix) {
                                    edge
                                } else {
                                    ink.opacity(0.45)
                                };
                                finish(fill_path, fill_color, window);
                                finish(edge_path, border_color, window);
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
