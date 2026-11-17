use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, AppContext as _, DispatchPhase, ElementId, EmptyView, EntityId,
    InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TouchDragEvent, TouchPhase, Window, canvas, div,
};

use super::Flip;

/// A row on its way, by key, and whose list it belongs to.
struct RowDrag {
    owner: EntityId,
    key: SharedString,
}

/// The row held, by key, where it would land, its top, and where the pointer took it.
#[derive(Clone)]
struct Hold {
    key: SharedString,
    target: usize,
    top: Pixels,
    grab: Pixels,
    touch: bool,
    origin: Pixels,
}

type OnReorder = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// Rows reordered by dragging: the held row follows the pointer, the others glide aside, and on drop `on_reorder(from, to)` asks the owner to move row `from` to place `to`.
#[derive(IntoElement)]
pub struct Reorder {
    id: ElementId,
    rows: Vec<(SharedString, AnyElement)>,
    on_reorder: Option<OnReorder>,
    touch_grip: Option<Pixels>,
}

impl Reorder {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            rows: Vec::new(),
            on_reorder: None,
            touch_grip: None,
        }
    }

    pub fn row(mut self, key: impl Into<SharedString>, row: impl IntoElement) -> Self {
        self.rows.push((key.into(), row.into_any_element()));
        self
    }

    pub fn on_reorder(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reorder = Some(Rc::new(handler));
        self
    }

    /// Claim touch drags that start within this width at the leading edge of a row.
    pub fn touch_grip(mut self, width: Pixels) -> Self {
        self.touch_grip = Some(width);
        self
    }
}

/// Where the held row lands: past every other row whose middle is above its own.
fn landing(heights: &[Pixels], from: usize, top: Pixels) -> usize {
    let middle = top + heights[from] / 2.0;
    let mut y = Pixels::ZERO;
    let mut target = 0;
    let others = heights
        .iter()
        .enumerate()
        .filter(|(ix, _)| *ix != from)
        .map(|(_, height)| height);
    for height in others {
        if y + *height / 2.0 < middle {
            target += 1;
        }
        y += *height;
    }
    target
}

