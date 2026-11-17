mod actions;
mod edit;
mod element;
mod events;
mod geometry;
mod highlight;
mod ime;
mod owner;

mod state;

#[cfg(all(test, feature = "test-support"))]
mod password_security_tests;
use std::{ops::Range, rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Context, EventEmitter, FocusHandle, Focusable, KeyboardType, Pixels, Point,
    SharedString, Subscription, Task, TextInputConfiguration, Window, WrappedLine,
};

pub(crate) use actions::{
    Backspace, Copy, Cut, Delete, DeleteWordLeft, Down, Enter, Redo, Submit, Undo, Up, bind_keys,
};
pub use edit::History;
use edit::Snapshot;
pub(crate) use edit::{from_utf16, to_utf16};
pub use events::InputEvent;
use events::{Filter, Highlighter};
pub use highlight::Highlight;
pub use owner::{TextInputOwner, set_input_owner_reporter};

pub(crate) type Fit = Rc<dyn Fn(&str, Range<usize>, &str) -> (String, usize)>;

/// The last shaped text: one entry per hard line, with its display offset.
pub(crate) struct Layout {
    pub lines: Rc<[(usize, WrappedLine)]>,
    pub bounds: Bounds<Pixels>,
    pub line_height: Pixels,
    /// The lines show the placeholder, not content.
    pub placeholder: bool,
}

pub(super) const BULLET: char = '•';
pub(super) const BLINK: Duration = Duration::from_millis(530);

