use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, Bounds, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Task, Window, canvas, div, prelude::*,
};

use super::{Popover, popover::bounded};
use crate::{
    buttons::IconButton,
    forms::{float_height, surface},
    primitives::IconName,
    theme::ActiveTheme,
};

/// How long the pointer rests on the trigger before the card opens.
const OPEN: Duration = Duration::from_millis(500);
/// How long the card waits after the pointer leaves, so it can cross the gap.
const LINGER: Duration = Duration::from_millis(200);

type Content = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Whether the card shows, what the pointer is over, the pending change and the trigger's box.
#[derive(Default)]
struct Hover {
    shown: bool,
    on_trigger: bool,
    on_card: bool,
    host: Bounds<Pixels>,
    height: Pixels,
    _pending: Option<Task<()>>,
}

/// After `after`, shows or hides the card as the pointer then warrants.
fn settle_after(state: &Entity<Hover>, after: Duration, window: &mut Window, cx: &mut App) {
    let weak = state.downgrade();
    let task = window.spawn(cx, async move |cx| {
        cx.background_executor().timer(after).await;
        let done = cx.update(|_, cx| {
            weak.update(cx, |hover, cx| {
                let shown = hover.on_trigger || hover.on_card;
                if hover.shown != shown {
                    log::info!("hover card: {}", if shown { "shown" } else { "hidden" });
                    hover.shown = shown;
                    cx.notify();
                }
                hover._pending = None;
            })
        });
        if let Err(error) = done.and_then(|inner| inner) {
            log::error!("hover card: trigger vanished: {error:#}");
        }
    });
    state.update(cx, |hover, _| hover._pending = Some(task));
}

/// A desktop hover preview; touch mode adds a separate disclosure with popover focus and dismissal.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    trigger: AnyElement,
    content: Content,
    usable_bounds: Option<Bounds<Pixels>>,
    touch_label: SharedString,
}

impl HoverCard {
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        trigger: impl IntoElement,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: trigger.into_any_element(),
            content: Rc::new(move |window, cx| content(window, cx).into_any_element()),
            usable_bounds: None,
            touch_label: "Show details".into(),
        }
    }

    /// Limits placement and scrolling content to the host's logical usable rectangle.
    /// If its frame leaves no content space, hides the card and cancels delayed opening.
    pub fn usable_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.usable_bounds = Some(bounds);
        self
    }

    /// Accessible name for the separate touch disclosure; the trigger keeps its own action.
    pub fn touch_label(mut self, label: impl Into<SharedString>) -> Self {
        self.touch_label = label.into();
        self
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| Hover::default());
        let touch = cx.theme().touch_mode;
        let available = bounded::available(self.usable_bounds, touch, window);
        if touch {
            state.update(cx, |hover, _| {
                hover._pending = None;
                hover.shown = false;
                hover.on_trigger = false;
                hover.on_card = false;
            });
            let content = self.content;
            let button_id = (self.id.clone(), "disclosure");
            let disclosure = Popover::with_opener(
                (self.id.clone(), "touch"),
                move |toggle| {
                    div()
                        .id("disclosure-host")
                        .debug_selector(|| "hover-card-disclosure".into())
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .child(
                            IconButton::new(button_id, IconName::Info)
                                .tooltip(self.touch_label)
                                .on_click(move |_, window, cx| {
                                    cx.stop_propagation();
                                    toggle(window, cx);
                                }),
                        )
                },
                move |_, window, cx| content(window, cx),
            );
            return div()
                .id(self.id)
                .flex()
                .items_center()
                .gap(cx.theme().touch_spacing())
                .child(self.trigger)
                .child(
                    disclosure
                        .when_some(available, |popover, bounds| popover.usable_bounds(bounds)),
                );
        }
        let room = bounded::has_room(available, bounded::HOVER_PADDING, window);
        let (enter, measure) = (state.clone(), state.clone());
        let host = div()
            .id(self.id.clone())
            .relative()
            .child(self.trigger)
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                if !room {
                    return;
                }
                enter.update(cx, |hover, _| hover.on_trigger = *hovered);
                settle_after(&enter, if *hovered { OPEN } else { LINGER }, window, cx);
            })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measure.read(cx).host != bounds {
                            measure.update(cx, |hover, cx| {
                                hover.host = bounds;
                                cx.notify();
                                window.request_animation_frame();
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        if !room {
            state.update(cx, |hover, _| {
                hover._pending = None;
                hover.shown = false;
                hover.on_trigger = false;
                hover.on_card = false;
            });
            return host;
        }
        if !state.read(cx).shown {
            return host;
        }
        let (over, tall) = (state.clone(), state.clone());
        let card = surface((self.id.clone(), "card"), cx)
            .debug_selector(|| "hover-card-panel".into())
            .relative()
            .occlude()
            .p(bounded::HOVER_PADDING)
            .map(|card| {
                if let Some(bounds) = available {
                    card.max_w(bounds.size.width)
                        .max_h(bounds.size.height)
                        .overflow_scroll()
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                } else {
                    card
                }
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                over.update(cx, |hover, _| hover.on_card = *hovered);
                settle_after(&over, LINGER, window, cx);
            })
            .child((self.content)(window, cx))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if tall.read(cx).height != bounds.size.height {
                            tall.update(cx, |hover, cx| {
                                hover.height = bounds.size.height;
                                cx.notify();
                                window.request_animation_frame();
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        let (anchor, height) = (state.read(cx).host, state.read(cx).height);
        if let Some(bounds) = available {
            return host.child(bounded::place(self.id, anchor, bounds, card));
        }
        host.child(float_height(
            self.id.clone(),
            anchor,
            height,
            card,
            window,
            cx,
        ))
    }
}
