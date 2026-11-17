use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, canvas, div, prelude::*,
};

use super::TextInput;
use crate::{
    buttons::IconButton,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// A text field's frame: border, focus, and room for things before and after.
#[derive(IntoElement)]
pub struct Input {
    state: Entity<TextInput>,
    size: ControlSize,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    clearable: bool,
    invalid: bool,
    least: Option<SharedString>,
    reveal: bool,
    secure: bool,
}

impl Input {
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            size: ControlSize::default(),
            prefix: None,
            suffix: None,
            clearable: false,
            invalid: false,
            least: None,
            reveal: true,
            secure: false,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Holds room to show `text` whole.
    pub fn least(mut self, text: impl Into<SharedString>) -> Self {
        self.least = Some(text.into());
        self
    }

    /// Adds a clear button while there is text.
    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    /// Draws the frame in the danger tone.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub(crate) fn without_reveal(mut self) -> Self {
        self.reveal = false;
        self
    }

    fn secure(mut self) -> Self {
        self.secure = true;
        self
    }
}

fn should_request_keyboard(focused: bool, disabled: bool, prevented: bool) -> bool {
    focused && !disabled && !prevented
}

pub(crate) fn text_size(size: ControlSize) -> TextSize {
    match size {
        ControlSize::Sm => TextSize::Sm,
        ControlSize::Md => TextSize::Base,
        ControlSize::Lg => TextSize::Md,
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.reveal {
            self.state
                .update(cx, |input, _| input.clear_reveal_bounds());
        }
        self.state.update(cx, |input, cx| {
            if self.secure {
                input.enter_secure(cx);
            } else {
                input.leave_secure(cx);
            }
        });
        let input = self.state.read(cx);
        let focus = input.focus().clone();
        let (focused, empty, multi, disabled) = (
            focus.is_focused(window),
            input.is_empty(),
            input.is_multi_line(),
            input.is_disabled(),
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let border = if self.invalid {
            colors.danger
        } else if focused {
            colors.focus
        } else {
            colors.border_strong
        };
        let (state, clear_id) = (self.state.clone(), self.state.entity_id());
        let text = state.clone();
        let focus_state = self.state.clone();
        let disabled_state = self.state.clone();
        let content = div()
            .debug_selector(|| "input-root".into())
            .flex()
            .gap_2()
            .px(theme.control_padding(self.size))
            .map(|frame| {
                if multi {
                    frame.items_start().py_1p5()
                } else {
                    frame.items_center().h(theme.control_height(self.size))
                }
            })
            .map(|frame| theme.touch_target(frame))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(border)
            .bg(if disabled {
                colors.sunken
            } else {
                colors.surface
            })
            .text_size(theme.text_size(text_size(self.size)))
            .text_color(if disabled {
                colors.fg_disabled
            } else {
                colors.fg
            })
            .capture_any_mouse_down(move |_, window, cx| {
                if disabled_state.read(cx).is_disabled() {
                    window.prevent_default();
                }
            })
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                let disabled = focus_state.read(cx).is_disabled();
                if window.default_prevented() || disabled {
                    return;
                }
                if should_request_keyboard(focus.is_focused(window), disabled, false) {
                    window.request_virtual_keyboard();
                }
                window.focus(&focus, cx);
            })
            .children(self.prefix)
            .child(
                div()
                    .debug_selector(|| "input-text".into())
                    .flex_1()
                    .min_w_0()
                    .child(text)
                    // A strut keeps the frame's inset to type in.
                    .child(
                        div()
                            .px(theme.control_padding(self.size))
                            .border_x_1()
                            .h_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .children(self.least),
                    ),
            )
            .when(self.clearable && !empty && !disabled, |frame| {
                frame.child(
                    IconButton::new(("input-clear", clear_id), IconName::CircleX)
                        .size(ControlSize::Sm)
                        .tooltip("Clear")
                        .on_click(move |_, window, cx| {
                            state.update(cx, |input, cx| input.set_text("", cx));
                            window.focus(&state.read(cx).focus().clone(), cx);
                        }),
                )
            })
            .children(self.suffix);
        let reveal = self.state.clone();
        let track_reveal = self.reveal;
        div().relative().w_full().child(content).child(
            canvas(
                move |bounds, _, cx| {
                    if track_reveal {
                        reveal.update(cx, |input, _| input.set_reveal_bounds(bounds));
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

/// A masked field with a lock before it and an eye that shows the text.
#[derive(IntoElement)]
pub struct PasswordInput {
    state: Entity<TextInput>,
    size: ControlSize,
}

impl PasswordInput {
    /// Masks ordinary text state on its first render.
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            size: ControlSize::default(),
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for PasswordInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let secured =
            window.use_keyed_state(("password-secured", self.state.entity_id()), cx, |_, _| {
                false
            });
        if !*secured.read(cx) {
            self.state.update(cx, |input, cx| input.remask_secure(cx));
            secured.update(cx, |secured, _| *secured = true);
        }
        let masked = self.state.read(cx).is_masked();
        let disabled = self.state.read(cx).is_disabled();
        let (toggle, subtle) = (self.state.clone(), cx.theme().colors.fg_subtle);
        Input::new(&self.state)
            .size(self.size)
            .secure()
            .prefix(Icon::new(IconName::Lock).size(IconSize::Sm).color(subtle))
            .suffix(
                IconButton::new(
                    ("password-eye", self.state.entity_id()),
                    if masked {
                        IconName::Eye
                    } else {
                        IconName::EyeOff
                    },
                )
                .debug_selector(|| "password-visibility".into())
                .size(ControlSize::Sm)
                .disabled(disabled)
                .tooltip(if masked {
                    "Show password"
                } else {
                    "Hide password"
                })
                .on_click(move |_, _, cx| {
                    toggle.update(cx, |input, cx| {
                        if input.is_disabled() {
                            return;
                        }
                        let masked = input.is_masked();
                        log::info!(
                            "password input: {}",
                            if masked { "shown" } else { "hidden" }
                        );
                        input.set_revealed(masked, cx);
                    })
                }),
            )
    }
}

#[cfg(all(test, feature = "test-support"))]
mod password_tests {
    use gpui::{
        AppContext as _, Context, Entity, EntityInputHandler, InteractiveElement, IntoElement,
        Modifiers, ParentElement, Render, TestAppContext, TextInputConfiguration,
        VisualTestContext, Window, div, point, px,
    };

    use super::{Input, PasswordInput, TextInput};
    use crate::{forms::bind_keys, theme::Theme};

    #[derive(Clone, Copy)]
    enum Mode {
        Password,
        Hidden,
        Plain,
    }

    struct Password {
        input: Entity<TextInput>,
        mode: Mode,
    }

    impl Render for Password {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            match self.mode {
                Mode::Password => div()
                    .child(PasswordInput::new(&self.input))
                    .into_any_element(),
                Mode::Hidden => div()
                    .debug_selector(|| "password-hidden".into())
                    .into_any_element(),
                Mode::Plain => div().child(Input::new(&self.input)).into_any_element(),
            }
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
    }

    fn configuration(
        input: &Entity<TextInput>,
        cx: &mut VisualTestContext,
    ) -> TextInputConfiguration {
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                EntityInputHandler::text_input_configuration(input, window, cx)
            })
        })
    }

    #[gpui::test]
    fn ordinary_text_state_becomes_secure_on_first_password_render(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            bind_keys(cx);
        });
        let (view, cx) = cx.add_window_view(|window, cx| Password {
            input: cx.new(|cx| TextInput::new(window, cx)),
            mode: Mode::Password,
        });
        let input = view.read_with(cx, |view, _| view.input.clone());
        cx.update(|window, _| window.activate_window());
        draw(cx);

        assert!(input.read_with(cx, |input, _| input.is_masked()));
        assert!(configuration(&input, cx).secure);

        let eye = cx
            .debug_bounds("password-visibility")
            .expect("password eye");
        cx.simulate_click(eye.center(), Modifiers::none());
        assert!(!input.read_with(cx, |input, _| input.is_masked()));
        assert!(configuration(&input, cx).secure);

        view.update(cx, |view, cx| {
            view.mode = Mode::Hidden;
            cx.notify();
        });
        draw(cx);
        view.update(cx, |view, cx| {
            view.mode = Mode::Password;
            cx.notify();
        });
        draw(cx);
        assert!(input.read_with(cx, |input, _| input.is_masked()));
        assert!(configuration(&input, cx).secure);

        view.update(cx, |view, cx| {
            view.mode = Mode::Plain;
            cx.notify();
        });
        draw(cx);
        assert!(!input.read_with(cx, |input, _| input.is_masked()));
        assert!(!configuration(&input, cx).secure);
    }

    #[gpui::test]
    fn explicit_masked_state_survives_plain_input_render(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            bind_keys(cx);
        });
        let (view, cx) = cx.add_window_view(|window, cx| Password {
            input: cx.new(|cx| TextInput::new(window, cx).masked()),
            mode: Mode::Password,
        });
        let input = view.read_with(cx, |view, _| view.input.clone());
        draw(cx);

        assert!(input.read_with(cx, |input, _| input.is_masked()));
        assert!(configuration(&input, cx).secure);
        view.update(cx, |view, cx| {
            view.mode = Mode::Plain;
            cx.notify();
        });
        draw(cx);
        assert!(input.read_with(cx, |input, _| input.is_masked()));
        assert!(configuration(&input, cx).secure);
    }

    #[gpui::test]
    fn explicitly_set_masked_state_survives_password_and_plain_roles(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            bind_keys(cx);
        });
        let (view, cx) = cx.add_window_view(|window, cx| {
            let input = cx.new(|cx| TextInput::new(window, cx));
            input.update(cx, |input, cx| input.set_masked(true, cx));
            Password {
                input,
                mode: Mode::Password,
            }
        });
        let input = view.read_with(cx, |view, _| view.input.clone());
        draw(cx);
        view.update(cx, |view, cx| {
            view.mode = Mode::Plain;
            cx.notify();
        });
        draw(cx);

        assert!(input.read_with(cx, |input, _| input.is_masked()));
        assert!(configuration(&input, cx).secure);
    }

    #[test]
    fn frame_tap_requests_keyboard_only_for_an_enabled_focused_field() {
        assert!(super::should_request_keyboard(true, false, false));
        assert!(!super::should_request_keyboard(false, false, false));
        assert!(!super::should_request_keyboard(true, true, false));
        assert!(!super::should_request_keyboard(true, false, true));
    }

    #[gpui::test]
    fn padding_tap_reopens_an_already_focused_field(cx: &mut TestAppContext) {
        cx.update(Theme::init);
        let (view, cx) = cx.add_window_view(|window, cx| Password {
            input: cx.new(|cx| TextInput::new(window, cx)),
            mode: Mode::Plain,
        });
        let input = view.read_with(cx, |view, _| view.input.clone());
        draw(cx);
        let root = cx.debug_bounds("input-root").expect("input frame");
        cx.simulate_click(root.center(), Modifiers::none());
        cx.update(|window, _| window.dismiss_virtual_keyboard());
        cx.simulate_click(
            point(root.left() + px(1.), root.center().y),
            Modifiers::none(),
        );
        assert!(cx.update(|window, cx| input.read(cx).focus().is_focused(window)));
    }
}
