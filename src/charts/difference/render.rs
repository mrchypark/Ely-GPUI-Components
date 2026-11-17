use super::*;

impl Styled for DifferenceChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DifferenceChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<DifferenceState> =
            window.use_keyed_state((self.id.clone(), "difference"), cx, |_, _| {
                DifferenceState::default()
            });
        let (bounds, state_hover, state_pinned, raw_span, brush) = {
            let st = state.read(cx);
            (st.bounds, st.hover, st.pinned, st.span, st.brush)
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
        let (a_vals, b_vals) = self.series_slices();
        let n = self.labels.len().min(a_vals.len()).min(b_vals.len());
        let zoomed = raw_span.filter(|(a, b)| *a < *b && *b < n);
        let (start_ix, end_ix) = zoomed.unwrap_or((0, n.saturating_sub(1)));
        let vis_n = if n == 0 { 0 } else { end_ix - start_ix + 1 };
        let (vis_labels, vis_a, vis_b) = if n == 0 {
            (&[][..], &[][..], &[][..])
        } else {
            (
                &self.labels[start_ix..=end_ix],
                &a_vals[start_ix..=end_ix],
                &b_vals[start_ix..=end_ix],
            )
        };
        let layout = compute_diff(vis_labels, vis_a, vis_b, rect);
        let hover = state_hover.filter(|ix| *ix >= start_ix && *ix < start_ix + vis_n);
        let pinned = state_pinned.filter(|ix| *ix >= start_ix && *ix < start_ix + vis_n);
        let active = pinned.or(hover);
        let format = self.format.clone();
        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(content)
        };
        let every_x =
            ((vis_n as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil() as usize).max(1);
        let x_labels: Vec<_> = layout
            .xs
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every_x == 0)
            .map(|(ix, &x_px)| below(x_px, rect, text(self.labels[start_ix + ix].clone())))
            .collect();
        let y_labels: Vec<_> = layout
            .ticks
            .iter()
            .map(|(y_px, val)| {
                beside(
                    *y_px,
                    sizes.gutter.to_pixels(rem),
                    tabular(text(format(*val).into())),
                )
            })
            .collect();

        let a_name = self
            .primary
            .as_ref()
            .map_or_else(|| "Primary".into(), |s| s.name.clone());
        let b_name = self
            .baseline
            .as_ref()
            .map_or_else(|| "Baseline".into(), |s| s.name.clone());
        let tooltip = active.and_then(|ix| {
            let local_ix = ix - start_ix;
            let px = *layout.xs.get(local_ix)?;
            let (va, vb) = (a_vals[ix], b_vals[ix]);
            let delta = va - vb;
            let delta_color = if delta >= 0.0 {
                colors.success
            } else {
                colors.danger
            };
            let card = ChartTooltip::new(self.labels[ix].clone())
                .row(Some(colors.fg), a_name, format(va))
                .row(Some(colors.fg_subtle), b_name, format(vb))
                .row(Some(delta_color), "Delta", format(delta));
            Some(anchored(
                (px, rect.y + rect.h * 0.4),
                px > box_size.0 * 0.55,
                card,
            ))
        });

        let show_all = (zoomed.is_some() || pinned.is_some()).then(|| {
            let reset = state.clone();
            let label = if zoomed.is_some() {
                "Show all"
            } else {
                "Reset"
            };
            let button = Button::new((self.id.clone(), "show-all"), label)
                .debug_selector(|| "chart-show-all".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.span = None;
                        st.pinned = None;
                        st.hover = None;
                        st.brush = None;
                        cx.notify();
                    })
                }))
        });

        let ticks = layout.ticks;
        let regions = layout.regions;
        let line_a = layout.line_a;
        let line_b = layout.line_b;
        let xs = layout.xs;
        let active_local = active.map(|ix| ix - start_ix);
        let is_pinned = pinned.is_some();
        let pick_x = move |pos: (f32, f32)| -> Option<usize> {
            if !rect.contains(pos) || vis_n == 0 || rect.w <= 0.0 {
                return None;
            }
            if vis_n == 1 {
                return Some(start_ix);
            }
            let t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
            let local_ix = ((t * (vis_n - 1) as f32).round() as usize).min(vis_n - 1);
            Some(start_ix + local_ix)
        };
        let col_at = move |x: f32| -> Option<usize> {
            if vis_n < 2 || rect.w <= 0.0 {
                return None;
            }
            let t = ((x.clamp(rect.x, rect.x + rect.w) - rect.x) / rect.w).clamp(0.0, 1.0);
            Some(start_ix + ((t * (vis_n - 1) as f32).round() as usize).min(vis_n - 1))
        };
        let (moved, pressed, released, left, wheeled, pinched, touch_drag, touch_finish) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let finish_brush = move |inside: bool, cx: &mut App| {
            released.update(cx, |st, cx| {
                let Some((a, b)) = st.brush.take() else {
                    return;
                };
                if (b - a).abs() > 4.0 {
                    if n >= 3
                        && let (Some(from), Some(to)) = (col_at(a.min(b)), col_at(a.max(b)))
                        && to > from
                    {
                        st.span = (from > 0 || to + 1 < n).then_some((from, to));
                    }
                } else if inside && let Some(ix) = pick_x((a, rect.y + rect.h * 0.5)) {
                    st.pinned = if st.pinned == Some(ix) {
                        None
                    } else {
                        Some(ix)
                    };
                }
                cx.notify();
            })
        };
        let (up_in, up_out) = (finish_brush.clone(), finish_brush);
        let (stroke, hairline) = (sizes.stroke.to_pixels(rem), sizes.hairline);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        root.id((self.id.clone(), "difference"))
            .when(n >= 3, |plot| plot.cursor_crosshair())
            .when(n >= 3 && zoomed.is_none(), |plot| {
                plot.child(
                    canvas(
                        |_, _, _| {},
                        move |_, _, window, _| {
                            let touch = touch_drag.clone();
                            let finish = touch_finish.clone();
                            window.on_mouse_event(move |ev: &TouchDragEvent, phase, window, cx| {
                                if phase != DispatchPhase::Bubble {
                                    return;
                                }
                                let bounds = touch.read(cx).bounds;
                                let start = local(ev.start_position, bounds);
                                let at = local(ev.position, bounds);
                                if ev.phase == TouchPhase::Started {
                                    if window.default_prevented()
                                        || !rect.contains(start)
                                        || (at.0 - start.0).abs() <= (at.1 - start.1).abs()
                                    {
                                        return;
                                    }
                                    touch.update(cx, |st, cx| {
                                        st.brush =
                                            Some((start.0, at.0.clamp(rect.x, rect.x + rect.w)));
                                        st.hover = None;
                                        cx.notify();
                                    });
                                    window.prevent_default();
                                    cx.stop_propagation();
                                } else if ev.phase == TouchPhase::Cancelled {
                                    touch.update(cx, |st, cx| {
                                        st.brush = None;
                                        cx.notify();
                                    });
                                } else if touch.read(cx).brush.is_some() {
                                    touch.update(cx, |st, cx| {
                                        if let Some(brush) = st.brush.as_mut() {
                                            brush.1 = at.0.clamp(rect.x, rect.x + rect.w);
                                        }
                                        cx.notify();
                                    });
                                    if ev.phase == TouchPhase::Ended {
                                        finish.update(cx, |st, cx| {
                                            let Some((a, b)) = st.brush.take() else {
                                                return;
                                            };
                                            if (b - a).abs() > 4.0
                                                && let (Some(from), Some(to)) =
                                                    (col_at(a.min(b)), col_at(a.max(b)))
                                                && to > from
                                            {
                                                st.span =
                                                    (from > 0 || to + 1 < n).then_some((from, to));
                                            }
                                            cx.notify();
                                        });
                                    }
                                    cx.stop_propagation();
                                }
                            });
                        },
                    )
                    .absolute()
                    .inset_0()
                    .size_full(),
                )
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(brush) = st.brush.as_mut()
                    {
                        brush.1 = pos.0.clamp(rect.x, rect.x + rect.w);
                        cx.notify();
                        return;
                    }
                    let next = pick_x(pos);
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                })
            })
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |st, cx| {
                    if event.click_count == 2 {
                        st.span = None;
                        st.pinned = None;
                        st.brush = None;
                        cx.notify();
                        return;
                    }
                    st.hover = pick_x(pos);
                    if rect.contains(pos) && vis_n > 0 {
                        st.brush = Some((pos.0, pos.0));
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| up_in(true, cx))
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| up_out(false, cx))
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |st, cx| {
                        st.hover = None;
                        cx.notify();
                    })
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if n < 3 || rect.w <= 0.0 {
                    return;
                }
                let pos = local(ev.position, wheeled.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                if ev.source == ScrollSource::Touch {
                    if zoomed.is_some() && dx.abs() > dy.abs() && dx.abs() >= 0.01 {
                        wheeled.update(cx, |st, cx| {
                            st.span = pan_span(st.span, n, dx, rect.w);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                    return;
                }
                if dy.abs() < 0.01 {
                    return;
                }
                let factor = (-dy * 0.004).exp().clamp(0.25, 4.0);
                wheeled.update(cx, |st, cx| {
                    st.span = zoom_span(st.span, n, factor, anchor_t);
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                if !ev.delta.is_finite() || ev.phase == TouchPhase::Cancelled {
                    pinched.update(cx, |st, _| st.brush = None);
                    return;
                }
                let scale = ev.delta.exp().clamp(0.1, 10.0);
                if (scale - 1.0).abs() < 0.001 || n < 3 || rect.w <= 0.0 {
                    pinched.update(cx, |st, _| st.brush = None);
                    return;
                }
                let pos = local(ev.position, pinched.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                pinched.update(cx, |st, cx| {
                    st.brush = None;
                    st.span = zoom_span(st.span, n, 1.0 / scale, anchor_t);
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let grid = colors.border.opacity(0.6);
                        for &(y_px, _) in &ticks {
                            window.paint_quad(fill(
                                gpui::Bounds::new(
                                    at(origin, (rect.x, y_px)),
                                    size(Pixels::from(rect.w), hairline),
                                ),
                                grid,
                            ));
                        }
                        for (above, poly) in &regions {
                            if poly.len() < 3 {
                                continue;
                            }
                            let mut path = PathBuilder::fill();
                            path.move_to(at(origin, poly[0]));
                            for &pt in &poly[1..] {
                                path.line_to(at(origin, pt));
                            }
                            path.close();
                            let fill_color = if *above {
                                colors.success
                            } else {
                                colors.danger
                            }
                            .opacity(0.24);
                            finish(path, fill_color, window);
                        }
                        for (pts, color) in [(&line_b, colors.fg_subtle), (&line_a, colors.fg)] {
                            if pts.len() < 2 {
                                continue;
                            }
                            let mut path = PathBuilder::stroke(stroke);
                            path.move_to(at(origin, pts[0]));
                            for &pt in &pts[1..] {
                                path.line_to(at(origin, pt));
                            }
                            finish(path, color, window);
                        }
                        if let Some(ix) = active_local {
                            if let Some(&x_px) = xs.get(ix) {
                                window.paint_quad(fill(
                                    gpui::Bounds::new(
                                        at(origin, (x_px, rect.y)),
                                        size(hairline, Pixels::from(rect.h)),
                                    ),
                                    if is_pinned {
                                        colors.fg
                                    } else {
                                        colors.border_strong
                                    },
                                ));
                            }
                            if let Some(&pt) = line_a.get(ix) {
                                ring(
                                    at(origin, pt),
                                    stroke * 2.0,
                                    (colors.bg, colors.fg),
                                    stroke,
                                    window,
                                );
                            }
                        }
                        if let Some((a, b)) = brush
                            && (b - a).abs() > 4.0
                        {
                            let band = Rect {
                                x: a.min(b),
                                y: rect.y,
                                w: (a - b).abs(),
                                h: rect.h,
                            };
                            window.paint_quad(fill(place(origin, band), colors.selection));
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(x_labels)
            .children(y_labels)
            .children(tooltip)
            .children(show_all)
            .child(measure(state, |st| &mut st.bounds))
    }
}
