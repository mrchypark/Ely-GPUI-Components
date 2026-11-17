use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, DispatchPhase, ElementId, FontWeight, GestureTuning,
    HitboxBehavior, InteractiveElement, IntoElement, LongPressEvent, MouseButton, MouseMoveEvent,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Task, TouchPhase,
    Window, canvas, div, prelude::*, relative,
};

use super::button::{ButtonVariant, label_size, tone};
use crate::{
    motion,
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, ControlSize, Radius},
};

/// How long a hold takes to confirm.
const HOLD: Duration = Duration::from_millis(1200);
/// How long a first click stays armed.
const ARMED: Duration = Duration::from_millis(3000);
/// How long the done state shows.
const DONE: Duration = Duration::from_millis(1400);

// shortcut: assumes GPUI's default recognition window; revisit if mobile overrides gestures.
fn touch_hold_after_recognition() -> Duration {
    HOLD.saturating_sub(GestureTuning::default().long_press_duration)
}

/// Hold the button down, or click it twice. Keys always press twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmMode {
    Hold,
    Twice,
}

#[derive(Default)]
struct Confirm {
    holding: bool,
    armed: bool,
    done: bool,
    generation: u64,
    hold_duration: Duration,
    _timer: Option<Task<()>>,
}

type OnConfirm = Rc<dyn Fn(&mut Window, &mut App)>;

/// A destructive button that asks twice: by holding, or by a second click.
#[derive(IntoElement)]
pub struct ConfirmButton {
    id: ElementId,
    label: SharedString,
    armed_label: SharedString,
    mode: ConfirmMode,
    size: ControlSize,
    on_confirm: Option<OnConfirm>,
}

impl ConfirmButton {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        mode: ConfirmMode,
    ) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            armed_label: match mode {
                ConfirmMode::Hold => "Press again to confirm".into(),
                ConfirmMode::Twice => "Click again to confirm".into(),
            },
            label,
            mode,
            size: ControlSize::default(),
            on_confirm: None,
        }
    }

    /// The label while waiting for the second click.
    pub fn armed_label(mut self, label: impl Into<SharedString>) -> Self {
        self.armed_label = label.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }
}

/// Clears `field` after `after`, unless a newer change came first.
fn expire(
    state: &gpui::Entity<Confirm>,
    after: Duration,
    clear: fn(&mut Confirm),
    window: &mut Window,
    cx: &mut App,
) -> Task<()> {
    let (weak, generation) = (state.downgrade(), state.read(cx).generation);
    window.spawn(cx, async move |cx| {
        cx.background_executor().timer(after).await;
        let cleared = cx.update(|_, cx| {
            weak.update(cx, |confirm, cx| {
                if confirm.generation == generation {
                    clear(confirm);
                    cx.notify();
                }
            })
        });
        if let Err(error) = cleared.and_then(|inner| inner) {
            log::error!("confirm button: state gone before it expired: {error:#}");
        }
    })
}

fn confirm_now(
    state: &gpui::Entity<Confirm>,
    handler: &Option<OnConfirm>,
    window: &mut Window,
    cx: &mut App,
) {
    log::info!("confirm button: confirmed");
    state.update(cx, |confirm, _| {
        confirm.holding = false;
        confirm.armed = false;
        confirm.done = true;
        confirm.hold_duration = Duration::ZERO;
        confirm.generation += 1;
    });
    let timer = expire(state, DONE, |confirm| confirm.done = false, window, cx);
    state.update(cx, |confirm, cx| {
        confirm._timer = Some(timer);
        cx.notify();
    });
    if let Some(handler) = handler {
        handler(window, cx);
    }
}

fn begin_hold(
    state: &gpui::Entity<Confirm>,
    handler: &Option<OnConfirm>,
    duration: Duration,
    window: &mut Window,
    cx: &mut App,
) {
    if state.read(cx).done || state.read(cx).holding {
        return;
    }
    state.update(cx, |confirm, _| {
        confirm.holding = true;
        confirm.armed = false;
        confirm.generation += 1;
        confirm.hold_duration = duration;
    });
    let (weak, handler) = (state.downgrade(), handler.clone());
    let generation = state.read(cx).generation;
    let timer = window.spawn(cx, async move |cx| {
        cx.background_executor().timer(duration).await;
        let held = cx.update(|window, cx| {
            let Some(state) = weak.upgrade() else {
                return;
            };
            if state.read(cx).holding && state.read(cx).generation == generation {
                confirm_now(&state, &handler, window, cx);
            }
        });
        if let Err(error) = held {
            log::error!("confirm button: window gone during a hold: {error:#}");
        }
    });
    state.update(cx, |confirm, cx| {
        confirm._timer = Some(timer);
        cx.notify();
    });
}

