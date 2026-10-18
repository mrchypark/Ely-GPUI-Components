use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyWindowHandle, App, Bounds, ElementId, Entity, InteractiveElement,
    IntoElement, ParentElement, Pixels, RenderOnce, ScrollHandle, StatefulInteractiveElement,
    Styled, WeakEntity, Window, div, prelude::*,
};

use super::WindowManager;
use crate::{
    buttons::IconButton,
    motion,
    overlays::{tooltip_available, tooltip_has_room},
    primitives::{
        Backdrop, FocusNext, FocusPrev, Icon, IconName, Place, Takeover, give_back, take_focus,
    },
    theme::{ActiveTheme, ControlSize, Elevation, IconSize, Radius, TextSize},
    typography::Caption,
};

#[derive(Default)]
struct Switch {
    selected: usize,
    suppressed: bool,
    takeover: Option<WeakEntity<Takeover>>,
    scroll: ScrollHandle,
}

type Close = Rc<dyn Fn(&mut Window, &mut App)>;

/// Lists managed windows over a scrim. Arrows or Tab move, Enter switches.
#[derive(IntoElement)]
pub struct WindowSwitcher {
    id: ElementId,
    usable_bounds: Option<Bounds<Pixels>>,
    on_close: Close,
}

impl WindowSwitcher {
    pub fn new(
        id: impl Into<ElementId>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            usable_bounds: None,
            on_close: Rc::new(on_close),
        }
    }

    /// Current usable rectangle in logical window coordinates.
    pub fn usable_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.usable_bounds = Some(bounds);
        self
    }
}

fn step(state: &Entity<Switch>, by: isize, count: usize, cx: &mut App) {
    if count == 0 {
        return;
    }
    state.update(cx, |switch, cx| {
        switch.selected = (switch.selected as isize + by).rem_euclid(count as isize) as usize;
        switch.scroll.scroll_to_item(switch.selected);
        cx.notify();
    });
}

