use super::paint::{HorizonPaint, paint_horizon_canvas};
use super::*;

impl RenderOnce for HorizonChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<HorizonState> =
            window.use_keyed_state((self.id.clone(), "horizon"), cx, |_, _| {
                HorizonState::default()
            });
        let (bounds, state_hover, state_pinned, hidden, raw_span, brush) = {
            let st = state.read(cx);
            (
                st.bounds,
                st.hover,
                st.pinned,
                st.hidden.clone(),
                st.span,
                st.brush,
            )
        };
        let total_cols = self.labels.len();
        let shown: Vec<&Series> = self
            .series
            .iter()
            .filter(|s| !hidden.contains(&s.name))
            .collect();
        let zoomed = raw_span.filter(|(a, b)| *a < *b && *b < total_cols);
        let effective_span = zoomed.unwrap_or((0, total_cols.saturating_sub(1)));
        let (start_col, end_col) = effective_span;
        let n_rows = shown.len();
        let n_cols = if total_cols == 0 {
            0
        } else {
            end_col - start_col + 1
        };
        let hover = state_hover.filter(|ix| n_rows > 0 && n_cols > 0 && *ix < n_rows * n_cols);
        let pinned = state_pinned
            .filter(|&(r, c)| n_rows > 0 && r < n_rows && c >= start_col && c <= end_col);
        let active = pinned.map(|(r, c)| r * n_cols + (c - start_col)).or(hover);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let text_size = theme.text_size(TextSize::Xs).to_pixels(rem);
        let widest = shown
            .iter()
            .map(|series| f32::from(text_width(&series.name, text_size, window)))
            .fold(0.0, f32::max);
        let gutter = (widest + pixels(PAD) + 4.0)
            .max(pixels(sizes.gutter))
            .min(box_size.0 / 2.0);
        let rect = frame(box_size, gutter, pixels(sizes.foot), pixels(sizes.inset));
        let layout = compute_horizon(
            &shown,
            effective_span,
            rect,
            self.bands,
            self.baseline,
            self.mirror,
        );
        let format = self.format.clone();
        let label_color = colors.fg_subtle;
        let text = |content: SharedString| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(label_color)
                .child(content)
        };
        let every =
            ((n_cols as f32 * pixels(sizes.label) / rect.w.max(1.0)).ceil() as usize).max(1);
        let x_labels: Vec<_> = layout
            .xs
            .iter()
            .enumerate()
            .filter(|(ix, _)| ix % every == 0)
            .map(|(ix, &x_px)| below(x_px, rect, text(self.labels[start_col + ix].clone())))
            .collect();
        let y_labels: Vec<_> = layout
            .rows
            .iter()
            .zip(&shown)
            .map(|(row, series)| {
                let name = div()
                    .debug_selector(|| "horizon-row-label".into())
                    .min_w_0()
                    .text_right()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(label_color)
                    .child(Ellipsis::new(series.name.clone()));
                beside(row.rect.y + row.rect.h * 0.5, Pixels::from(gutter), name)
            })
            .collect();

        let tooltip = active.and_then(|hit_ix| {
            if n_cols == 0 || n_rows == 0 {
                return None;
            }
            let row_ix = (hit_ix / n_cols).min(n_rows - 1);
            let local_col = hit_ix % n_cols;
            let col_ix = start_col + local_col;
            let x_px = *layout.xs.get(local_col)?;
            let row = layout.rows.get(row_ix)?;
            let val = shown[row_ix].values[col_ix];
            let delta = val - self.baseline;
            let mag = delta.abs();
            let band_num = if mag <= 0.0 || layout.max_dev <= 0.0 {
                0
            } else {
                ((mag / layout.band_step).ceil() as usize).clamp(1, self.bands)
            };
            let sign = if delta >= 0.0 { "+" } else { "-" };
            let band_color = if delta >= 0.0 {
                colors.chart[0]
            } else {
                colors.danger
            };
            let mut card = ChartTooltip::new(self.labels[col_ix].clone()).row(
                Some(band_color.opacity(band_alpha(band_num.saturating_sub(1), self.bands))),
                format!("{} band", shown[row_ix].name),
                format!("{band_num}/{} ({sign})", self.bands),
            );
            for series in &shown {
                let v = series.values[col_ix];
                let dot = if v >= self.baseline {
                    colors.chart[0]
                } else {
                    colors.danger
                };
                card = card.row(Some(dot), series.name.clone(), format(v));
            }
            Some(anchored(
                (x_px, row.rect.y + row.rect.h * 0.5),
                x_px > box_size.0 * 0.55,
                card,
            ))
        });

        let legend = (self.series.len() > 1).then(|| {
            let toggled = state.clone();
            let legend = self.series.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "legend")),
                |legend, (ix, series)| {
                    legend.entry(
                        tint(&colors, ix),
                        series.name.clone(),
                        hidden.contains(&series.name),
                    )
                },
            );
            legend.on_toggle(move |name, _, cx| {
                toggled.update(cx, |st, cx| {
                    if !st.hidden.remove(name) {
                        st.hidden.insert(name.clone());
                    }
                    cx.notify();
                })
            })
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

        let rows = layout.rows;
        let xs = layout.xs;
        let bands = self.bands;
        let hovered_rc = active.and_then(|hit_ix| {
            (n_cols > 0 && n_rows > 0)
                .then_some(((hit_ix / n_cols).min(n_rows - 1), hit_ix % n_cols))
        });
        let is_pinned = pinned.is_some();
        let pick_rc = move |pos: (f32, f32)| -> Option<usize> {
            if !rect.contains(pos) || n_rows == 0 || n_cols == 0 || rect.w <= 0.0 || rect.h <= 0.0 {
                return None;
            }
            let row = (((pos.1 - rect.y) / rect.h) * n_rows as f32).floor() as usize;
            let row = row.min(n_rows - 1);
            let col = if n_cols == 1 {
                0
            } else {
                let t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                ((t * (n_cols - 1) as f32).round() as usize).min(n_cols - 1)
            };
            Some(row * n_cols + col)
        };
        let col_at = move |x: f32| -> Option<usize> {
            if n_cols < 2 || rect.w <= 0.0 {
                return None;
            }
            let t = ((x.clamp(rect.x, rect.x + rect.w) - rect.x) / rect.w).clamp(0.0, 1.0);
            Some(start_col + ((t * (n_cols - 1) as f32).round() as usize).min(n_cols - 1))
        };
        let (moved, pressed, released, up_bounds, left, wheeled, pinched, touch_drag, touch_finish) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let finish_brush = move |pos: Option<(f32, f32)>, cx: &mut App| {
            released.update(cx, |st, cx| {
                let Some((a, b)) = st.brush.take() else {
                    return;
                };
                if (b - a).abs() > 4.0
                    && total_cols >= 3
                    && let (Some(from), Some(to)) = (col_at(a.min(b)), col_at(a.max(b)))
                    && to > from
                {
                    st.span = (from > 0 || to + 1 < total_cols).then_some((from, to));
                } else if let Some(tap) = pos
                    && let Some(hit_ix) = pick_rc(tap)
                    && n_cols > 0
                    && n_rows > 0
                {
                    let row_ix = (hit_ix / n_cols).min(n_rows - 1);
                    let col_ix = start_col + (hit_ix % n_cols);
                    let rc = (row_ix, col_ix);
                    st.pinned = if st.pinned == Some(rc) {
                        None
                    } else {
                        Some(rc)
                    };
                }
                cx.notify();
            })
        };
        let (up_in, up_out) = (finish_brush.clone(), finish_brush);
        let hairline = sizes.hairline;
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let plot = div()
            .id((self.id.clone(), "horizon"))
            .debug_selector(|| "horizon-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .when(total_cols >= 3, |plot| plot.cursor_crosshair())
            .when(total_cols >= 3 && zoomed.is_none(), |plot| {
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
                                                st.span = (from > 0 || to + 1 < total_cols)
                                                    .then_some((from, to));
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
                        if total_cols >= 3 {
                            brush.1 = pos.0.clamp(rect.x, rect.x + rect.w);
                        }
                        cx.notify();
                        return;
                    }
                    let next = pick_rc(pos);
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
                    st.hover = pick_rc(pos);
                    if rect.contains(pos) && n_rows > 0 && n_cols > 0 {
                        st.brush = Some((pos.0, pos.0));
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let pos = local(event.position, up_bounds.read(cx).bounds);
                up_in(Some(pos), cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| up_out(None, cx))
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
                if total_cols < 3 || rect.w <= 0.0 {
                    return;
                }
                let pos = local(ev.position, wheeled.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                if ev.source == ScrollSource::Touch {
                    if zoomed.is_some() && dx.abs() > dy.abs() && dx.abs() >= 0.01 {
                        wheeled.update(cx, |st, cx| {
                            st.span = pan_span(st.span, total_cols, dx, rect.w);
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
                    st.span = zoom_span(st.span, total_cols, factor, anchor_t);
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
                if (scale - 1.0).abs() < 0.001 || total_cols < 3 || rect.w <= 0.0 {
                    pinched.update(cx, |st, _| st.brush = None);
                    return;
                }
                let pos = local(ev.position, pinched.read(cx).bounds);
                let anchor_t = ((pos.0 - rect.x) / rect.w).clamp(0.0, 1.0);
                pinched.update(cx, |st, cx| {
                    st.brush = None;
                    st.span = zoom_span(st.span, total_cols, 1.0 / scale, anchor_t);
                    cx.notify();
                });
                cx.stop_propagation();
            })
            .child(
                paint_horizon_canvas(HorizonPaint {
                    rows,
                    xs,
                    rect,
                    bands,
                    hovered_rc,
                    is_pinned,
                    brush,
                    total_cols,
                    hairline,
                    colors,
                })
                .absolute()
                .inset_0(),
            )
            .children(x_labels)
            .children(y_labels)
            .children(tooltip)
            .children(show_all)
            .child(measure(state, |st| &mut st.bounds));
        root.flex().flex_col().gap_2().children(legend).child(plot)
    }
}