impl RenderOnce for ConfirmButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Confirm::default());
        let (holding, armed, done, generation, hold_duration) = {
            let confirm = state.read(cx);
            (
                confirm.holding,
                confirm.armed,
                confirm.done,
                confirm.generation,
                confirm.hold_duration,
            )
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let variant = if armed {
            ButtonVariant::Danger
        } else {
            ButtonVariant::Outline
        };
        let tone = tone(variant, colors);
        let (text, icon_size) = label_size(self.size);
        let label = if armed {
            self.armed_label.clone()
        } else {
            self.label.clone()
        };
        let fill = colors.danger.opacity(0.16);
        let handler = self.on_confirm.clone();
        let (press, release, out, click) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        let hold = self.mode == ConfirmMode::Hold;
        let (hold_handler, click_handler) = (handler.clone(), handler);
        let touch = hold.then(|| {
            let (press_outside, started, handler) =
                (state.clone(), state.clone(), hold_handler.clone());
            canvas(
                |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                move |_, hitbox, window, _| {
                    let moved_hitbox = hitbox.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble
                            && event.pressed_button == Some(MouseButton::Left)
                            && press_outside.read(cx).holding
                            && !moved_hitbox.is_hovered_at(event.position, window)
                        {
                            let_go(&press_outside, cx);
                        }
                    });
                    window.on_mouse_event(move |event: &LongPressEvent, phase, window, cx| {
                        if phase != DispatchPhase::Bubble
                            || !hitbox.is_hovered_at(event.start_position, window)
                        {
                            return;
                        }
                        match event.phase {
                            TouchPhase::Started => {
                                window.prevent_default();
                                begin_hold(
                                    &started,
                                    &handler,
                                    touch_hold_after_recognition(),
                                    window,
                                    cx,
                                );
                            }
                            TouchPhase::Ended | TouchPhase::Cancelled => let_go(&started, cx),
                            TouchPhase::Moved if !hitbox.is_hovered_at(event.position, window) => {
                                let_go(&started, cx);
                            }
                            TouchPhase::Moved => {}
                        }
                    });
                },
            )
            .absolute()
            .inset_0()
        });
        div()
            .id(self.id)
            .debug_selector(|| "confirm-button".into())
            .relative()
            .overflow_hidden()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap_1p5()
            .h(theme.control_height(self.size))
            .map(|el| theme.touch_target(el))
            .px(theme.control_padding(self.size))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if holding { colors.danger } else { tone.border })
            .bg(tone.bg)
            .text_size(theme.text_size(text))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if holding { colors.danger } else { tone.fg })
            .cursor_pointer()
            .tab_index(0)
            .focus_ring(cx)
            .hover(|style| style.bg(tone.hover))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.prevent_default();
                if !hold {
                    return;
                }
                begin_hold(&press, &hold_handler, HOLD, window, cx);
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| let_go(&release, cx))
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| let_go(&out, cx))
            .on_click(move |event, window, cx| {
                if (hold && !event.is_keyboard()) || click.read(cx).done {
                    return;
                }
                if click.read(cx).armed {
                    confirm_now(&click, &click_handler, window, cx);
                    return;
                }
                log::info!("confirm button: armed");
                click.update(cx, |confirm, _| {
                    confirm.holding = false;
                    confirm.armed = true;
                    confirm.hold_duration = Duration::ZERO;
                    confirm.generation += 1;
                });
                let timer = expire(&click, ARMED, |confirm| confirm.armed = false, window, cx);
                click.update(cx, |confirm, cx| {
                    confirm._timer = Some(timer);
                    cx.notify();
                });
            })
            .when(holding, |button| {
                button.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .bg(fill)
                        .with_animation(
                            ("confirm-hold", generation),
                            Animation::new(hold_duration),
                            |bar, t| bar.w(relative(t)),
                        ),
                )
            })
            .children(touch)
            .map(|button| {
                if done {
                    button.child(
                        Icon::new(IconName::Check)
                            .size(icon_size)
                            .color(colors.success)
                            .with_animation(
                                ("confirm-done", generation),
                                Animation::new(motion::duration(motion::FAST, cx))
                                    .with_easing(motion::ease_out_cubic),
                                |icon, t| icon.rotate(gpui::radians((1.0 - t) * -0.6)),
                            ),
                    )
                } else {
                    button
                }
            })
            .child(label)
    }
}

