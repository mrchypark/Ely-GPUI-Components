use std::rc::Rc;

use gpui::{Context, EntityId, Window, WindowId};

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

impl TextInput {
    /// Installs once during construction, before focus or disabled transitions.
    /// The UI-thread callback only reports into the host's scoped owner query.
    /// The host ignores reports outside that scope and rejects missing reports.
    /// Query outside App borrows; never query synchronously from handler install.
    /// It must not reenter GPUI, notify, or retain this entity through a cycle.
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
        if let Some(observer) = &self.owner_observer {
            observer(TextInputOwner {
                window: window.window_handle().window_id(),
                editor: cx.entity_id(),
                activation: self.owner_activation,
                focused: self.focus.is_focused(window),
                disabled: self.disabled,
            });
        }
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
