use super::View;
use crate::{buttons::Button, tables::Column, theme::ActiveTheme, typography::Ellipsis};
use gpui::{
    App, Bounds, Div, ElementId, Entity, ParentElement, Pixels, SharedString, Styled, Window, div,
    prelude::*,
};

pub(super) fn measure(
    view: &Entity<View>,
    keys: &[SharedString],
    bounds: Vec<Bounds<Pixels>>,
    window: &mut Window,
    cx: &mut App,
) {
    if !cx.theme().touch_mode {
        return;
    }
    let start = bounds.len().saturating_sub(keys.len());
    let changed = view.update(cx, |view, cx| {
        let mut changed = false;
        for (key, bounds) in keys.iter().zip(bounds.iter().skip(start)) {
            if view.measured_widths.get(key) != Some(&bounds.size.width) {
                view.measured_widths.insert(key.clone(), bounds.size.width);
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
        changed
    });
    if changed {
        window.request_animation_frame();
    }
}

pub(super) fn controls(
    id: &ElementId,
    view: &Entity<View>,
    columns: &[Column],
    shown: &[usize],
    cx: &mut App,
) -> Option<Div> {
    if !cx.theme().touch_mode {
        return None;
    }
    let visible: Vec<_> = shown
        .iter()
        .map(|ix| {
            (
                columns[*ix].key.clone(),
                columns[*ix].title.clone(),
                columns[*ix].pinned,
            )
        })
        .collect();
    view.update(cx, |view, _| {
        if view
            .column_target
            .as_ref()
            .is_some_and(|key| !visible.iter().any(|col| col.0 == *key))
        {
            view.column_target = None;
        }
    });
    let state = view.read(cx);
    let open = state.column_controls;
    let at = state
        .column_target
        .as_ref()
        .and_then(|key| visible.iter().position(|col| col.0 == *key));
    let target = at.map(|at| visible[at].0.clone());
    let neighbor = |right: bool| {
        at.and_then(|at| {
            let same: Vec<_> = visible
                .iter()
                .filter(|col| col.2 == visible[at].2)
                .map(|col| col.0.clone())
                .collect();
            let index = same.iter().position(|key| *key == visible[at].0)?;
            let next = if right {
                index.checked_add(1)?
            } else {
                index.checked_sub(1)?
            };
            same.get(next).cloned()
        })
    };
    let left = neighbor(false);
    let right = neighbor(true);
    let width = target
        .as_ref()
        .and_then(|key| state.measured_widths.get(key))
        .copied();
    let minimum = state.narrowest;
    let reset = target
        .as_ref()
        .is_some_and(|key| state.widths.contains_key(key));
    let step = cx.theme().touch_spacing() * 3.;
    let owner = view.clone();
    let first = visible.first().map(|col| col.0.clone());
    let mut strip = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap_1()
        .debug_selector(|| "table-column-tools".into())
        .child(
            div().child(
                Button::new((id.clone(), "columns"), "Adjust columns")
                    .debug_selector(|| "table-columns-toggle".into())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        owner.update(cx, |view, cx| {
                            view.column_controls = !view.column_controls;
                            if view.column_controls && view.column_target.is_none() {
                                view.column_target = first.clone();
                            }
                            cx.notify();
                        });
                    }),
            ),
        );
    if !open {
        return Some(strip);
    }
    let mut chooser = div().flex().items_center().gap_1();
    for (name, forward) in [("Previous", false), ("Next", true)] {
        let keys: Vec<_> = visible.iter().map(|col| col.0.clone()).collect();
        let owner = view.clone();
        chooser = chooser.child(
            div().child(
                Button::new((id.clone(), name), name)
                    .debug_selector(move || format!("table-column-{name}"))
                    .disabled(keys.is_empty())
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        owner.update(cx, |view, cx| {
                            let at = view
                                .column_target
                                .as_ref()
                                .and_then(|key| keys.iter().position(|known| known == key));
                            let next = match (at, forward) {
                                (Some(at), true) => (at + 1) % keys.len(),
                                (Some(at), false) => (at + keys.len() - 1) % keys.len(),
                                (None, true) => 0,
                                (None, false) => keys.len() - 1,
                            };
                            view.column_target = Some(keys[next].clone());
                            cx.notify();
                        });
                    }),
            ),
        );
    }
    chooser = chooser.child(
        div().flex_1().min_w_0().child(Ellipsis::new(
            at.map(|at| visible[at].1.clone())
                .unwrap_or_else(|| "Choose a column".into()),
        )),
    );
    let mut actions = div().flex().flex_wrap().gap_1();
    for (name, disabled) in [
        ("Move left", left.is_none()),
        ("Move right", right.is_none()),
        ("Narrower", width.is_none_or(|w| w <= minimum)),
        ("Wider", width.is_none()),
        ("Reset width", !reset),
    ] {
        let owner = view.clone();
        let key = target.clone();
        let neighbor = if name == "Move left" {
            left.clone()
        } else {
            right.clone()
        };
        actions = actions.child(
            div().child(
                Button::new((id.clone(), name), name)
                    .debug_selector(move || format!("table-column-{name}"))
                    .disabled(disabled)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let Some(key) = &key else {
                            return;
                        };
                        owner.update(cx, |view, cx| {
                            if view.column_target.as_ref() != Some(key) {
                                return;
                            }
                            match name {
                                "Move left" | "Move right" => {
                                    if let Some(other) = &neighbor
                                        && let (Some(a), Some(b)) = (
                                            view.order.iter().position(|k| k == key),
                                            view.order.iter().position(|k| k == other),
                                        )
                                    {
                                        view.order.swap(a, b);
                                    }
                                }
                                "Reset width" => {
                                    view.widths.remove(key);
                                }
                                _ => {
                                    if let Some(wide) = view.measured_widths.get(key).copied() {
                                        let next = if name == "Narrower" {
                                            wide - step
                                        } else {
                                            wide + step
                                        };
                                        view.widths.insert(key.clone(), next.max(view.narrowest));
                                    }
                                }
                            }
                            cx.notify();
                        });
                    }),
            ),
        );
    }
    strip = strip.child(chooser).child(actions);
    Some(strip)
}
