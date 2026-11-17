use super::*;

pub(super) fn svg_hex(color: Hsla) -> (String, f32) {
    let rgba: gpui::Rgba = color.into();
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            byte(rgba.r),
            byte(rgba.g),
            byte(rgba.b)
        ),
        rgba.a,
    )
}

pub(super) fn svg_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

pub(super) fn transform_point(p: (f32, f32), zoom: f32, pan: (f32, f32)) -> (f32, f32) {
    (p.0 * zoom + pan.0, p.1 * zoom + pan.1)
}

pub(super) fn local_point(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

pub(super) fn hit_node_index(nodes: &[HitNode], pos: (f32, f32), reach: f32) -> Option<usize> {
    let limit = reach * reach;
    nodes
        .iter()
        .enumerate()
        .map(|(ix, node)| {
            (
                ix,
                (pos.0 - node.pos.0).powi(2) + (pos.1 - node.pos.1).powi(2),
            )
        })
        .filter(|(_, d2)| *d2 <= limit)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(ix, _)| ix)
}

impl Styled for TreeChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for TreeChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<TreeState> =
            window.use_keyed_state((self.id.clone(), "tree"), cx, |_, _| TreeState::default());
        let (bounds, zoom, pan, collapsed, modified) = {
            let s = state.read(cx);
            (
                s.bounds,
                s.zoom,
                s.pan,
                s.collapsed.clone(),
                s.is_modified(),
            )
        };
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let theme = cx.theme();
        let hit_reach = if theme.touch_mode { 24.0 } else { 18.0 };
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let font_px = pixels(theme.text_size(TextSize::Xs));
        let label_room = pixels(sizes.label);
        let mut layout = compute_layout(
            self.root.as_ref(),
            self.mode,
            self.horizontal,
            (width, height),
            pixels(sizes.inset),
            label_room,
            &collapsed,
        );
        for node in &mut layout.nodes {
            (node.x, node.y) = transform_point((node.x, node.y), zoom, pan);
        }
        for link in &mut layout.links {
            link.from = transform_point(link.from, zoom, pan);
            link.ctrl1 = transform_point(link.ctrl1, zoom, pan);
            link.ctrl2 = transform_point(link.ctrl2, zoom, pan);
            link.to = transform_point(link.to, zoom, pan);
        }
        let hover = state.read(cx).pointed(layout.nodes.len());

        let tooltip = hover.map(|ix| {
            let node = &layout.nodes[ix];
            let ink = if node.depth == 0 {
                colors.fg
            } else {
                tint(&colors, node.branch)
            };
            let children_value = if node.is_collapsed {
                format!("{} (collapsed)", node.total_children)
            } else {
                node.total_children.to_string()
            };
            let card = ChartTooltip::new(node.path.join(" › "))
                .row(Some(ink), "Weight", compact(node.weight))
                .row(None, "Children", children_value);
            anchored((node.x, node.y), node.x > width * 0.55, card)
        });

        let radial = matches!(
            self.mode,
            TreeLayoutMode::RadialTree | TreeLayoutMode::RadialCluster
        );
        let (fg, fg_muted) = (colors.fg, colors.fg_muted);
        let labels: Vec<Div> = select_labels(
            &layout.nodes,
            radial,
            hover,
            (width, height),
            font_px,
            label_room,
        )
        .into_iter()
        .map(|lbl| {
            let word = div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if lbl.depth == 0 { fg } else { fg_muted })
                .child(lbl.text);
            let anchor = div()
                .absolute()
                .left(Pixels::from(lbl.ax))
                .top(Pixels::from(lbl.ay))
                .h_0()
                .flex()
                .items_center();
            match lbl.align {
                LabelAlign::Start => anchor.child(word),
                LabelAlign::End => anchor.w_0().justify_end().child(word),
                LabelAlign::Center => anchor.w_0().justify_center().child(word),
            }
        })
        .collect();

        let hit_nodes: Rc<Vec<HitNode>> = Rc::new(
            layout
                .nodes
                .iter()
                .map(|n| HitNode {
                    pos: (n.x, n.y),
                    path_key: n.path_key.clone(),
                    expandable: n.total_children > 0,
                })
                .collect(),
        );
        let paint_nodes = layout.nodes;
        let paint_links = layout.links;
        let hairline = sizes.hairline;
        let stroke = sizes.stroke.to_pixels(rem);

        let reset_button = modified.then(|| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "reset"), "Reset")
                .debug_selector(|| "tree-reset".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    reset.update(cx, |s, cx| {
                        s.reset();
                        cx.notify();
                    });
                });
            div().absolute().top_0().right_0().child(button)
        });

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let (pressed, moved, released, released_out, left, scrolled, pinched) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (down_hits, move_hits, up_hits) = (hit_nodes.clone(), hit_nodes.clone(), hit_nodes);

        let viewport = div()
            .id((self.id.clone(), "tree"))
            .relative()
            .size_full()
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_point(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |s, cx| {
                    if event.click_count >= 2 {
                        s.reset();
                        s.hover = None;
                        cx.notify();
                        return;
                    }
                    s.drag_start = Some(pos);
                    s.drag_last = Some(pos);
                    s.dragged = false;
                    s.hover = hit_node_index(&down_hits, pos, hit_reach);
                    cx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_point(event.position, moved.read(cx).bounds);
                moved.update(cx, |s, cx| {
                    if event.pressed_button == Some(MouseButton::Left)
                        && let (Some(start), Some(last)) = (s.drag_start, s.drag_last)
                    {
                        let dist = (pos.0 - start.0).hypot(pos.1 - start.1);
                        if s.dragged || dist > 4.0 {
                            s.pan.0 += pos.0 - last.0;
                            s.pan.1 += pos.1 - last.1;
                            s.drag_last = Some(pos);
                            if !s.dragged {
                                s.dragged = true;
                                s.hover = None;
                            }
                            cx.notify();
                        }
                        return;
                    }
                    let next = hit_node_index(&move_hits, pos, hit_reach);
                    if s.hover != next {
                        s.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let pos = local_point(event.position, released.read(cx).bounds);
                released.update(cx, |s, cx| {
                    let was_dragged = s.dragged;
                    let had_press = s.drag_start.take().is_some();
                    s.drag_last = None;
                    s.dragged = false;
                    if had_press
                        && !was_dragged
                        && event.click_count < 2
                        && let Some(ix) = hit_node_index(&up_hits, pos, hit_reach)
                        && let Some(node) = up_hits.get(ix)
                        && node.expandable
                    {
                        if !s.collapsed.remove(&node.path_key) {
                            s.collapsed.insert(node.path_key.clone());
                        }
                        s.hover = Some(ix);
                        cx.notify();
                    } else if had_press {
                        cx.notify();
                    }
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                released_out.update(cx, |s, cx| {
                    if s.drag_start.take().is_some() || s.dragged {
                        s.drag_last = None;
                        s.dragged = false;
                        cx.notify();
                    }
                });
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |s, cx| {
                        if s.hover.take().is_some() {
                            cx.notify();
                        }
                    });
                }
            })
            .on_scroll(move |ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(Pixels::from(16.0));
                let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
                if ev.source == ScrollSource::Touch {
                    if dx.abs() >= 0.01 || dy.abs() >= 0.01 {
                        scrolled.update(cx, |s, cx| {
                            s.pan.0 += dx;
                            s.pan.1 += dy;
                            s.hover = None;
                            cx.notify();
                        });
                        cx.stop_propagation();
                    }
                } else if dy.abs() >= 0.01 {
                    let pointer = local_point(ev.position, scrolled.read(cx).bounds);
                    let scale = (dy * 0.002).exp().clamp(0.1, 10.0);
                    scrolled.update(cx, |s, cx| {
                        s.zoom_at(scale, pointer);
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .on_pinch(move |ev: &PinchEvent, _, cx| {
                let pointer = local_point(ev.position, pinched.read(cx).bounds);
                pinched.update(cx, |s, cx| {
                    s.drag_start = None;
                    s.drag_last = None;
                    s.dragged = false;
                    if ev.phase != TouchPhase::Cancelled && ev.delta.is_finite() {
                        let scale = ev.delta.exp().clamp(0.1, 10.0);
                        if (scale - 1.0).abs() > 1e-4 {
                            s.zoom_at(scale, pointer);
                            cx.notify();
                        }
                    }
                });
                cx.stop_propagation();
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for link in &paint_links {
                            let active = hover == Some(link.parent) || hover == Some(link.child);
                            let link_w = if active { stroke * 0.85 } else { hairline };
                            let mut path = PathBuilder::stroke(link_w);
                            path.move_to(at(origin, link.from));
                            path.cubic_bezier_to(
                                at(origin, link.to),
                                at(origin, link.ctrl1),
                                at(origin, link.ctrl2),
                            );
                            let color = if active {
                                tint(&colors, link.branch)
                            } else if hover.is_some() {
                                colors.border.opacity(0.45)
                            } else {
                                colors.border_strong.opacity(0.75)
                            };
                            finish(path, color, window);
                        }
                        for (ix, node) in paint_nodes.iter().enumerate() {
                            let lit = hover == Some(ix);
                            let ink = if node.depth == 0 {
                                colors.fg
                            } else {
                                tint(&colors, node.branch)
                            };
                            let r = Pixels::from(if lit { node.radius + 1.5 } else { node.radius });
                            let center = at(origin, (node.x, node.y));
                            let edge = if lit { colors.fg } else { ink };
                            if node.is_collapsed {
                                let outer_w = if lit {
                                    stroke
                                } else {
                                    stroke.max(hairline * 2.5)
                                };
                                ring(
                                    center,
                                    r + Pixels::from(0.75),
                                    (colors.bg, edge),
                                    outer_w,
                                    window,
                                );
                                ring(center, r * 0.48, (ink, ink), Pixels::ZERO, window);
                            } else {
                                let inside = if node.total_children == 0 {
                                    ink
                                } else {
                                    colors.bg
                                };
                                ring(
                                    center,
                                    r,
                                    (inside, edge),
                                    if lit { stroke * 0.85 } else { hairline * 1.5 },
                                    window,
                                );
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels);

        root.child(viewport)
            .children(tooltip)
            .children(reset_button)
            .child(measure(state, |s| &mut s.bounds))
    }
}
