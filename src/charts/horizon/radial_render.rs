use super::*;

impl RenderOnce for RadialLineChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<RadialState> =
            window.use_keyed_state((self.id.clone(), "radial-line"), cx, |_, _| {
                RadialState::default()
            });
        let n = self.labels.len();
        let (bounds, state_hover, state_pinned, hidden, rotation, zoom) = {
            let st = state.read(cx);
            (
                st.bounds,
                st.hover,
                st.pinned,
                st.hidden.clone(),
                st.rotation,
                if st.zoom <= 0.0 { 1.0 } else { st.zoom },
            )
        };
        let hover = state_hover.filter(|ix| *ix < n);
        let pinned = state_pinned.filter(|ix| *ix < n);
        let active = pinned.or(hover);
        let shown: Vec<(usize, &Series)> = self
            .series
            .iter()
            .enumerate()
            .filter(|(_, s)| !hidden.contains(&s.name))
            .collect();
        let shown_series: Vec<&Series> = shown.iter().map(|(_, s)| *s).collect();
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let room = pixels(sizes.foot);
        let gap = pixels(sizes.inset);
        let center = (width * 0.5, height * 0.5);
        let base_outer = (width.min(height) * 0.5 - room).max(0.0);
        let outer = base_outer * zoom;
        let layout = compute_radial(
            n,
            &shown_series,
            self.range_band.as_ref(),
            center,
            outer,
            rotation,
        );
        let format = self.format.clone();
        let (fg_color, muted_color, subtle_color) = (colors.fg, colors.fg_muted, colors.fg_subtle);
        let text = |content: String, strong: bool| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if strong { fg_color } else { muted_color })
                .child(content)
        };

        let every = if n == 0 || outer <= 0.0 {
            1
        } else {
            ((n as f32 * pixels(sizes.label) / (TAU * outer).max(1.0)).ceil() as usize).max(1)
        };
        let spoke_labels: Vec<_> = (0..n)
            .filter(|ix| ix % every == 0)
            .map(|ix| {
                let a = radial_angle(ix, n, rotation);
                let (dx, dy) = (a.cos(), a.sin());
                let (lx, ly) = (center.0 + (outer + gap) * dx, center.1 + (outer + gap) * dy);
                let box_div = div()
                    .absolute()
                    .left(Pixels::from(lx))
                    .top(Pixels::from(ly))
                    .w_0()
                    .h_0()
                    .flex();
                let box_div = if dx > 0.3 {
                    box_div.justify_start()
                } else if dx < -0.3 {
                    box_div.justify_end()
                } else {
                    box_div.justify_center()
                };
                let box_div = if dy > 0.3 {
                    box_div.items_start()
                } else if dy < -0.3 {
                    box_div.items_end()
                } else {
                    box_div.items_center()
                };
                box_div.child(text(self.labels[ix].to_string(), active == Some(ix)))
            })
            .collect();

        let ring_labels: Vec<_> = layout
            .rings
            .iter()
            .skip(1)
            .map(|&(r, val)| {
                div()
                    .absolute()
                    .left(Pixels::from(center.0))
                    .top(Pixels::from(center.1 - r))
                    .h_0()
                    .flex()
                    .items_center()
                    .pl_1p5()
                    .child(tabular(text(format(val), false)).text_color(subtle_color))
            })
            .collect();

        let tooltip = active.map(|ix| {
            let mut card = ChartTooltip::new(self.labels[ix].clone());
            for &(orig_ix, s) in &shown {
                card = card.row(
                    Some(tint(&colors, orig_ix)),
                    s.name.clone(),
                    format(s.values[ix]),
                );
            }
            if let Some((ref low, ref high)) = self.range_band {
                card = card.row(
                    None,
                    "Range",
                    format!("{} – {}", format(low[ix]), format(high[ix])),
                );
            }
            let a = radial_angle(ix, n, rotation);
            let reach = outer.min(base_outer) * 0.78;
            let pos = (center.0 + reach * a.cos(), center.1 + reach * a.sin());
            anchored(pos, pos.0 > width * 0.55 || a.cos() > 0.0, card)
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
        let modified = rotation.abs() > 1e-4 || (zoom - 1.0).abs() > 1e-3 || pinned.is_some();
        let reset_btn = modified.then(|| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "radial-reset"), "Reset")
                .debug_selector(|| "radial-reset".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.rotation = 0.0;
                        st.zoom = 1.0;
                        st.pinned = None;
                        st.hover = None;
                        st.drag = None;
                        cx.notify();
                    })
                }))
        });

        let rings = layout.rings;
        let band = layout.band;
        let painted_series: Vec<(usize, PointRing)> = shown
            .iter()
            .zip(layout.series_pts)
            .map(|((orig_ix, _), pts)| (*orig_ix, pts))
            .collect();
        let inner = layout.inner;
        let filled = self.filled;
        let (hairline, stroke) = (sizes.hairline, sizes.stroke.to_pixels(rem));
        let hit_outer = if touch_mode && outer > 0.0 {
            (outer + room * 0.5).max(24.0)
        } else {
            outer + room * 0.5
        };
        let pick_spoke = move |pos: (f32, f32)| -> Option<usize> {
            if n == 0 || outer <= 0.0 {
                return None;
            }
            let (_, dist) = bearing(center, pos);
            (dist <= hit_outer).then(|| radial_spoke_at(n, center, pos, rotation))
        };
        let (moved, pressed, released, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let released_end = released.clone();
        let end_drag = move |pos: Option<(f32, f32)>, cx: &mut App| {
            released_end.update(cx, |st, cx| {
                let Some((_, down_pos, dragged)) = st.drag.take() else {
                    return;
                };
                if !dragged
                    && let Some(place) = pos
                    && n > 0
                    && outer > 0.0
                {
                    let (_, dist) = bearing(center, place);
                    if dist <= hit_outer {
                        let ix = radial_spoke_at(n, center, down_pos, st.rotation);
                        st.pinned = if st.pinned == Some(ix) {
                            None
                        } else {
                            Some(ix)
                        };
                    }
                }
                cx.notify();
            })
        };
        let (up_in, up_out) = (end_drag.clone(), end_drag);
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let plot = div()
            .id((self.id.clone(), "radial-line"))
            .debug_selector(|| "radial-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .cursor_grab()
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some((prev_angle, down_pos, dragged)) = st.drag.as_mut()
                    {
                        let dist = (pos.0 - down_pos.0).hypot(pos.1 - down_pos.1);
                        if *dragged || dist > 4.0 {
                            *dragged = true;
                            let (angle, _) = bearing(center, pos);
                            let mut da = angle - *prev_angle;
                            if da > std::f32::consts::PI {
                                da -= TAU;
                            }
                            if da < -std::f32::consts::PI {
                                da += TAU;
                            }
                            st.rotation = (st.rotation + da).rem_euclid(TAU);
                            *prev_angle = angle;
                            cx.notify();
                        }
                        return;
                    }
                    let next = pick_spoke(pos);
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
                        st.rotation = 0.0;
                        st.zoom = 1.0;
                        st.pinned = None;
                        st.drag = None;
                        cx.notify();
                        return;
                    }
                    st.hover = pick_spoke(pos);
                    if n > 0 && outer > 0.0 {
                        let (angle, dist) = bearing(center, pos);
                        if dist <= hit_outer {
                            st.drag = Some((angle, pos, false));
                        }
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let bounds = released.read(cx).bounds;
                up_in(Some(local(event.position, bounds)), cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                up_out(None, cx);
            })
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
                if ev.source == ScrollSource::Touch {
                    if dx.abs() > dy.abs() && dx.abs() >= 0.2 {
                        scrolled.update(cx, |st, cx| {
                            st.rotation = (st.rotation + dx * 0.008).rem_euclid(TAU);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                    return;
                }
                if dy.abs() >= 0.05 && base_outer > 0.0 {
                    scrolled.update(cx, |st, cx| {
                        let z = if st.zoom <= 0.0 { 1.0 } else { st.zoom };
                        st.zoom = (z * (1.0 + dy * 0.002)).clamp(0.5, 3.0);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                if !ev.delta.is_finite() || ev.phase == TouchPhase::Cancelled {
                    pinched.update(cx, |st, _| st.drag = None);
                    return;
                }
                let scale = ev.delta.exp().clamp(0.25, 4.0);
                pinched.update(cx, |st, cx| {
                    st.drag = None;
                    if (scale - 1.0).abs() > 1e-4 && base_outer > 0.0 {
                        let z = if st.zoom <= 0.0 { 1.0 } else { st.zoom };
                        st.zoom = (z * scale).clamp(0.5, 3.0);
                        cx.notify();
                    }
                });
                if (scale - 1.0).abs() > 1e-4 && base_outer > 0.0 {
                    cx.stop_propagation();
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let center_pt = bounds.center();
                        let segs = 56;
                        for &(ring_r, _) in &rings {
                            if ring_r <= 0.5 {
                                continue;
                            }
                            let mut ring_path = PathBuilder::stroke(hairline);
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
                        for ix in 0..n {
                            let is_active = active == Some(ix);
                            if ix % every != 0 && !is_active && n > 24 {
                                continue;
                            }
                            let a = radial_angle(ix, n, rotation);
                            let mut spoke = PathBuilder::stroke(if pinned == Some(ix) {
                                stroke
                            } else {
                                hairline
                            });
                            spoke.move_to(toward(center_pt, inner, a));
                            spoke.line_to(toward(center_pt, outer, a));
                            finish(
                                spoke,
                                if pinned == Some(ix) {
                                    colors.fg
                                } else if is_active {
                                    colors.border_strong
                                } else {
                                    colors.border.opacity(0.45)
                                },
                                window,
                            );
                        }
                        if let Some((ref low_pts, ref high_pts)) = band
                            && low_pts.len() >= 2
                        {
                            let mut area = PathBuilder::fill();
                            area.move_to(at(origin, high_pts[0]));
                            for &pt in &high_pts[1..] {
                                area.line_to(at(origin, pt));
                            }
                            area.line_to(at(origin, high_pts[0]));
                            area.line_to(at(origin, low_pts[0]));
                            for &pt in low_pts.iter().rev() {
                                area.line_to(at(origin, pt));
                            }
                            area.close();
                            finish(area, tint(&colors, 0).opacity(0.16), window);
                        }
                        for (orig_ix, pts) in &painted_series {
                            let ink = tint(&colors, *orig_ix);
                            if pts.len() >= 2 {
                                if filled {
                                    let mut area = PathBuilder::fill();
                                    if inner > 0.5 && pts.len() >= 3 {
                                        area.move_to(at(origin, pts[0]));
                                        for &pt in &pts[1..] {
                                            area.line_to(at(origin, pt));
                                        }
                                        area.line_to(at(origin, pts[0]));
                                        let first_inner =
                                            toward(center_pt, inner, radial_angle(0, n, rotation));
                                        area.line_to(first_inner);
                                        for ix in (0..n).rev() {
                                            area.line_to(toward(
                                                center_pt,
                                                inner,
                                                radial_angle(ix, n, rotation),
                                            ));
                                        }
                                        area.close();
                                    } else {
                                        paint_closed_loop(&mut area, pts, origin);
                                    }
                                    finish(area, ink.opacity(0.18), window);
                                }
                                let mut edge = PathBuilder::stroke(stroke);
                                paint_closed_loop(&mut edge, pts, origin);
                                finish(edge, ink, window);
                            } else if let Some(&pt) = pts.first() {
                                ring(
                                    at(origin, pt),
                                    stroke * 2.0,
                                    (colors.bg, ink),
                                    stroke,
                                    window,
                                );
                            }
                        }
                        if let Some(ix) = active {
                            for (orig_ix, pts) in &painted_series {
                                if let Some(&pt) = pts.get(ix) {
                                    ring(
                                        at(origin, pt),
                                        stroke * 2.0,
                                        (colors.bg, tint(&colors, *orig_ix)),
                                        stroke,
                                        window,
                                    );
                                }
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(ring_labels)
            .children(spoke_labels)
            .children(tooltip)
            .children(reset_btn)
            .child(measure(state, |st| &mut st.bounds));
        root.flex().flex_col().gap_2().children(legend).child(plot)
    }
}
