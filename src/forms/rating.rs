use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Pixels, RenderOnce, StatefulInteractiveElement, Styled, TouchPhase, Window, canvas, div,
    prelude::*, transparent_black,
};

use crate::{
    data_display::star,
    primitives::tab_stop,
    theme::{ActiveTheme, IconSize, Radius},
};

fn paint_star(bounds: Bounds<Pixels>, filled: bool, color: Hsla, window: &mut Window) {
    if filled {
        window.paint_path(star(bounds, true), color);
    }
    window.paint_path(star(bounds, false), color);
}

type OnRate = Rc<dyn Fn(u8, &mut Window, &mut App)>;

/// Stars from one to `max`. Hover previews; clicking the current value clears it.
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    value: u8,
    max: u8,
    disabled: bool,
    on_change: Option<OnRate>,
}

impl Rating {
    pub fn new(id: impl Into<ElementId>, value: u8) -> Self {
        Self {
            id: id.into(),
            value,
            max: 5,
            disabled: false,
            on_change: None,
        }
    }

    pub fn max(mut self, max: u8) -> Self {
        assert!(max > 0, "a rating needs at least one star");
        self.max = max;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(u8, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Rating {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (value, max) = (self.value, self.max);
        assert!(value <= max, "rating {value} is above {max}");
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            !self.disabled,
            window,
            cx,
        );
        let focused = focus.is_focused(window);
        let hover = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| None::<u8>);
        // A finger has no hover, so the press alone shows the preview. A pointer
        // previews on hover and keeps it after the release.
        let pressed = window.use_keyed_state((self.id.clone(), "pressed"), cx, |_, _| None::<u8>);
        // A click commits a rating, so that rating stays on screen even once
        // the pointer leaves the row. The value it was committed against is
        // kept alongside it, so a lock that belongs to an older rating is
        // ignored instead of previewing a rating nobody asked for.
        let locked_state =
            window.use_keyed_state((self.id.clone(), "locked"), cx, |_, _| None::<(u8, u8)>);
        let locked = locked_state
            .read(cx)
            .filter(|(committed, _)| *committed == value)
            .map(|(_, star)| star);
        let shown = pressed
            .read(cx)
            .or(*hover.read(cx))
            .or(locked)
            .unwrap_or(value);
        let rate: OnRate = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next, window, cx| {
                log::info!("rating {id:?}: {next}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (on, off) = (colors.accent, colors.border_strong);
        let stars: Vec<_> = (1..=max)
            .map(|n| {
                let filled = n <= shown;
                let color = if filled { on } else { off };
                let (hover, rate) = (hover.clone(), rate.clone());
                let (pressed, rate) = (pressed.clone(), rate.clone());
                let (locked, rate) = (locked_state.clone(), rate.clone());
                div()
                    .id(("star", n as usize))
                    .debug_selector(move || format!("rating-star-{n}"))
                    .flex()
                    .items_center()
                    .justify_center()
                    .map(|star| theme.touch_target(star))
                    .child(
                        div().flex_none().size(theme.icon_size(IconSize::Lg)).child(
                            canvas(
                                |_, _, _| {},
                                move |bounds, _, window, _| {
                                    paint_star(bounds, filled, color, window)
                                },
                            )
                            .size_full(),
                        ),
                    )
                    .when(!self.disabled, |star| {
                        star.cursor_pointer()
                            .on_hover(move |inside, _, cx| {
                                // A finger never hovers, so a touch keeps the press
                                // as the only preview it shows.
                                if *inside && !cx.theme().touch_mode {
                                    hover.update(cx, |hover, cx| {
                                        *hover = Some(n);
                                        cx.notify();
                                    });
                                }
                            })
                            // A tap arrives as a mouse down without a hover,
                            // so the press shows the preview the pointer would.
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                window.prevent_default();
                                pressed.update(cx, |hover, cx| {
                                    *hover = Some(n);
                                    cx.notify();
                                });
                            })
                            .on_click(move |_, window, cx| {
                                let next = if n == value { 0 } else { n };
                                if !cx.theme().touch_mode {
                                    // The pointer keeps the rating it just
                                    // committed, even once it leaves the row.
                                    locked.update(cx, |locked, cx| {
                                        *locked = Some((value, n));
                                        cx.notify();
                                    });
                                }
                                rate(next, window, cx)
                            })
                    })
            })
            .collect();
        let (leave, keys) = (hover, rate.clone());
        let keys_locked = locked_state.clone();
        div()
            .id(self.id)
            .track_focus(&focus)
            .flex()
            .gap_0p5()
            .p_0p5()
            .rounded(theme.radius(Radius::Sm))
            .border_1()
            .border_color(if focused {
                colors.focus
            } else {
                transparent_black()
            })
            .when(self.disabled, |row| row.opacity(0.5))
            .when(shown != value, |row| {
                row.debug_selector(|| "rating-preview".into())
            })
            .on_hover(move |inside, _, cx| {
                // The release clears the press, so a pointer keeps its hover preview.
                if !*inside && !cx.theme().touch_mode {
                    leave.update(cx, |hover, cx| {
                        *hover = None;
                        cx.notify();
                    });
                }
            })
            // A finger lifts off the star, so the press preview goes with it.
            .on_mouse_up(MouseButton::Left, {
                let pressed = pressed.clone();
                move |_, _, cx| {
                    pressed.update(cx, |pressed, cx| {
                        if pressed.is_some() {
                            *pressed = None;
                            cx.notify();
                        }
                    });
                }
            })
            // A finger that leaves the row before it lifts takes the press
            // preview with it.
            .on_mouse_up_out(MouseButton::Left, {
                let pressed = pressed.clone();
                move |_, _, cx| {
                    pressed.update(cx, |pressed, cx| {
                        if pressed.is_some() {
                            *pressed = None;
                            cx.notify();
                        }
                    });
                }
            })
            // A touch that turns into a scroll or is cancelled by the system
            // never delivers a release, so the press preview is dropped here.
            .on_scroll_wheel({
                let pressed = pressed.clone();
                move |event, _, cx| {
                    if matches!(event.touch_phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                        pressed.update(cx, |pressed, cx| {
                            if pressed.is_some() {
                                *pressed = None;
                                cx.notify();
                            }
                        });
                    }
                }
            })
            .when(!self.disabled, |row| {
                row.on_key_down(move |event, window, cx| {
                    let next = match event.keystroke.key.as_str() {
                        "left" | "down" => value.saturating_sub(1),
                        "right" | "up" => (value + 1).min(max),
                        "home" => 0,
                        "end" => max,
                        _ => return,
                    };
                    cx.stop_propagation();
                    if next != value {
                        // The keyboard owns the rating again, so a rating left
                        // on screen by a click stops holding the preview.
                        keys_locked.update(cx, |locked, cx| {
                            *locked = None;
                            cx.notify();
                        });
                        keys(next, window, cx);
                    }
                })
            })
            .children(stars)
    }
}
