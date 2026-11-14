use super::*;

impl RenderOnce for SurfaceChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<SurfaceState> =
            window.use_keyed_state((self.id.clone(), "surface"), cx, |_, _| {
                SurfaceState::default()
            });
        let (bounds, hover_raw, az_off, el_off, zoom) = {
            let st = state.read(cx);
            (
                st.bounds,
                st.hover,
                st.azimuth_offset,
                st.elevation_offset,
                st.zoom,
            )
        };
        let azimuth_deg = (self.azimuth_deg + az_off).rem_euclid(360.0);
        let elevation_deg = (self.elevation_deg + el_off).clamp(-80.0, 80.0);
        let zoom = zoom.clamp(0.5, 4.0);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.inset),
            pixels(sizes.inset),
            pixels(sizes.inset),
        );
        let scene = build_surface_scene(
            self.cols,
            self.rows,
            &self.values,
            azimuth_deg,
            elevation_deg,
            zoom,
            rect,
        );
        let hover = hover_raw.filter(|&ix| ix < scene.quads.len());
        let format = self.format.clone();
        let low_color = colors.chart[1];
        let high_color = colors.chart[0];
        let steps = [0.0, 0.25, 0.5, 0.75, 1.0].map(|t| low_color.mix(&high_color, t));
        let legend = key(steps, ("Low", "High"), cx);

        let tooltip = hover.map(|ix| {
            let quad = &scene.quads[ix];
            let fill_color = low_color.mix(&high_color, quad.t);
            let card = ChartTooltip::new(format!("({}, {})", quad.col, quad.row))
                .row(Some(fill_color), "Z", format(quad.z))
                .row(None, "Col", quad.col.to_string())
                .row(None, "Row", quad.row.to_string());
            anchored(quad.center, quad.center.0 > box_size.0 * 0.55, card)
        });

        let labels: Vec<Div> = scene
            .axis_labels
            .iter()
            .map(|((lx, ly), text)| {
                div()
                    .absolute()
                    .left(Pixels::from(*lx))
                    .top(Pixels::from(*ly))
                    .w_0()
                    .h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex_none()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(text.clone()),
                    )
            })
            .collect();

        let hit_quads: Vec<[(f32, f32); 4]> = scene.quads.iter().map(|q| q.pts).collect();
        let hit = Rc::new(move |pos: (f32, f32)| {
            // Scan front-to-back so the visually topmost quad wins.
            hit_quads
                .iter()
                .enumerate()
                .rev()
                .find(|(_, pts)| point_in_quad(pos, **pts))
                .map(|(ix, _)| ix)
        });
        let paint_quads = scene.quads;
        let paint_axes = scene.axes;
        let wireframe = self.wireframe;
        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);
        let modified = az_off.abs() > 1e-3 || el_off.abs() > 1e-3 || (zoom - 1.0).abs() > 1e-3;
        let reset_btn = modified.then(|| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "reset"), "Reset")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm);
            div()
                .debug_selector(|| "surface-reset".into())
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button.on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.azimuth_offset = 0.0;
                        st.elevation_offset = 0.0;
                        st.zoom = 1.0;
                        st.drag_last = None;
                        cx.notify();
                    })
                }))
        });
        let base_el = self.elevation_deg;
        let (pressed, moved, released, released_out, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (press_hit, move_hit) = (hit.clone(), hit);

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let chart = div()
            .id((self.id.clone(), "surface"))
            .debug_selector(|| "surface-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_pos(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |st, cx| {
                    if event.click_count >= 2 {
                        st.azimuth_offset = 0.0;
                        st.elevation_offset = 0.0;
                        st.zoom = 1.0;
                        st.drag_last = None;
                        st.hover = press_hit(pos);
                        cx.notify();
                        return;
                    }
                    st.drag_last = Some(pos);
                    st.hover = press_hit(pos);
                    cx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, moved.read(cx).bounds);
                moved.update(cx, |st, cx| {
                    if let (Some(last), Some(MouseButton::Left)) =
                        (st.drag_last, event.pressed_button)
                    {
                        let dx = pos.0 - last.0;
                        let dy = pos.1 - last.1;
                        if dx.abs() > 0.01 || dy.abs() > 0.01 {
                            st.azimuth_offset = wrap_deg(st.azimuth_offset + dx * 0.5);
                            let next_el =
                                (base_el + st.elevation_offset + dy * 0.5).clamp(-80.0, 80.0);
                            st.elevation_offset = next_el - base_el;
                            st.drag_last = Some(pos);
                            st.hover = None;
                            cx.notify();
                        }
                        return;
                    }
                    st.drag_last = None;
                    let next = move_hit(pos);
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released.update(cx, |st, _| {
                    st.drag_last = None;
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released_out.update(cx, |st, _| {
                    st.drag_last = None;
                });
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if !dx.is_finite() || !dy.is_finite() {
                    return;
                }
                scrolled.update(cx, |st, cx| {
                    if ev.source == ScrollSource::Touch {
                        if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                            st.azimuth_offset = wrap_deg(st.azimuth_offset + dx * 0.5);
                            let next_el =
                                (base_el + st.elevation_offset + dy * 0.5).clamp(-80.0, 80.0);
                            st.elevation_offset = next_el - base_el;
                            st.hover = None;
                            cx.notify();
                        }
                    } else if dy.abs() >= 0.01 {
                        let factor = (1.0 - dy * 0.002).clamp(0.1, 10.0);
                        st.zoom = (st.zoom * factor).clamp(0.5, 4.0);
                        cx.notify();
                    }
                });
                cx.stop_propagation();
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                pinched.update(cx, |st, cx| {
                    st.drag_last = None;
                    if ev.phase != TouchPhase::Cancelled && ev.delta.is_finite() {
                        let scale = ev.delta.exp().clamp(0.1, 10.0);
                        if (scale - 1.0).abs() > 1e-4 {
                            let next_zoom = (st.zoom * scale).clamp(0.5, 4.0);
                            if (next_zoom - st.zoom).abs() > f32::EPSILON {
                                st.zoom = next_zoom;
                                cx.notify();
                            }
                        }
                    }
                });
                cx.stop_propagation();
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |st, cx| {
                        st.hover = None;
                        st.drag_last = None;
                        cx.notify();
                    });
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        let axis_color = colors.border.opacity(0.75);
                        for &(a, b) in &paint_axes {
                            let mut path = PathBuilder::stroke(hairline);
                            path.move_to(at(origin, a));
                            path.line_to(at(origin, b));
                            finish(path, axis_color, window);
                        }
                        let wire_color = colors.fg.opacity(0.22);
                        for (ix, quad) in paint_quads.iter().enumerate() {
                            let is_hover = hover == Some(ix);
                            let fill_color = low_color
                                .mix(&high_color, quad.t)
                                .opacity(if is_hover { 1.0 } else { 0.92 });
                            let mut fill_path = PathBuilder::fill();
                            fill_path.move_to(at(origin, quad.pts[0]));
                            for &pt in &quad.pts[1..] {
                                fill_path.line_to(at(origin, pt));
                            }
                            fill_path.close();
                            finish(fill_path, fill_color, window);
                            if wireframe || is_hover {
                                let mut edge =
                                    PathBuilder::stroke(if is_hover { stroke } else { hairline });
                                edge.move_to(at(origin, quad.pts[0]));
                                for &pt in &quad.pts[1..] {
                                    edge.line_to(at(origin, pt));
                                }
                                edge.close();
                                finish(edge, if is_hover { colors.fg } else { wire_color }, window);
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels)
            .children(reset_btn)
            .children(tooltip)
            .child(measure(state, |st| &mut st.bounds));

        root.flex().flex_col().gap_2().child(chart).child(legend)
    }
}
