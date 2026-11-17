use std::rc::Rc;

use gpui::{App, Context, EntityId, Global, Window, WindowId};

use super::TextInput;

/// Logical editor identity reported by the handler being queried, not by paint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextInputOwner {
    /// Window lifetime; the backend must invalidate ownership when it closes.
    pub window: WindowId,
    /// Entity lifetime, including its generation; never a handler address.
    pub editor: EntityId,
    /// Changes on observed focus/blur and disabled transitions, not repaint.
    pub activation: u64,
    /// Current focus, even before the next focus notification is delivered.
    pub focused: bool,
    /// Editability metadata; the acceptance return value remains unchanged.
    pub disabled: bool,
}

struct InputOwnerReporter(Rc<dyn Fn(TextInputOwner)>);

impl Global for InputOwnerReporter {}

/// Set the app-wide owner reporter; per-instance observers override it. Call at startup; later calls replace it.
pub fn set_input_owner_reporter(reporter: impl Fn(TextInputOwner) + 'static, cx: &mut App) {
    cx.set_global(InputOwnerReporter(Rc::new(reporter)));
}

impl TextInput {
    /// Install before focus changes; report outside App borrows. Do not reenter GPUI, notify, or retain this entity.
    pub fn input_owner_observer(mut self, observer: impl Fn(TextInputOwner) + 'static) -> Self {
        assert!(
            self.owner_observer.is_none(),
            "input owner observer already installed"
        );
        assert_eq!(
            self.owner_activation, 0,
            "install input owner observer before activation"
        );
        self.owner_observer = Some(Rc::new(observer));
        self
    }

    pub(super) fn advance_owner_activation(&mut self) {
        self.owner_activation = self
            .owner_activation
            .checked_add(1)
            .expect("text input owner activation exhausted");
    }

    pub(super) fn report_input_owner(&self, window: &Window, cx: &Context<Self>) {
        let owner = TextInputOwner {
            window: window.window_handle().window_id(),
            editor: cx.entity_id(),
            activation: self.owner_activation,
            focused: self.focus.is_focused(window),
            disabled: self.disabled,
        };
        if let Some(observer) = &self.owner_observer {
            observer(owner);
        } else if let Some(reporter) = cx.try_global::<InputOwnerReporter>() {
            (reporter.0)(owner);
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
