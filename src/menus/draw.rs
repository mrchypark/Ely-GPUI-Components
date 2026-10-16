use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, Div, ElementId, Entity, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, Role, ScrollHandle, SharedString, Stateful, Styled, Window,
    accesskit::Action, canvas, div, prelude::*,
};

use super::{
    aim,
    menu::{Open, at_depth, choose},
    model::{Entry, Kind, Menu, MenuItem},
};
use crate::{
    forms::{Run, reveal, revealer, surface},
    i18n,
    navigation::marked,
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::KbdCombo,
};

fn row(item: &MenuItem, ix: usize, current: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    let fg = if item.disabled {
        colors.fg_disabled
    } else {
        colors.fg
    };
    let (lead, lead_color) = match &item.kind {
        Kind::Check(true) => (Some(IconName::Check), fg),
        Kind::Radio(true) => (Some(IconName::Dot), fg),
        _ => (item.icon, if item.disabled { fg } else { colors.fg_muted }),
    };
    div()
        .id(("row", ix))
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .h(theme.control_height(ControlSize::Md))
        .px_2()
        .rounded(theme.radius(Radius::Md))
        .text_color(fg)
        .when(current && !item.disabled, |row| row.bg(colors.hover))
        .when(!item.disabled, |row| row.cursor_pointer())
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(theme.icon_size(IconSize::Sm))
                .when_some(lead, |slot, icon| {
                    slot.child(Icon::new(icon).size(IconSize::Sm).color(lead_color))
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(if item.hits.is_empty() {
                    item.label.clone().into_any_element()
                } else {
                    marked(item.label.clone(), item.hits.clone(), cx)
                }),
        )
        .when_some(item.keys.clone(), |row, keys| {
            row.child(
                div()
                    .flex_none()
                    .pl_4()
                    .text_color(colors.fg_subtle)
                    .child(KbdCombo::new(&keys)),
            )
        })
        .when(matches!(item.kind, Kind::Sub(_)), |row| {
            row.child(
                Icon::new(IconName::ChevronRight)
                    .size(IconSize::Xs)
                    .color(colors.fg_subtle),
            )
        })
}

/// Button `column` of row `ix`, named by its label.
pub(super) fn button(
    item: &MenuItem,
    (ix, column): (usize, usize),
    current: bool,
    press: Run,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let colors = &theme.colors;
    let fg = match (item.disabled, current) {
        (true, _) => colors.fg_disabled,
        (false, true) => colors.fg,
        (false, false) => colors.fg_muted,
    };
    let (label, keys) = (item.label.clone(), item.keys.clone());
    let icon = item.icon.expect("a menu button has its icon");
    div()
        .id((SharedString::from(format!("button-{ix}")), column))
        .debug_selector(move || format!("menu-button-{ix}-{column}"))
        .role(Role::Button)
        .aria_label(item.label.clone())
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .h(theme.control_height(ControlSize::Md))
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(colors.border)
        .aria_selected(current)
        .when(current, |button| button.aria_active_descendant())
        .when(current && !item.disabled, |button| button.bg(colors.hover))
        .when(item.disabled, |button| {
            button.aria_description(i18n::text(cx, "state.unavailable", &[]))
        })
        .when(!item.disabled, |button| {
            let read = press.clone();
            button
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    press(window, cx);
                })
                .on_a11y_action(Action::Click, move |_, window, cx| read(window, cx))
        })
        .child(Icon::new(icon).size(IconSize::Sm).color(fg))
        .tooltip(Tooltip::rich(move |_, cx| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(label.clone())
                .when_some(keys.clone(), |tip, keys| {
                    tip.child(
                        div()
                            .text_color(cx.theme().colors.tooltip_fg.opacity(0.6))
                            .child(KbdCombo::new(&keys)),
                    )
                })
                .into_any_element()
        }))
}

