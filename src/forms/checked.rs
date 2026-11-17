use std::sync::LazyLock;

use gpui::{
    App, Entity, IntoElement, KeyboardType, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};
use regex::Regex;

use super::{Input, TextInput};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").expect("email pattern compiles"));
static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^https?://[^\s/?#]+\.[^\s/?#]+\S*$").expect("url pattern compiles")
});

/// Whether `text` reads as one email address.
pub fn is_email(text: &str) -> bool {
    EMAIL.is_match(text)
}

/// Whether `text` reads as a web link.
pub fn is_url(text: &str) -> bool {
    URL.is_match(text)
}

/// A field checked once focus leaves it; a note explains a miss.
fn checked(
    state: &Entity<TextInput>,
    icon: IconName,
    valid: fn(&str) -> bool,
    note: &'static str,
    size: ControlSize,
    window: &Window,
    cx: &App,
) -> impl IntoElement + use<> {
    let input = state.read(cx);
    let miss = !input.focus().is_focused(window) && !input.is_empty() && !valid(input.text());
    let theme = cx.theme();
    let (subtle, danger) = (theme.colors.fg_subtle, theme.colors.danger);
    div()
        .debug_selector(|| "checked-root".into())
        .flex()
        .flex_col()
        .gap_1()
        .child(
            Input::new(state)
                .size(size)
                .invalid(miss)
                .prefix(Icon::new(icon).size(IconSize::Sm).color(subtle)),
        )
        .when(miss, |field| {
            field.child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(danger)
                    .child(SharedString::from(note)),
            )
        })
}

/// An email field, checked when focus leaves it.
#[derive(IntoElement)]
pub struct EmailInput {
    state: Entity<TextInput>,
    size: ControlSize,
}

impl EmailInput {
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

impl RenderOnce for EmailInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |input, cx| {
            input.set_keyboard_type(KeyboardType::EmailAddress, cx)
        });
        checked(
            &self.state,
            IconName::AtSign,
            is_email,
            "Enter an address like name@example.com",
            self.size,
            window,
            cx,
        )
    }
}

/// A link field, checked when focus leaves it.
#[derive(IntoElement)]
pub struct UrlInput {
    state: Entity<TextInput>,
    size: ControlSize,
}

impl UrlInput {
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

impl RenderOnce for UrlInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |input, cx| {
            input.set_keyboard_type(KeyboardType::URL, cx)
        });
        checked(
            &self.state,
            IconName::Link,
            is_url,
            "Enter a link that starts with https://",
            self.size,
            window,
            cx,
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext, Context, Entity, EntityInputHandler, IntoElement, KeyboardType, Render,
        TestAppContext, VisualTestContext, Window,
    };

    use super::{EmailInput, UrlInput, is_email, is_url};
    use crate::{forms::TextInput, theme::Theme};

    #[derive(Clone, Copy)]
    enum Hint {
        Email,
        Url,
    }

    struct Reused {
        input: Entity<TextInput>,
        hint: Hint,
    }

    impl Render for Reused {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            match self.hint {
                Hint::Email => EmailInput::new(&self.input).into_any_element(),
                Hint::Url => UrlInput::new(&self.input).into_any_element(),
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

    fn requested_keyboard(input: &Entity<TextInput>, cx: &mut VisualTestContext) -> KeyboardType {
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                EntityInputHandler::text_input_configuration(input, window, cx)
            })
        })
        .keyboard_type
    }

    #[test]
    fn emails_and_urls() {
        assert!(is_email("ada@example.com"));
        assert!(!is_email("ada@example"));
        assert!(!is_email("ada example.com"));
        assert!(is_url("https://ely.dev/docs?x=1"));
        assert!(is_url("http://localhost.test"));
        assert!(!is_url("ely.dev"));
        assert!(!is_url("https://nodot"));
    }

    #[gpui::test]
    fn reused_email_and_url_state_follow_focus_and_wrapper(cx: &mut TestAppContext) {
        cx.update(|cx| {
            Theme::init(cx);
            crate::forms::bind_keys(cx);
        });
        let (view, cx) = cx.add_window_view(|window, cx| Reused {
            input: cx.new(|cx| TextInput::new(window, cx)),
            hint: Hint::Email,
        });
        cx.update(|window, _| window.activate_window());
        draw(cx);
        cx.update(|window, cx| window.focus_next(cx));
        draw(cx);
        let input = view.read_with(cx, |view, _| view.input.clone());
        assert_eq!(requested_keyboard(&input, cx), KeyboardType::EmailAddress);

        view.update(cx, |view, cx| {
            view.hint = Hint::Url;
            cx.notify();
        });
        draw(cx);
        assert_eq!(requested_keyboard(&input, cx), KeyboardType::URL);

        cx.update(|window, cx| {
            window.blur(cx);
            window.focus_next(cx);
        });
        draw(cx);
        assert_eq!(requested_keyboard(&input, cx), KeyboardType::URL);
    }
}
