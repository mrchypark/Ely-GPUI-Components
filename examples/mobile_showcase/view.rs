use ely_gpui_component::{
    buttons::Button,
    canvas::{CanvasTouch, InfiniteCanvas, TouchAction, TouchContact, Viewport, ZoomControls},
    forms::{Input, TextInput},
    layout::{ViewportInsets, is_compact, reveal_when_focused_in_bounds, usable_bounds},
    menus::{DropdownMenu, Menu, MenuItem},
    overlays::Dialog,
    primitives::{FocusScope, Place},
    theme::ActiveTheme,
};
use gpui::{
    AppContext, Axis, Bounds, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, ScrollHandle, StatefulInteractiveElement, Styled,
    Window, div, point, px,
};

pub struct Showcase {
    focus: FocusHandle,
    input: Entity<TextInput>,
    viewport: Viewport,
    keyboard: bool,
    safe: bool,
    scroll: ScrollHandle,
    navigation: bool,
    info: bool,
}

impl Showcase {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            focus,
            input: cx.new(|cx| TextInput::new(window, cx).placeholder("Map label")),
            viewport: Viewport::new(0.0, 0.0, 1.0),
            keyboard: false,
            safe: false,
            scroll: ScrollHandle::new(),
            navigation: false,
            info: false,
        }
    }
}

// Replays normalized contacts; it is not an OS touch event adapter.
fn pinch(view: Viewport) -> Viewport {
    let mut touch = CanvasTouch::new(4.0);
    let pair = |distance: f32| {
        [
            TouchContact {
                id: 1,
                at: (100.0 - distance, 100.0),
            },
            TouchContact {
                id: 2,
                at: (100.0 + distance, 100.0),
            },
        ]
    };
    touch.update(&pair(20.0), view);
    match touch.update(&pair(30.0), view) {
        TouchAction::Viewport(next) => next,
        other => panic!("pinch replay did not yield a viewport: {other:?}"),
    }
}

