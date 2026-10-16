use gpui::{
    AnyElement, App, Div, ElementId, Entity, FontWeight, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, ScrollHandle, Stateful, Styled, Window, div, prelude::*,
};

use super::{
    menu::{Open, at_depth, choose, mark, measure},
    model::{Entry, Kind, Menu, MenuItem},
};
use crate::{
    forms::{Run, reveal, revealer, surface},
    navigation::marked,
    primitives::{Icon, IconName},
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
        .map(|level| level.at)
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
                        row.on_mouse_move(move |_, _, cx| {
                            let open = hover.read(cx);
                            let Some(level) = open.levels.get(depth) else {
                                return;
                            };
                            let shown = open.levels.len() > depth + 1;
                            if level.at != Some(ix) || (sub && !shown) {
                                mark(&hover, &hover_menu, depth, ix, true, cx);
                            }
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            move |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                choose(&click, &menu, depth, ix, &close, window, cx);
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
        .debug_selector(move || format!("menu-panel-{depth}"))
        .relative()
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