pub(super) fn panel(
    id: &ElementId,
    menu: &Menu,
    state: &Entity<Open>,
    depth: usize,
    close: &Run,
    head: Option<AnyElement>,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let path: Vec<_> = state.read(cx).levels[..depth]
        .iter()
        .map(|level| (level.at, level.column, level.name.clone()))
        .collect();
    let scrolling = window.use_keyed_state((id.clone(), format!("scroll-{path:?}")), cx, |_, _| {
        (ScrollHandle::new(), None)
    });
    let at = state.read(cx).levels[depth].at;
    let reveal = reveal(
        &scrolling,
        |state| &mut state.1,
        at.is_some(),
        at.unwrap_or(0),
        cx,
    );
    let handle = scrolling.read(cx).0.clone();
    let open = state.read(cx);
    let here = at_depth(menu, &open.levels, depth).expect("settled levels follow submenu rows");
    let at = open.levels[depth].at;
    let theme = cx.theme();
    let colors = &theme.colors;
    let height = theme
        .list_max_height()
        .to_pixels(window.rem_size())
        .min((window.viewport_size().height - theme.window_margin() * 2.0).max(Pixels::ZERO));
    let usable = menu
        .usable_bounds
        .map(|bounds| super::placement::area(bounds, window.viewport_size()));
    let height = usable.map_or(height, |bounds| height.min(bounds.size.height));
    let rows = here
        .entries
        .iter()
        .enumerate()
        .map(|(ix, entry)| match entry {
            Entry::Separator => div().h_px().my_1().bg(colors.border).into_any_element(),
            Entry::Heading(title) => div()
                .px_2()
                .pt_1p5()
                .pb_1()
                .text_size(theme.text_size(TextSize::Xs))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.fg_subtle)
                .child(title.clone())
                .into_any_element(),
            Entry::Buttons(buttons) => div()
                .flex()
                .gap_1()
                .py_0p5()
                .children(buttons.iter().enumerate().map(|(column, item)| {
                    let current = at == Some(ix) && open.levels[depth].column == column;
                    let (hover, click, close, menu) =
                        (state.clone(), state.clone(), close.clone(), menu.clone());
                    let hover_menu = menu.clone();
                    let press: Run = Rc::new(move |window, cx| {
                        choose(&click, &menu, depth, (ix, column), &close, window, cx)
                    });
                    let drawn = button(item, (ix, column), current, press, cx);
                    let left = state.clone();
                    let here = (depth, (ix, column));
                    drawn.when(!item.disabled, |button| {
                        button
                            .on_mouse_move(move |event, _, cx| {
                                aim::hover(&hover, &hover_menu, here, false, event.position, cx)
                            })
                            .on_hover(move |hovered, _, cx| {
                                if !hovered {
                                    aim::leave(&left, here, cx);
                                }
                            })
                    })
                }))
                .into_any_element(),
            Entry::Item(item) => {
                let marked = at == Some(ix);
                let sub = matches!(item.kind, Kind::Sub(_));
                let (hover, click, close, menu) =
                    (state.clone(), state.clone(), close.clone(), menu.clone());
                let hover_menu = menu.clone();
                let label = item.label.clone();
                row(item, ix, marked, cx)
                    .debug_selector(move || format!("menu-row-{label}"))
                    .when(!item.disabled, |row| {
                        let left = state.clone();
                        row.on_mouse_move(move |event, _, cx| {
                            aim::hover(
                                &hover,
                                &hover_menu,
                                (depth, (ix, 0)),
                                sub,
                                event.position,
                                cx,
                            )
                        })
                        .on_hover(move |hovered, _, cx| {
                            if !hovered {
                                aim::leave(&left, (depth, (ix, 0)), cx);
                            }
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            move |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                choose(&click, &menu, depth, (ix, 0), &close, window, cx);
                            },
                        )
                    })
                    .when(marked && sub, |row| {
                        row.child(measure(state.clone(), move |open, bounds| {
                            let level = &mut open.levels[depth];
                            let changed = level.row != bounds;
                            level.row = bounds;
                            changed
                        }))
                    })
                    .into_any_element()
            }
        });
    surface((id.clone(), format!("panel-{depth}")), cx)
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .role(Role::Menu)
        .debug_selector(move || format!("menu-panel-{depth}"))
        .relative()
        // Rows under this panel take no hover.
        .occlude()
        // Presses here are the menu's, not its host's.
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .min_w(theme.menu_width())
        .when_some(usable, |panel, bounds| {
            let width = theme
                .menu_width()
                .to_pixels(window.rem_size())
                .min(bounds.size.width);
            panel.min_w(width).w(width).max_w(width).overflow_hidden()
        })
        .max_h(height)
        .p_1()
        .flex()
        .flex_col()
        .children(head.map(|head| div().flex_none().child(head)))
        .child(
            div()
                .id((id.clone(), format!("rows-{depth}")))
                .debug_selector(move || format!("menu-scroll-{depth}"))
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&handle)
                .flex()
                .flex_col()
                .children(rows.map(|row| div().flex_none().child(row)))
                .when_some(reveal, |rows, (ix, done)| {
                    rows.child(revealer(&handle, ix, done))
                })
                .on_scroll_wheel({
                    let state = state.clone();
                    move |_, _, cx| {
                        state.update(cx, |open, cx| {
                            open.pending = None;
                            if open.levels.len() > depth + 1 {
                                open.levels.truncate(depth + 1);
                                cx.notify();
                            }
                        });
                    }
                }),
        )
        .child(measure(state.clone(), move |open, bounds| {
            let level = &mut open.levels[depth];
            let changed = level.panel != bounds;
            level.panel = bounds;
            changed
        }))
}

/// Records the host's, a marked row's or a panel's box.
pub(super) fn measure(
    state: Entity<Open>,
    write: impl Fn(&mut Open, Bounds<Pixels>) -> bool + 'static,
) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            if state.update(cx, |open, _| write(open, bounds)) {
                state.update(cx, |_, cx| cx.notify());
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}
