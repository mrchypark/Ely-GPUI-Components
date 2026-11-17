use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, ScrollHandle, Styled, Window, div,
};

use crate::{
    motion::Spinner,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};

pub(crate) type OnRefresh = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Default)]
pub(crate) struct PullState {
    pub(crate) pull: Pixels,
    dragging: bool,
    drag_start_y: Pixels,
    armed: bool,
}

fn threshold(window: &Window, cx: &App) -> Pixels {
    cx.theme()
        .control_height(ControlSize::Lg)
        .to_pixels(window.rem_size())
}

pub(crate) fn on_scroll_pull(
    state: &Entity<PullState>,
    handle: &ScrollHandle,
    delta_y: Pixels,
    refreshing: bool,
    on_refresh: &OnRefresh,
    window: &mut Window,
    cx: &mut App,
) {
    if refreshing {
        state.update(cx, |pull, _| {
            pull.pull = Pixels::ZERO;
            pull.armed = false;
        });
        return;
    }
    if handle.offset().y >= Pixels::ZERO && delta_y > Pixels::ZERO {
        let limit = threshold(window, cx);
        let fire = state.update(cx, |pull, cx| {
            pull.pull += delta_y;
            let ready = pull.pull >= limit && !pull.armed;
            if ready {
                pull.armed = true;
                pull.pull = Pixels::ZERO;
            }
            cx.notify();
            ready
        });
        if fire {
            on_refresh(window, cx);
        }
    } else if state.read(cx).pull != Pixels::ZERO || state.read(cx).armed {
        state.update(cx, |pull, cx| {
            pull.pull = Pixels::ZERO;
            pull.armed = false;
            cx.notify();
        });
    }
}

pub(crate) fn on_pointer_down(
    state: &Entity<PullState>,
    handle: &ScrollHandle,
    pos_y: Pixels,
    refreshing: bool,
    cx: &mut App,
) {
    if refreshing || handle.offset().y < Pixels::ZERO {
        return;
    }
    state.update(cx, |pull, _| {
        pull.dragging = true;
        pull.drag_start_y = pos_y;
        pull.pull = Pixels::ZERO;
        pull.armed = false;
    });
}

pub(crate) fn on_pointer_move(
    state: &Entity<PullState>,
    handle: &ScrollHandle,
    pos_y: Pixels,
    refreshing: bool,
    cx: &mut App,
) {
    if !state.read(cx).dragging || refreshing || handle.offset().y < Pixels::ZERO {
        return;
    }
    state.update(cx, |pull, cx| {
        pull.pull = (pos_y - pull.drag_start_y).max(Pixels::ZERO);
        cx.notify();
    });
}

pub(crate) fn on_pointer_up(
    state: &Entity<PullState>,
    refreshing: bool,
    on_refresh: &OnRefresh,
    window: &mut Window,
    cx: &mut App,
) {
    if !state.read(cx).dragging {
        return;
    }
    let limit = threshold(window, cx);
    let fire = state.update(cx, |pull, cx| {
        pull.dragging = false;
        let ready = !refreshing && pull.pull >= limit;
        pull.pull = Pixels::ZERO;
        cx.notify();
        ready
    });
    if fire {
        on_refresh(window, cx);
    }
}

pub(crate) fn attach_pointer_pull(
    area: gpui::Stateful<Div>,
    pull_state: Entity<PullState>,
    handle: ScrollHandle,
    refreshing: bool,
    on_refresh: OnRefresh,
) -> gpui::Stateful<Div> {
    let (down_handle, move_handle) = (handle.clone(), handle);
    let (down_pull, move_pull, up_pull) = (pull_state.clone(), pull_state.clone(), pull_state);
    area.on_mouse_down(MouseButton::Left, move |event, _, cx| {
        on_pointer_down(&down_pull, &down_handle, event.position.y, refreshing, cx);
    })
    .on_mouse_move(move |event, _, cx| {
        on_pointer_move(&move_pull, &move_handle, event.position.y, refreshing, cx);
    })
    .on_mouse_up(MouseButton::Left, move |_, window, cx| {
        on_pointer_up(&up_pull, refreshing, &on_refresh, window, cx);
    })
}

pub(crate) fn refresh_banner(
    id: ElementId,
    refreshing: bool,
    pull: Pixels,
    window: &Window,
    cx: &App,
) -> Option<AnyElement> {
    if !refreshing && pull <= Pixels::ZERO {
        return None;
    }
    let theme = cx.theme();
    let colors = &theme.colors;
    let height = theme
        .control_height(ControlSize::Sm)
        .to_pixels(window.rem_size());
    let mark: AnyElement = if refreshing {
        Spinner::new((id, "refresh-spinner"))
            .size(IconSize::Sm)
            .into_any_element()
    } else {
        Icon::new(IconName::RefreshCw)
            .size(IconSize::Sm)
            .color(colors.fg_muted)
            .into_any_element()
    };
    Some(
        div()
            .debug_selector(|| "scroll-refresh-indicator".into())
            .w_full()
            .h(if refreshing { height } else { pull.min(height) })
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .child(mark)
            .into_any_element(),
    )
}