impl Render for Showcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = if self.safe {
            ViewportInsets {
                top: px(24.0),
                bottom: px(34.0),
                ..Default::default()
            }
        } else {
            ViewportInsets::default()
        };
        let available = usable_bounds(
            Bounds::new(point(Pixels::ZERO, Pixels::ZERO), window.viewport_size()),
            safe,
            if self.keyboard {
                px(240.0)
            } else {
                Pixels::ZERO
            },
        );
        let compact = is_compact(available, px(600.0));
        let colors = cx.theme().colors.clone();
        let keyboard = cx.entity();
        let safe_toggle = cx.entity();
        let replay = cx.entity();
        let pan = cx.entity();
        let zoom = cx.entity();
        let nav_open = cx.entity();
        let info_open = cx.entity();
        let nav_close = cx.entity();
        let info_close = cx.entity();
        let navigation = if compact {
            Button::new("navigation", "Navigation")
                .on_click(move |_, _, cx| {
                    nav_open.update(cx, |view, cx| {
                        view.navigation = true;
                        cx.notify();
                    });
                })
                .into_any_element()
        } else {
            div().child("Navigation · Map workspace").into_any_element()
        };
        let sheet = (compact && self.navigation).then(|| {
            ely_gpui_component::layout::Sheet::new(
                "navigation-sheet",
                Place::Bottom,
                "Navigation",
                move |_, cx| {
                    nav_close.update(cx, |view, cx| {
                        view.navigation = false;
                        cx.notify();
                    });
                },
            )
            .usable_bounds(available)
            .child("Map workspace")
        });
        let dialog = self.info.then(|| {
            Dialog::new("map-info", "Map preview", move |_, cx| {
                info_close.update(cx, |view, cx| {
                    view.info = false;
                    cx.notify();
                });
            })
            .usable_bounds(available)
            .detail("Simulated insets and raw contacts; no native mobile backend is connected.")
            .action(|close| {
                Button::new("info-close", "Close").on_click(move |_, window, cx| close(window, cx))
            })
        });
        let controls = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new(
                    "safe",
                    if self.safe {
                        "Safe area: on"
                    } else {
                        "Safe area: off"
                    },
                )
                .on_click(move |_, _, cx| {
                    safe_toggle.update(cx, |view, cx| {
                        view.safe = !view.safe;
                        cx.notify();
                    })
                }),
            )
            .child(
                Button::new(
                    "keyboard",
                    if self.keyboard {
                        "Hide mock keyboard"
                    } else {
                        "Show mock keyboard"
                    },
                )
                .on_click(move |_, _, cx| {
                    keyboard.update(cx, |view, cx| {
                        view.keyboard = !view.keyboard;
                        cx.notify();
                    })
                }),
            )
            .child(
                Button::new("pinch", "Replay pinch").on_click(move |_, _, cx| {
                    replay.update(cx, |view, cx| {
                        view.viewport = pinch(view.viewport);
                        cx.notify();
                    })
                }),
            );
        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .relative()
            .bg(colors.surface)
            .text_color(colors.fg)
            .child(
                div()
                    .absolute()
                    .left(available.origin.x)
                    .top(available.origin.y)
                    .w(available.size.width)
                    .h(available.size.height)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .bg(colors.bg)
                    .child(
                        div()
                            .id("controls")
                            .track_scroll(&self.scroll)
                            .overflow_y_scroll()
                            .max_h(available.size.height / 2.0)
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(if compact {
                                "Compact map"
                            } else {
                                "Map workspace"
                            })
                            .child("Desktop preview · simulated insets and contacts")
                            .child(controls)
                            .child(navigation)
                            .child(DropdownMenu::new(
                                "mobile-menu",
                                "Menu",
                                Menu::new().usable_bounds(available).item(MenuItem::submenu(
                                    "Workspace",
                                    Menu::new()
                                        .item(MenuItem::new("Map"))
                                        .item(MenuItem::new("Layers")),
                                )),
                            ))
                            .child(Button::new("info", "Map info").on_click(move |_, _, cx| {
                                info_open.update(cx, |view, cx| {
                                    view.info = true;
                                    cx.notify();
                                });
                            }))
                            .child(div().relative().child(Input::new(&self.input)).child(
                                reveal_when_focused_in_bounds(
                                    "input-reveal",
                                    &self.scroll,
                                    &self.input.focus_handle(cx),
                                    Axis::Vertical,
                                    available,
                                    window,
                                    cx,
                                ),
                            ))
                            .child(ZoomControls::new("zoom", self.viewport.zoom).on_zoom(
                                move |value, _, cx| {
                                    zoom.update(cx, |view, cx| {
                                        view.viewport = view
                                            .viewport
                                            .zoomed(value / view.viewport.zoom, (100.0, 100.0));
                                        cx.notify();
                                    })
                                },
                            )),
                    )
                    .child(div().flex_1().min_h_0().overflow_hidden().child(
                        InfiniteCanvas::new("map", self.viewport).on_viewport(
                            move |value, _, cx| {
                                pan.update(cx, |view, cx| {
                                    view.viewport = value;
                                    cx.notify();
                                });
                            },
                        ),
                    )),
            )
            .children(sheet)
            .children(dialog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ely_gpui_component::canvas::Frame;
    use gpui::size;

    #[test]
    fn patch_map_viewport_contract_keeps_pinch_anchor() {
        let frame = Frame::new(10.0, 20.0, 500.0, 300.0);
        let fitted = Viewport::fitting(frame, (320.0, 400.0), 16.0);
        let before = fitted.to_canvas((100.0, 100.0));
        let after = pinch(fitted).to_canvas((100.0, 100.0));
        assert!((before.0 - after.0).abs() < 0.001);
        assert!((before.1 - after.1).abs() < 0.001);
    }

    #[test]
    fn safe_origin_is_removed_before_canvas_hit_testing() {
        let available = usable_bounds(
            Bounds::new(point(px(0.0), px(0.0)), size(px(390.0), px(844.0))),
            ViewportInsets {
                left: px(12.0),
                top: px(24.0),
                bottom: px(34.0),
                ..Default::default()
            },
            px(300.0),
        );
        assert_eq!(available.size.height, px(520.0));
        let local = point(px(112.0), px(124.0)) - available.origin;
        let view = Viewport::new(10.0, 20.0, 2.0);
        assert_eq!(
            view.to_canvas((f32::from(local.x), f32::from(local.y))),
            (60.0, 70.0)
        );
    }
}
