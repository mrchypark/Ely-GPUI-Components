use super::*;

#[derive(Default)]
pub(super) struct ParSetsState {
    pub(super) bounds: Bounds<Pixels>,
    pub(super) hover: Option<usize>,
    pub(super) pinned: Option<usize>,
    pub(super) hidden: HashSet<SharedString>,
    pub(super) press_pos: Option<(f32, f32)>,
}

impl Styled for ParallelSetsChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ParallelSetsChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state: Entity<ParSetsState> =
            window.use_keyed_state((self.id.clone(), "parsets"), cx, |_, _| {
                ParSetsState::default()
            });
        let (bounds, raw_hover, raw_pinned, hidden) = {
            let st = state.read(cx);
            (st.bounds, st.hover, st.pinned, st.hidden.clone())
        };
        let mut first_cats: Vec<SharedString> = Vec::new();
        for (path, _) in &self.ribbons {
            if let Some(cat) = path.first()
                && !first_cats.contains(cat)
            {
                first_cats.push(cat.clone());
            }
        }
        let first_color_map: HashMap<SharedString, usize> = first_cats
            .iter()
            .enumerate()
            .map(|(ix, cat)| (cat.clone(), ix))
            .collect();
        let orig_count = self.ribbons.len();
        let mut visible_ribbons: Vec<(Vec<SharedString>, f64)> = Vec::new();
        let mut vis_to_orig: Vec<usize> = Vec::new();
        let mut orig_to_vis: Vec<Option<usize>> = vec![None; orig_count];
        let mut vis_color_ix: Vec<usize> = Vec::new();
        for (orig_ix, (path, count)) in self.ribbons.iter().enumerate() {
            if path.first().is_some_and(|cat| hidden.contains(cat)) {
                continue;
            }
            orig_to_vis[orig_ix] = Some(visible_ribbons.len());
            vis_to_orig.push(orig_ix);
            vis_color_ix.push(
                path.first()
                    .and_then(|cat| first_color_map.get(cat).copied())
                    .unwrap_or(0),
            );
            visible_ribbons.push((path.clone(), *count));
        }

        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let box_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            box_size,
            pixels(sizes.inset),
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let bar_w = (pixels(sizes.inset) * 1.4).max(8.0);
        let layout = compute_parallel_sets(&self.dimensions, &visible_ribbons, rect, bar_w);
        let total_targets = orig_count + layout.bars.len();
        let active_pinned = raw_pinned.filter(|&ix| ix < orig_count && orig_to_vis[ix].is_some());
        let hover = raw_hover
            .filter(|&ix| ix < total_targets && (ix >= orig_count || orig_to_vis[ix].is_some()));
        let active = active_pinned.or(hover);
        let format = self.format.clone();

        let legend = (first_cats.len() > 1).then(|| {
            let toggled = state.clone();
            let legend = first_cats.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "legend")),
                |legend, (ix, name)| {
                    legend.entry(tint(&colors, ix), name.clone(), hidden.contains(name))
                },
            );
            legend.on_toggle(move |name, _, cx| {
                toggled.update(cx, |st, cx| {
                    if !st.hidden.remove(name) {
                        st.hidden.insert(name.clone());
                    }
                    st.pinned = None;
                    st.hover = None;
                    cx.notify();
                });
            })
        });

        let active_ribbon_vis =
            active.and_then(|ix| (ix < orig_count).then(|| orig_to_vis[ix]).flatten());
        let active_bar = active.and_then(|ix| ix.checked_sub(orig_count));

        let tooltip = active.map(|ix| {
            if ix < orig_count {
                let vis_ix = orig_to_vis[ix].expect("visible ribbon");
                let (path, count) = &self.ribbons[ix];
                let title = path
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .join(" › ");
                let share = if layout.total > 0.0 {
                    *count / layout.total
                } else {
                    0.0
                };
                let color_ix = vis_color_ix.get(vis_ix).copied().unwrap_or(0);
                let card = ChartTooltip::new(title)
                    .row(Some(tint(&colors, color_ix)), "Count", format(*count))
                    .row(None, "Share", fmt::percent(share, 1, false));
                let pos = layout.ribbon_anchors[vis_ix];
                anchored(pos, pos.0 > box_size.0 * 0.55, card)
            } else {
                let bar = &layout.bars[ix - orig_count];
                let share = if layout.total > 0.0 {
                    bar.count / layout.total
                } else {
                    0.0
                };
                let dot = (bar.dim == 0).then(|| {
                    let c_ix = first_color_map
                        .get(&bar.name)
                        .copied()
                        .unwrap_or(bar.cat_ix);
                    tint(&colors, c_ix)
                });
                let card = ChartTooltip::new(format!("{}: {}", self.dimensions[bar.dim], bar.name))
                    .row(dot, "Count", format(bar.count))
                    .row(None, "Share", fmt::percent(share, 1, false));
                let pos = (bar.rect.x + bar.rect.w, bar.rect.y + bar.rect.h * 0.5);
                anchored(pos, pos.0 > box_size.0 * 0.55, card)
            }
        });

        let dim_labels: Vec<Div> = layout
            .dim_xs
            .iter()
            .enumerate()
            .map(|(d, &x)| {
                below(
                    x,
                    rect,
                    div()
                        .flex_none()
                        .whitespace_nowrap()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .child(self.dimensions[d].clone()),
                )
            })
            .collect();

        let d_count = self.dimensions.len();
        let cat_labels: Vec<Div> = layout
            .bars
            .iter()
            .filter(|bar| bar.rect.h >= 12.0)
            .map(|bar| {
                let is_last = d_count > 1 && bar.dim == d_count - 1;
                let anchor_x = if is_last {
                    bar.rect.x - 4.0
                } else {
                    bar.rect.x + bar.rect.w + 4.0
                };
                div()
                    .absolute()
                    .left(Pixels::from(anchor_x))
                    .top(Pixels::from(bar.rect.y + bar.rect.h * 0.5))
                    .w_0()
                    .h_0()
                    .flex()
                    .items_center()
                    .when(is_last, |el| el.justify_end())
                    .child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .px_1()
                            .rounded(theme.radius(Radius::Sm))
                            .bg(colors.bg.opacity(0.82))
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg)
                            .child(bar.name.clone()),
                    )
            })
            .collect();

        let hit_bars: Vec<Rect> = layout.bars.iter().map(|b| b.rect).collect();
        let hit_segs = layout.segments.clone();
        let hit = Rc::new(move |pos: (f32, f32)| -> Option<usize> {
            if let Some(bar_ix) = hit_bars.iter().position(|r| r.contains(pos)) {
                return Some(orig_count + bar_ix);
            }
            hit_segs
                .iter()
                .find(|seg| seg.holds(pos))
                .and_then(|seg| vis_to_orig.get(seg.ribbon_ix).copied())
        });
        let paint_bars = layout.bars;
        let paint_segs = layout.segments;
        let corner = theme.radius(Radius::Sm).to_pixels(rem);
        let (moved, pressed, released, released_out, left) = (
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
            state.clone(),
        );
        let (move_hit, press_hit, release_hit) = (hit.clone(), hit.clone(), hit);
        let has_active = active.is_some();
        let clear_btn = active_pinned.map(|_| {
            let reset = state.clone();
            let button = Button::new((self.id.clone(), "clear"), "Clear")
                .debug_selector(|| "parallel-sets-clear".into())
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    reset.update(cx, |st, cx| {
                        st.pinned = None;
                        st.hover = None;
                        st.press_pos = None;
                        cx.notify();
                    });
                });
            div()
                .absolute()
                .top_0()
                .right_0()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(button)
        });

        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());

        let plot = div()
            .id((self.id.clone(), "parsets"))
            .debug_selector(|| "parallel-sets-plot".into())
            .relative()
            .flex_1()
            .min_h_0()
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let pos = local_pos(event.position, moved.read(cx).bounds);
                let next = move_hit(pos);
                moved.update(cx, |st, cx| {
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let pos = local_pos(event.position, pressed.read(cx).bounds);
                let next = press_hit(pos);
                pressed.update(cx, |st, cx| {
                    st.press_pos = Some(pos);
                    if st.hover != next {
                        st.hover = next;
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |event: &MouseUpEvent, _, cx| {
                let pos = local_pos(event.position, released.read(cx).bounds);
                released.update(cx, |st, cx| {
                    let clicked = st
                        .press_pos
                        .take()
                        .is_some_and(|p| (pos.0 - p.0).hypot(pos.1 - p.1) <= 4.0);
                    if !clicked {
                        return;
                    }
                    let target = release_hit(pos);
                    st.hover = target;
                    if let Some(rix) = target.filter(|&ix| ix < orig_count) {
                        st.pinned = if st.pinned == Some(rix) {
                            None
                        } else {
                            Some(rix)
                        };
                    } else {
                        st.pinned = None;
                    }
                    cx.notify();
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released_out.update(cx, |st, _| {
                    st.press_pos = None;
                });
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |st, cx| {
                        if st.hover.take().is_some() {
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
                        let active_bar_ref = active_bar
                            .and_then(|ix| paint_bars.get(ix))
                            .map(|b| (b.dim, b.cat_ix));
                        for seg in &paint_segs {
                            let active = match (active_ribbon_vis, active_bar_ref) {
                                (Some(rix), _) => seg.ribbon_ix == rix,
                                (None, Some((dim, cat))) => {
                                    (seg.dim == dim && seg.from_cat == cat)
                                        || (seg.dim + 1 == dim && seg.to_cat == cat)
                                }
                                (None, None) => true,
                            };
                            let alpha = if !has_active {
                                0.4
                            } else if active {
                                0.74
                            } else {
                                0.1
                            };
                            let mut path = PathBuilder::fill();
                            path.move_to(at(origin, seg.from));
                            let mid = (seg.from.0 + seg.to.0) * 0.5;
                            path.cubic_bezier_to(
                                at(origin, seg.to),
                                at(origin, (mid, seg.from.1)),
                                at(origin, (mid, seg.to.1)),
                            );
                            path.line_to(at(origin, (seg.to.0, seg.to.1 + seg.thick)));
                            path.cubic_bezier_to(
                                at(origin, (seg.from.0, seg.from.1 + seg.thick)),
                                at(origin, (mid, seg.to.1 + seg.thick)),
                                at(origin, (mid, seg.from.1 + seg.thick)),
                            );
                            path.close();
                            let color_ix = vis_color_ix
                                .get(seg.ribbon_ix)
                                .copied()
                                .unwrap_or(seg.color_ix);
                            finish(path, tint(&colors, color_ix).opacity(alpha), window);
                        }
                        for (ix, bar) in paint_bars.iter().enumerate() {
                            let lit = active_bar == Some(ix);
                            let color = if bar.dim == 0 {
                                let c_ix = first_color_map
                                    .get(&bar.name)
                                    .copied()
                                    .unwrap_or(bar.cat_ix);
                                tint(&colors, c_ix).opacity(if lit { 1.0 } else { 0.88 })
                            } else {
                                colors.fg_muted.opacity(if lit { 0.8 } else { 0.45 })
                            };
                            window.paint_quad(
                                fill(place(origin, bar.rect), color).corner_radii(corner),
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(dim_labels)
            .children(cat_labels)
            .children(clear_btn)
            .children(tooltip)
            .child(measure(state, |st| &mut st.bounds));

        root.flex().flex_col().gap_2().children(legend).child(plot)
    }
}