impl RenderOnce for Reorder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hold = window.use_keyed_state((self.id.clone(), "hold"), cx, |_, _| None::<Hold>);
        let sizes = window.use_keyed_state((self.id.clone(), "heights"), cx, |_, _| {
            HashMap::<SharedString, Pixels>::new()
        });
        let owner = hold.entity_id();
        if hold.read(cx).as_ref().is_some_and(|held| !held.touch) && !cx.has_active_drag() {
            log::debug!("reorder {:?}: the drag ended elsewhere", self.id);
            hold.update(cx, |hold, _| *hold = None);
        }
        let keys: Vec<SharedString> = self.rows.iter().map(|(key, _)| key.clone()).collect();
        let held = hold.read(cx).clone().and_then(|held| {
            let from = keys.iter().position(|key| *key == held.key);
            if from.is_none() {
                log::info!("reorder: the held row {} left the list", held.key);
            }
            from.map(|from| (from, held.target.min(keys.len() - 1), held.top))
        });
        if held.is_none() && hold.read(cx).is_some() {
            hold.update(cx, |hold, _| *hold = None);
        }
        let heights: Vec<Pixels> = keys
            .iter()
            .map(|key| sizes.read(cx).get(key).copied().unwrap_or_default())
            .collect();
        let mut order: Vec<usize> = (0..keys.len()).collect();
        if let Some((from, target, _)) = held {
            let row = order.remove(from);
            order.insert(target, row);
        }
        let mut rows: Vec<Option<AnyElement>> =
            self.rows.into_iter().map(|(_, row)| Some(row)).collect();
        let mut flip = Flip::new((self.id.clone(), "flip"));
        for ix in order {
            let key = keys[ix].clone();
            let (grab, measured) = (hold.clone(), sizes.clone());
            let (touch_hold, touch_keys, touch_heights, touch_callback) = (
                hold.clone(),
                keys.clone(),
                heights.clone(),
                self.on_reorder.clone(),
            );
            let (touch_key, touch_grip) = (key.clone(), self.touch_grip);
            let row = div()
                .id((self.id.clone(), key.clone()))
                .relative()
                .child(rows[ix].take().expect("each row once"))
                .child(
                    canvas(
                        {
                            let key = key.clone();
                            move |bounds, window, cx| {
                                if measured.read(cx).get(&key) != Some(&bounds.size.height) {
                                    measured.update(cx, |sizes, cx| {
                                        sizes.insert(key.clone(), bounds.size.height);
                                        cx.notify();
                                    });
                                    window.request_animation_frame();
                                }
                            }
                        },
                        move |bounds, _, window, _| {
                            let Some(width) = touch_grip else { return };
                            let (hold, keys, heights, callback, key) = (
                                touch_hold.clone(),
                                touch_keys.clone(),
                                touch_heights.clone(),
                                touch_callback.clone(),
                                touch_key.clone(),
                            );
                            window.on_mouse_event(
                                move |event: &TouchDragEvent, phase, window, cx| {
                                    if phase != DispatchPhase::Bubble {
                                        return;
                                    }
                                    if event.phase == TouchPhase::Started {
                                        if window.default_prevented()
                                            || callback.is_none()
                                            || !bounds.contains(&event.start_position)
                                            || event.start_position.x >= bounds.left() + width
                                            || heights.iter().any(|height| *height <= Pixels::ZERO)
                                        {
                                            return;
                                        }
                                        let top = heights
                                            .iter()
                                            .take(ix)
                                            .copied()
                                            .fold(Pixels::ZERO, |a, b| a + b);
                                        hold.update(cx, |held, cx| {
                                            *held = Some(Hold {
                                                key: key.clone(),
                                                target: ix,
                                                top,
                                                grab: event.start_position.y - bounds.top(),
                                                touch: true,
                                                origin: bounds.top() - top,
                                            });
                                            cx.notify();
                                        });
                                        window.prevent_default();
                                        cx.stop_propagation();
                                        return;
                                    }
                                    if !hold
                                        .read(cx)
                                        .as_ref()
                                        .is_some_and(|held| held.touch && held.key == key)
                                    {
                                        return;
                                    }
                                    if event.phase == TouchPhase::Cancelled {
                                        hold.update(cx, |held, cx| {
                                            *held = None;
                                            cx.notify();
                                        });
                                        return;
                                    }
                                    let result = hold.update(cx, |slot, cx| {
                                        let held = slot.as_mut()?;
                                        let from = keys.iter().position(|key| *key == held.key)?;
                                        let total = heights
                                            .iter()
                                            .copied()
                                            .fold(Pixels::ZERO, |a, b| a + b);
                                        held.top = (event.position.y - held.origin - held.grab)
                                            .clamp(
                                                Pixels::ZERO,
                                                (total - heights[from]).max(Pixels::ZERO),
                                            );
                                        held.target = landing(&heights, from, held.top);
                                        let result = (from, held.target);
                                        if event.phase == TouchPhase::Ended {
                                            *slot = None;
                                        }
                                        cx.notify();
                                        Some(result)
                                    });
                                    if event.phase == TouchPhase::Ended
                                        && let (Some((from, to)), Some(callback)) =
                                            (result, &callback)
                                        && from != to
                                    {
                                        callback(from, to, window, cx);
                                    }
                                },
                            );
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
                .on_drag(
                    RowDrag {
                        owner,
                        key: key.clone(),
                    },
                    move |drag, offset, _, cx| {
                        log::info!("reorder: row {} lifted", drag.key);
                        grab.update(cx, |hold, cx| {
                            *hold = Some(Hold {
                                key: drag.key.clone(),
                                target: ix,
                                top: Pixels::ZERO,
                                grab: offset.y,
                                touch: false,
                                origin: Pixels::ZERO,
                            });
                            cx.notify();
                        });
                        cx.new(|_| EmptyView)
                    },
                );
            flip = flip.row(key, row);
        }
        if let Some((from, _, top)) = held {
            flip = flip.lifted(keys[from].clone(), top);
        }
        let total = heights
            .iter()
            .fold(Pixels::ZERO, |total, height| total + *height);
        let (moves, drops, on_reorder) = (hold.clone(), hold, self.on_reorder);
        let dropped = keys.clone();
        div()
            .id(self.id)
            .child(flip)
            .on_drag_move::<RowDrag>(move |event, _, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let pointer = event.event.position.y - event.bounds.top();
                moves.update(cx, |hold, cx| {
                    let Some(hold) = hold else { return };
                    let Some(from) = keys.iter().position(|key| *key == hold.key) else {
                        return;
                    };
                    let reach = total - heights[from];
                    hold.top = (pointer - hold.grab).clamp(Pixels::ZERO, reach.max(Pixels::ZERO));
                    hold.target = landing(&heights, from, hold.top);
                    cx.notify();
                });
            })
            .on_drop(move |drag: &RowDrag, window, cx| {
                if drag.owner != owner {
                    return;
                }
                let Some(held) = drops.update(cx, |hold, cx| {
                    cx.notify();
                    hold.take()
                }) else {
                    return;
                };
                let Some(from) = dropped.iter().position(|key| *key == held.key) else {
                    return;
                };
                let target = held.target.min(dropped.len() - 1);
                if from != target {
                    log::info!("reorder: row {from} to {target}");
                    if let Some(on_reorder) = &on_reorder {
                        on_reorder(from, target, window, cx);
                    }
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::landing;

    #[test]
    fn a_held_row_lands_past_the_middles_above_it() {
        let heights = [px(20.0); 5];
        assert_eq!(landing(&heights, 0, px(40.0)), 2);
        assert_eq!(landing(&heights, 0, px(0.0)), 0);
        assert_eq!(landing(&heights, 4, px(0.0)), 0);
        assert_eq!(landing(&heights, 2, px(70.0)), 4);
    }
}

#[cfg(all(test, feature = "test-support"))]
mod touch_tests {
    use super::*;
    use gpui::{
        Context, Render, ScrollHandle, TestAppContext, TouchEvent, TouchId, VisualTestContext,
        point, px,
    };

    struct Host {
        moves: Vec<(usize, usize)>,
        scroll: ScrollHandle,
    }
    impl Render for Host {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let owner = cx.entity();
            let list = (0..3).fold(
                Reorder::new("touch-rows").touch_grip(px(48.)).on_reorder(
                    move |from, to, _, cx| {
                        owner.update(cx, |host, _| host.moves.push((from, to)));
                    },
                ),
                |list, row| {
                    list.row(
                        format!("{row}"),
                        div()
                            .debug_selector(move || format!("touch-row-{row}"))
                            .w(px(240.))
                            .h(px(60.)),
                    )
                },
            );
            div()
                .id("touch-parent")
                .w(px(240.))
                .h(px(180.))
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .child(list)
                .child(div().h(px(800.)))
        }
    }

    fn send(cx: &mut VisualTestContext, phase: TouchPhase, at: gpui::Point<Pixels>) {
        cx.simulate_event(TouchEvent {
            id: TouchId(1),
            phase,
            position: at,
            predicted_position: None,
            force: None,
        });
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
        }
    }

    #[gpui::test]
    fn grip_drag_reorders_once_cancel_and_outside_scroll(cx: &mut TestAppContext) {
        cx.update(crate::init_for_tests);
        cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.reduced_motion = true));
        let (view, cx) = cx.add_window_view(|_, _| Host {
            moves: Vec::new(),
            scroll: ScrollHandle::new(),
        });
        for _ in 0..3 {
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let row = cx.debug_bounds("touch-row-0").unwrap();
        assert!(cx.debug_bounds("touch-row-1").unwrap().top() >= row.bottom());
        let grip = row.origin + point(px(20.), px(30.));
        let down = grip + point(px(0.), px(120.));
        send(cx, TouchPhase::Started, grip);
        send(cx, TouchPhase::Moved, down);
        send(cx, TouchPhase::Ended, down);
        assert_eq!(
            view.read_with(cx, |view, _| view.moves.clone()),
            vec![(0, 2)]
        );
        assert_eq!(view.read_with(cx, |view, _| view.scroll.offset().y), px(0.));
        send(cx, TouchPhase::Started, grip);
        send(cx, TouchPhase::Moved, down);
        send(cx, TouchPhase::Cancelled, down);
        assert_eq!(view.read_with(cx, |view, _| view.moves.len()), 1);
        let outside = row.origin + point(px(120.), px(140.));
        send(cx, TouchPhase::Started, outside);
        send(cx, TouchPhase::Moved, outside - point(px(0.), px(100.)));
        send(cx, TouchPhase::Ended, outside - point(px(0.), px(100.)));
        assert!(view.read_with(cx, |view, _| view.scroll.offset().y) < px(0.));
        assert_eq!(view.read_with(cx, |view, _| view.moves.len()), 1);
    }
}