impl RenderOnce for WindowSwitcher {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let windows = WindowManager::windows(cx);
        let count = windows.len();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Switch::default());
        let area = tooltip_available(self.usable_bounds, cx.theme().touch_mode, window);
        if state.read(cx).suppressed {
            return div().into_any_element();
        }
        let minimum = cx.theme().touch_minimum(
            cx.theme()
                .control_height(ControlSize::Lg)
                .to_pixels(window.rem_size()),
        );
        let room = tooltip_has_room(area, gpui::rems(0.5), window)
            && area.is_none_or(|area| {
                area.size.width > minimum * 2.
                    && area.size.height
                        > minimum * 2. + gpui::rems(1.).to_pixels(window.rem_size()) + gpui::px(4.)
            });
        if !room {
            let previous = state.update(cx, |state, _| {
                state.suppressed = true;
                state.takeover.take()
            });
            if let Some(previous) = previous.and_then(|weak| weak.upgrade()) {
                give_back(&previous, window, cx);
            }
            window.defer(cx, move |window, cx| (self.on_close)(window, cx));
            return div().into_any_element();
        }
        let takeover = take_focus(self.id.clone(), window, cx);
        state.update(cx, |state, _| state.takeover = Some(takeover.downgrade()));
        if count > 0 && state.read(cx).selected >= count {
            state.update(cx, |switch, _| switch.selected = count - 1);
        }
        let selected = state.read(cx).selected;
        let close: Close = {
            let (takeover, on_close) = (takeover.clone(), self.on_close);
            Rc::new(move |window, cx| {
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows: Vec<_> = windows
            .iter()
            .enumerate()
            .map(|(ix, (handle, title))| {
                let (handle, close) = (*handle, close.clone());
                div()
                    .id(("switch", ix))
                    .debug_selector(move || format!("window-switcher-row-{ix}"))
                    .role(gpui::Role::ListBoxOption)
                    .aria_label(title.clone())
                    .aria_selected(ix == selected)
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_3()
                    .h(minimum)
                    .px_3()
                    .rounded(theme.radius(Radius::Md))
                    .text_size(theme.text_size(TextSize::Base))
                    .when(ix == selected, |row| row.bg(colors.hover))
                    .cursor_pointer()
                    .on_click(move |_, window, cx| switch_to(handle, &close, window, cx))
                    .child(
                        Icon::new(IconName::AppWindow)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(title.clone()),
                    )
            })
            .collect();
        let handles: Vec<AnyWindowHandle> = windows.iter().map(|(handle, _)| *handle).collect();
        let (next, prev, keys, chosen) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        let (close_keys, close_enter, close_scrim, close_button) =
            (close.clone(), close.clone(), close.clone(), close);
        let focus = takeover.read(cx).focus.clone();
        let panel = div()
            .id("window-switcher")
            .debug_selector(|| "window-switcher-panel".into())
            .track_focus(&focus)
            .w(theme.sheet_size())
            .when_some(area, |panel, area| {
                panel
                    .w(area
                        .size
                        .width
                        .min(theme.sheet_size().to_pixels(window.rem_size())))
                    .max_h(area.size.height)
                    .overflow_hidden()
            })
            .p_2()
            .flex()
            .flex_col()
            .gap_0p5()
            .rounded(theme.radius(Radius::Xl))
            .bg(colors.overlay)
            .border_1()
            .border_color(colors.border)
            .shadow(theme.elevation(Elevation::Modal))
            .on_action(move |_: &FocusNext, _, cx| step(&next, 1, count, cx))
            .on_action(move |_: &FocusPrev, _, cx| step(&prev, -1, count, cx))
            .on_key_down(move |event, window, cx| {
                match event.keystroke.key.as_str() {
                    "down" => step(&keys, 1, count, cx),
                    "up" => step(&keys, -1, count, cx),
                    "escape" => close_keys(window, cx),
                    "enter" if !event.keystroke.modifiers.modified() => {}
                    _ => return,
                }
                cx.stop_propagation();
            })
            .on_key_up(move |event, window, cx| {
                let stroke = &event.keystroke;
                if stroke.key == "enter" && !stroke.modifiers.modified() {
                    cx.stop_propagation();
                    if let Some(handle) = handles.get(chosen.read(cx).selected) {
                        switch_to(*handle, &close_enter, window, cx);
                    }
                }
            })
            .when(theme.touch_mode, |panel| {
                panel.child(
                    div().flex().flex_none().justify_end().child(
                        IconButton::new("window-switcher-close", IconName::X)
                            .debug_selector(|| "window-switcher-close".into())
                            .tooltip("Close windows")
                            .on_click(move |_, window, cx| {
                                cx.stop_propagation();
                                close_button(window, cx);
                            }),
                    ),
                )
            })
            .child(
                div()
                    .id("window-switcher-list")
                    .role(gpui::Role::ListBox)
                    .debug_selector(|| "window-switcher-list".into())
                    .flex()
                    .flex_col()
                    .min_h_0()
                    .gap_0p5()
                    .overflow_y_scroll()
                    .track_scroll(&state.read(cx).scroll)
                    .children(rows)
                    .when(count == 0, |list| {
                        list.child(
                            div()
                                .p_3()
                                .child(Caption::new("No managed windows are open.")),
                        )
                    }),
            );
        let panel = if area.is_some() {
            panel.into_any_element()
        } else {
            panel
                .with_animation(
                    "switcher-in",
                    Animation::new(motion::duration(motion::BASE, cx))
                        .with_easing(motion::ease_out_cubic),
                    |panel, t| panel.opacity(t).mt(motion::NUDGE * (1.0 - t)),
                )
                .into_any_element()
        };
        Backdrop::new(self.id)
            .place(Place::Center)
            .when_some(area, |backdrop, area| backdrop.content_bounds(area))
            .on_dismiss(move |window, cx| close_scrim(window, cx))
            .child(panel)
            .into_any_element()
    }
}

#[cfg(all(test, feature = "test-support"))]
#[path = "switcher_tests.rs"]
mod tests;

fn switch_to(handle: AnyWindowHandle, close: &Close, window: &mut Window, cx: &mut App) {
    close(window, cx);
    if handle == window.window_handle() {
        window.activate_window();
        return;
    }
    if let Err(error) = WindowManager::focus(handle, cx) {
        log::error!("window switcher: {error:#}");
    }
}
