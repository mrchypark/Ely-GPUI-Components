use super::*;

impl RenderOnce for GeoChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<GeoState> =
            window.use_keyed_state((self.id.clone(), "geo"), cx, |_, _| GeoState::default());
        let (bounds, yaw_offset, pitch_offset, zoom, pan, hover, show_reset) = {
            let s = state.read(cx);
            let pick_count = self.points.len() + self.regions.len();
            (
                s.bounds,
                s.yaw_offset,
                s.pitch_offset,
                s.zoom.clamp(0.5, 12.0),
                s.pan,
                s.pointed(pick_count),
                s.is_modified(),
            )
        };
        let view_rotate = (
            self.rotate.0 + yaw_offset,
            (self.rotate.1 + pitch_offset).clamp(-89.0, 89.0),
        );
        let theme = cx.theme();
        let touch_mode = theme.touch_mode;
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pad = f32::from(sizes.inset.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let scene = build_scene(
            &self,
            box_size,
            pad,
            GeoView {
                rotate: view_rotate,
                zoom,
                pan,
            },
        );
        let region_ink = tint(&colors, 0);
        let point_ink = if self.regions.is_empty() {
            tint(&colors, 0)
        } else {
            tint(&colors, 1)
        };
        let (low, high) = region_extent(&self.regions);
        let format = self.format.clone();

        let tooltip = hover.and_then(|ix| {
            if ix < self.points.len() {
                let pt = scene.points.iter().find(|p| p.index == ix)?;
                let raw = &self.points[ix];
                let card = ChartTooltip::new(raw.name.clone())
                    .row(Some(point_ink), "Value", format(raw.value))
                    .row(None, "Coords", format!("{:.1}°, {:.1}°", raw.lat, raw.lon));
                Some(anchored(pt.center, pt.center.0 > box_size.0 * 0.55, card))
            } else {
                let r_ix = ix - self.points.len();
                let reg = scene.regions.iter().find(|r| r.index == r_ix)?;
                let raw = &self.regions[r_ix];
                let alpha = strength(raw.value, low, high);
                let card = ChartTooltip::new(raw.name.clone()).row(
                    Some(region_ink.opacity(alpha)),
                    "Value",
                    format(raw.value),
                );
                Some(anchored(reg.anchor, reg.anchor.0 > box_size.0 * 0.55, card))
            }
        });

        let hit_points: Vec<(usize, (f32, f32), f32)> = scene
            .points
            .iter()
            .map(|p| (p.index, p.center, p.radius))
            .collect();
        type HitRegionRings = (usize, Vec<Vec<(f32, f32)>>);
        let hit_regions: Vec<HitRegionRings> = scene
            .regions
            .iter()
            .map(|r| (r.index, r.rings.clone()))
            .collect();
        let points_offset = self.points.len();

        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);
        let globe = scene.globe;
        let frame_rect = (scene.frame_x, scene.frame_y, scene.frame_w, scene.frame_h);
        let graticule = scene.graticule;
        let paint_regions = scene.regions;
        let paint_points = scene.points;
        let min_point_hit = if touch_mode { 24.0 } else { 8.0 };

        let find: Rc<dyn Fn((f32, f32)) -> Option<usize>> = Rc::new(move |pos| {
            if let Some((ix, _, _)) = hit_points
                .iter()
                .filter_map(|&(ix, c, r)| {
                    let d2 = (pos.0 - c.0).powi(2) + (pos.1 - c.1).powi(2);
                    let limit = (r + 5.0).max(min_point_hit);
                    (d2 <= limit * limit).then_some((ix, c, d2))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
            {
                return Some(ix);
            }
            for (r_ix, rings) in hit_regions.iter().rev() {
                let mut inside = false;
                for ring in rings {
                    if point_in_polygon(pos, ring) {
                        inside = !inside;
                    }
                }
                if inside {
                    return Some(points_offset + *r_ix);
                }
            }
            None
        });

        let base_rotate = self.rotate;
        let (down_state, move_state, up_state, scroll_state, pinch_state, hover_state) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (down_find, move_find) = (find.clone(), find);
        let clear_drag = move |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
            up_state.update(cx, |s, _| {
                s.drag_last = None;
            });
        };

        let reset_button = show_reset.then(|| {
            let reset_state = state.clone();
            div()
                .absolute()
                .top_1()
                .right_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    Button::new((self.id.clone(), "geo-reset"), "Reset")
                        .debug_selector(|| "geo-reset".into())
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, _, cx| {
                            reset_state.update(cx, |s, cx| {
                                s.reset();
                                cx.notify();
                            });
                        }),
                )
        });

        let map_area = div()
            .debug_selector(|| "geo-plot".into())
            .id((self.id.clone(), "geo-map"))
            .relative()
            .flex_1()
            .min_h_0()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_pos(event.position, down_state.read(cx).bounds.origin);
                down_state.update(cx, |s, cx| {
                    if event.click_count >= 2 {
                        s.reset();
                        s.hover = down_find(pos);
                        cx.notify();
                    } else {
                        s.drag_last = Some(pos);
                        let next = down_find(pos);
                        if s.hover != next {
                            s.hover = next;
                            cx.notify();
                        }
                    }
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, move_state.read(cx).bounds.origin);
                move_state.update(cx, |s, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let Some(prev) = s.drag_last
                    {
                        let dx = pos.0 - prev.0;
                        let dy = pos.1 - prev.1;
                        s.drag_last = Some(pos);
                        if dx.abs() > 0.0 || dy.abs() > 0.0 {
                            s.rotate_by(dx, dy, base_rotate.1);
                            s.hover = None;
                            cx.notify();
                        }
                        return;
                    }
                    s.drag_last = None;
                    let next = move_find(pos);
                    if s.hover != next {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, clear_drag.clone())
            .on_mouse_up_out(MouseButton::Left, clear_drag)
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if ev.source == ScrollSource::Touch {
                    if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                        scroll_state.update(cx, |s, cx| {
                            s.rotate_by(dx, dy, base_rotate.1);
                            s.hover = None;
                            cx.notify();
                        });
                    }
                } else if dy.abs() >= 0.01 {
                    let factor = (dy * 0.0025).exp().clamp(0.5, 2.0);
                    scroll_state.update(cx, |s, cx| {
                        if s.zoom_by(factor) {
                            cx.notify();
                        }
                    });
                }
                cx.stop_propagation();
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                pinch_state.update(cx, |s, cx| {
                    s.drag_last = None;
                    if ev.phase != TouchPhase::Cancelled
                        && ev.delta.is_finite()
                        && ev.delta.abs() > 0.0001
                    {
                        let scale = ev.delta.exp().clamp(0.1, 10.0);
                        if s.zoom_by(scale) {
                            cx.notify();
                        }
                    }
                });
                cx.stop_propagation();
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    hover_state.update(cx, |s, cx| {
                        if s.hover.take().is_some() {
                            cx.notify();
                        }
                    });
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            if let Some((cx_px, cy_px, r_px)) = globe
                                && r_px > 1.0
                            {
                                let segs = 72;
                                let mut fill_circle = PathBuilder::fill();
                                let mut edge_circle = PathBuilder::stroke(hairline);
                                let p0 = (cx_px + r_px, cy_px);
                                fill_circle.move_to(at(origin, p0));
                                edge_circle.move_to(at(origin, p0));
                                for s in 1..=segs {
                                    let a = std::f32::consts::TAU * s as f32 / segs as f32;
                                    let pt = (cx_px + r_px * a.cos(), cy_px + r_px * a.sin());
                                    fill_circle.line_to(at(origin, pt));
                                    edge_circle.line_to(at(origin, pt));
                                }
                                fill_circle.close();
                                edge_circle.close();
                                finish(fill_circle, colors.fg.opacity(0.03), window);
                                finish(edge_circle, colors.border.opacity(0.7), window);
                            } else if frame_rect.2 > 1.0 && frame_rect.3 > 1.0 {
                                let (fx, fy, fw, fh) = frame_rect;
                                let mut box_path = PathBuilder::stroke(hairline);
                                box_path.move_to(at(origin, (fx, fy)));
                                box_path.line_to(at(origin, (fx + fw, fy)));
                                box_path.line_to(at(origin, (fx + fw, fy + fh)));
                                box_path.line_to(at(origin, (fx, fy + fh)));
                                box_path.close();
                                finish(box_path, colors.border.opacity(0.55), window);
                            }

                            for line in &graticule {
                                if line.len() < 2 {
                                    continue;
                                }
                                let mut path = PathBuilder::stroke(hairline);
                                path.move_to(at(origin, line[0]));
                                for &pt in &line[1..] {
                                    path.line_to(at(origin, pt));
                                }
                                finish(path, colors.border.opacity(0.45), window);
                            }

                            for reg in &paint_regions {
                                let is_hovered = hover == Some(points_offset + reg.index);
                                let alpha = strength(reg.value, low, high);
                                let fill_color = region_ink.opacity(if is_hovered {
                                    (alpha + 0.14).min(1.0)
                                } else {
                                    alpha
                                });
                                let border_color = if is_hovered {
                                    colors.fg
                                } else {
                                    colors.bg.opacity(0.8)
                                };
                                let border_w = if is_hovered { stroke } else { hairline };
                                for ring_pts in &reg.rings {
                                    if ring_pts.len() < 3 {
                                        continue;
                                    }
                                    let mut fill_path = PathBuilder::fill();
                                    let mut edge_path = PathBuilder::stroke(border_w);
                                    fill_path.move_to(at(origin, ring_pts[0]));
                                    edge_path.move_to(at(origin, ring_pts[0]));
                                    for &pt in &ring_pts[1..] {
                                        fill_path.line_to(at(origin, pt));
                                        edge_path.line_to(at(origin, pt));
                                    }
                                    fill_path.close();
                                    edge_path.close();
                                    finish(fill_path, fill_color, window);
                                    finish(edge_path, border_color, window);
                                }
                            }

                            for pt in &paint_points {
                                let is_hovered = hover == Some(pt.index);
                                let edge = if is_hovered { colors.fg } else { point_ink };
                                let width = if is_hovered { stroke } else { hairline };
                                ring(
                                    at(origin, pt.center),
                                    Pixels::from(pt.radius),
                                    (point_ink.opacity(0.75), edge),
                                    width,
                                    window,
                                );
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(tooltip)
            .children(reset_button)
            .child(measure(state, |s| &mut s.bounds));

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        if self.regions.is_empty() {
            root.flex().flex_col().child(map_area)
        } else {
            let legend = key(steps(region_ink), ("Low", "High"), cx);
            root.flex().flex_col().gap_2().child(map_area).child(legend)
        }
    }
}
