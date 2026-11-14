use std::{ops::Range, rc::Rc};

use gpui::App;

use super::Highlight;

/// What a text input tells its owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputEvent {
    Changed,
    /// Enter in one line, or the submit key in several.
    Submit,
    Focus,
    Blur,
}

pub(super) type Highlighter = Rc<dyn Fn(&str, &App) -> Vec<(Range<usize>, Highlight)>>;
pub(super) type Filter = Rc<dyn Fn(char) -> bool>;