fn let_go(state: &gpui::Entity<Confirm>, cx: &mut App) {
    if !state.read(cx).holding {
        return;
    }
    log::info!("confirm button: released early");
    state.update(cx, |confirm, cx| {
        confirm.holding = false;
        confirm.hold_duration = Duration::ZERO;
        confirm.generation += 1;
        confirm._timer = None;
        cx.notify();
    });
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::{
        Context, IntoElement, KeyUpEvent, Keystroke, Modifiers, MouseButton, ParentElement, Render,
        Styled, TestAppContext, TouchEvent, TouchId, TouchPhase, VisualTestContext, Window, div,
        point, px,
    };

    use super::{ConfirmButton, ConfirmMode, HOLD};
    use crate::theme::Theme;

    struct Probe {
        mode: ConfirmMode,
        confirms: usize,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity();
            div().size_full().child(
                ConfirmButton::new("confirm", "Delete", self.mode)
                    .on_confirm(move |_, cx| view.update(cx, |probe, _| probe.confirms += 1)),
            )
        }
    }

    fn open(
        mode: ConfirmMode,
        cx: &mut TestAppContext,
    ) -> (gpui::Entity<Probe>, &mut VisualTestContext) {
        cx.update(|cx| {
            Theme::init(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (view, cx) = cx.add_window_view(|_, _| Probe { mode, confirms: 0 });
        cx.update(|window, _| window.activate_window());
        draw(cx);
        (view, cx)
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }

    fn touch(
        phase: TouchPhase,
        id: TouchId,
        center: gpui::Point<gpui::Pixels>,
        cx: &mut VisualTestContext,
    ) {
        cx.simulate_event(TouchEvent {
            id,
            phase,
            position: center,
            predicted_position: None,
            force: None,
        });
    }

    fn advance(cx: &mut VisualTestContext, duration: Duration) {
        cx.executor().advance_clock(duration);
        cx.run_until_parked();
    }

    fn press(key: &'static str, cx: &mut VisualTestContext) {
        cx.simulate_keystrokes(key);
        draw(cx);
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse(key).expect("a key"),
        });
        draw(cx);
    }

    #[gpui::test]
    fn touch_long_press_confirms_after_one_hold_duration(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        let center = cx
            .debug_bounds("confirm-button")
            .expect("confirm button")
            .center();
        touch(TouchPhase::Started, TouchId(1), center, cx);
        advance(cx, Duration::from_millis(499));
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        advance(cx, Duration::from_millis(1));
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        advance(cx, Duration::from_millis(600));
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        advance(cx, Duration::from_millis(100));
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 1);
    }

    #[gpui::test]
    fn cancelled_touch_long_press_does_not_confirm(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        let center = cx
            .debug_bounds("confirm-button")
            .expect("confirm button")
            .center();
        touch(TouchPhase::Started, TouchId(1), center, cx);
        advance(cx, Duration::from_millis(500));
        advance(cx, HOLD / 2);
        touch(TouchPhase::Cancelled, TouchId(1), center, cx);
        advance(cx, HOLD);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    }

    #[gpui::test]
    fn moving_outside_after_recognition_cancels_the_hold(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        let bounds = cx.debug_bounds("confirm-button").expect("confirm button");
        let center = bounds.center();
        let outside = point(bounds.right() + px(20.), center.y);
        touch(TouchPhase::Started, TouchId(1), center, cx);
        advance(cx, Duration::from_millis(500));
        touch(TouchPhase::Moved, TouchId(1), outside, cx);
        advance(cx, HOLD);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        touch(TouchPhase::Ended, TouchId(1), outside, cx);
    }

    #[gpui::test]
    fn mouse_exit_cancels_the_hold(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        let bounds = cx.debug_bounds("confirm-button").expect("confirm button");
        let center = bounds.center();
        let outside = point(bounds.right() + px(20.), center.y);
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
        advance(cx, HOLD);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        cx.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
    }

    #[gpui::test]
    fn cancelled_hold_after_keyboard_arm_requires_two_new_keyboard_activations(
        cx: &mut TestAppContext,
    ) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        cx.update(|window, cx| window.focus_next(cx));
        draw(cx);

        press("enter", cx);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);

        let center = cx
            .debug_bounds("confirm-button")
            .expect("confirm button")
            .center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::none());
        advance(cx, HOLD / 2);
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::none());
        draw(cx);

        press("enter", cx);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        press("enter", cx);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 1);
    }

    #[gpui::test]
    fn moving_before_recognition_keeps_the_gesture_out_of_hold(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        let bounds = cx.debug_bounds("confirm-button").expect("confirm button");
        let center = bounds.center();
        let panned = point(center.x + px(20.), center.y);
        touch(TouchPhase::Started, TouchId(1), center, cx);
        touch(TouchPhase::Moved, TouchId(1), panned, cx);
        advance(cx, Duration::from_millis(500) + HOLD);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
        touch(TouchPhase::Ended, TouchId(1), panned, cx);
    }

    #[gpui::test]
    fn touch_tap_does_not_start_a_confirming_hold(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Hold, cx);
        let center = cx
            .debug_bounds("confirm-button")
            .expect("confirm button")
            .center();
        touch(TouchPhase::Started, TouchId(1), center, cx);
        advance(cx, Duration::from_millis(100));
        touch(TouchPhase::Ended, TouchId(1), center, cx);
        advance(cx, HOLD);
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 0);
    }

    #[gpui::test]
    fn twice_mode_keeps_the_double_click_fallback(cx: &mut TestAppContext) {
        let (view, cx) = open(ConfirmMode::Twice, cx);
        let center = cx
            .debug_bounds("confirm-button")
            .expect("confirm button")
            .center();
        cx.simulate_click(center, Modifiers::none());
        cx.run_until_parked();
        cx.simulate_click(center, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |probe, _| probe.confirms), 1);
    }
}