/// Editable text: one line or many, with selection, undo, IME and clipboard.
pub struct TextInput {
    focus: FocusHandle,
    owner_activation: u64,
    owner_observer: Option<Rc<dyn Fn(TextInputOwner)>>,
    text: String,
    placeholder: SharedString,
    label: SharedString,
    selection: Range<usize>,
    reversed: bool,
    marked: Option<Range<usize>>,
    composing: Option<Snapshot>,
    rows: Option<(usize, usize)>,
    masked: bool,
    input_configuration: TextInputConfiguration,
    secure: bool,
    explicit_secure: bool,
    secure_owner: bool,
    filter: Option<Filter>,
    fit: Option<Fit>,
    max_len: Option<usize>,
    keyboard_type: KeyboardType,
    reveal_bounds: Option<Bounds<Pixels>>,
    highlighter: Option<Highlighter>,
    disabled: bool,
    history: History<Snapshot>,
    /// The selection the last edit replaced.
    edited_from: Range<usize>,
    /// Edits applied, text changed or not.
    edits: u64,
    pub(crate) layout: Option<Layout>,
    pub(crate) scroll: Point<Pixels>,
    selecting: bool,
    caret_on: bool,
    /// Blink runs only while focused, or unseen carets redraw forever.
    focused: bool,
    blink_epoch: u64,
    _blink: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<InputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let subscriptions = vec![
            cx.on_focus(&focus, window, |input, _, cx| {
                input.advance_owner_activation();
                input.focused = true;
                input.restart_blink(cx);
                cx.emit(InputEvent::Focus);
            }),
            cx.on_blur(&focus, window, |input, window, cx| {
                input.advance_owner_activation();
                input.focused = false;
                input.selecting = false;
                gpui::EntityInputHandler::unmark_text(input, window, cx);
                input._blink = None;
                cx.emit(InputEvent::Blur);
                cx.notify();
            }),
        ];
        Self {
            focus,
            owner_activation: 0,
            owner_observer: None,
            text: String::new(),
            placeholder: SharedString::default(),
            label: SharedString::default(),
            selection: 0..0,
            reversed: false,
            marked: None,
            composing: None,
            rows: None,
            masked: false,
            input_configuration: TextInputConfiguration::default(),
            secure: false,
            explicit_secure: false,
            secure_owner: false,
            filter: None,
            fit: None,
            max_len: None,
            keyboard_type: KeyboardType::Default,
            reveal_bounds: None,
            highlighter: None,
            disabled: false,
            history: History::default(),
            edited_from: 0..0,
            edits: 0,
            layout: None,
            scroll: Point::default(),
            selecting: false,
            caret_on: true,
            focused: false,
            blink_epoch: 0,
            _blink: None,
            _subscriptions: subscriptions,
        }
    }

    /// Several lines: grows from `min` rows to `max`, then scrolls.
    pub fn multi_line(mut self, min: usize, max: usize) -> Self {
        assert!(0 < min && min <= max, "text rows {min}..={max} are empty");
        self.rows = Some((min, max));
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// The name assistive technology reads; the placeholder otherwise.
    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = text.into();
        self
    }

    /// Shows bullets instead of the text, and keeps it off the clipboard.
    pub fn masked(mut self) -> Self {
        self.masked = true;
        self.secure = true;
        self.explicit_secure = true;
        self
    }

    /// Configure supported software keyboard hints (excludes keyboard type and secure-entry).
    pub fn input_configuration(mut self, configuration: TextInputConfiguration) -> Self {
        self.input_configuration = configuration;
        self
    }

    /// Update software keyboard hints for this field.
    pub fn set_input_configuration(
        &mut self,
        configuration: TextInputConfiguration,
        cx: &mut Context<Self>,
    ) {
        if self.input_configuration != configuration {
            self.input_configuration = configuration;
            cx.notify();
        }
    }

    /// Drops typed or pasted characters `allow` rejects.
    pub fn filter(mut self, allow: impl Fn(char) -> bool + 'static) -> Self {
        self.filter = Some(Rc::new(allow));
        self
    }

    /// Longest text, in characters.
    pub fn max_len(mut self, len: usize) -> Self {
        self.max_len = Some(len);
        self
    }

    /// The software keyboard to request when this input gains focus.
    pub fn keyboard_type(mut self, keyboard_type: KeyboardType) -> Self {
        self.keyboard_type = keyboard_type;
        self
    }

    /// Colors spans of the text as it changes.
    pub fn highlighter(
        mut self,
        highlight: impl Fn(&str, &App) -> Vec<(Range<usize>, Highlight)> + 'static,
    ) -> Self {
        self.highlighter = Some(Rc::new(highlight));
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The text as committed: while an input method composes, the text before it began.
    pub(crate) fn committed(&self) -> &str {
        self.composing
            .as_ref()
            .map_or(&self.text, |start| &start.text)
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn is_masked(&self) -> bool {
        self.masked
    }

    pub fn is_multi_line(&self) -> bool {
        self.rows.is_some()
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn rows(&self) -> Option<(usize, usize)> {
        self.rows
    }

    pub fn placeholder_text(&self) -> &SharedString {
        &self.placeholder
    }

    pub fn label_text(&self) -> &SharedString {
        &self.label
    }

    pub fn selection(&self) -> Range<usize> {
        self.selection.clone()
    }

    pub fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    pub fn set_placeholder(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = text.into();
        cx.notify();
    }

    pub fn set_label(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.label = text.into();
        cx.notify();
    }

    pub fn set_masked(&mut self, masked: bool, cx: &mut Context<Self>) {
        self.masked = masked;
        if masked {
            self.secure = true;
            self.explicit_secure = true;
        }
        cx.notify();
    }

    pub(crate) fn enter_secure(&mut self, cx: &mut Context<Self>) {
        self.secure_owner = true;
        if !self.secure {
            self.secure = true;
            self.masked = true;
            cx.notify();
        }
    }

    pub(crate) fn remask_secure(&mut self, cx: &mut Context<Self>) {
        self.secure_owner = true;
        if !self.secure || !self.masked {
            self.secure = true;
            self.masked = true;
            cx.notify();
        }
    }

    pub(crate) fn set_revealed(&mut self, revealed: bool, cx: &mut Context<Self>) {
        if self.masked == revealed {
            self.masked = !revealed;
            cx.notify();
        }
    }

    pub(crate) fn leave_secure(&mut self, cx: &mut Context<Self>) {
        if self.secure_owner {
            self.secure_owner = false;
            self.secure = self.explicit_secure;
            self.masked = self.explicit_secure;
            cx.notify();
        }
    }

    pub(crate) fn set_keyboard_type(
        &mut self,
        keyboard_type: KeyboardType,
        cx: &mut Context<Self>,
    ) {
        if self.keyboard_type != keyboard_type {
            self.keyboard_type = keyboard_type;
            cx.notify();
        }
    }

    pub(crate) fn reveal_bounds(&self) -> Option<Bounds<Pixels>> {
        self.reveal_bounds
    }

    pub(crate) fn set_reveal_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.reveal_bounds = Some(bounds);
    }

    pub(crate) fn clear_reveal_bounds(&mut self) {
        self.reveal_bounds = None;
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if !self.disabled && disabled {
            self.finish_composition(cx);
        }
        if self.disabled != disabled {
            self.advance_owner_activation();
        }
        self.disabled = disabled;
        self.focus = self.focus.clone().tab_stop(!disabled);
        cx.notify();
    }

    /// Replaces all text and puts the caret at its end. Not an undo step.
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        let text = self.admit(&text.into(), 0);
        let text = match &self.fit {
            Some(fit) => {
                let base = self.committed();
                fit(base, 0..base.len(), &text).0
            }
            None => text,
        };
        self.selection = text.len()..text.len();
        self.text = text;
        self.marked = None;
        self.composing = None;
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    /// Reshapes every edit, as a mask does: text, replaced range and typed text in; text and caret out. Undo starts afresh, since earlier text need not fit.
    pub(crate) fn set_fit(&mut self, fit: Fit) {
        self.fit = Some(fit);
        self.history = History::default();
    }

    /// Replaces the selection, as typing would. An undo step.
    pub fn insert(&mut self, text: &str, cx: &mut Context<Self>) {
        self.replace(self.selection.clone(), text, false, cx);
    }

    /// Selects `range`; one outside the text or inside a character is logged and left.
    pub fn select(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        let fits = range.start <= range.end
            && self.text.is_char_boundary(range.start)
            && self.text.is_char_boundary(range.end);
        if !fits {
            log::error!(
                "text input: no selection {range:?} in {} bytes",
                self.text.len()
            );
            return;
        }
        self.selection = range;
        self.reversed = false;
        self.restart_blink(cx);
    }
}
