use std::ops::Range;

use gpui::{App, Context, FocusHandle};

use super::{BLINK, BULLET, Highlight, InputEvent, Snapshot, TextInput, edit};
use crate::theme::ActiveTheme;

impl TextInput {
    /// Keeps what `filter` and `max_len` allow of `incoming`.
    pub(super) fn admit(&self, incoming: &str, keep: usize) -> String {
        let allowed: String = incoming
            .chars()
            .filter(|ch| self.filter.as_ref().is_none_or(|allow| allow(*ch)))
            .collect();
        match self.max_len {
            Some(max) => allowed.chars().take(max.saturating_sub(keep)).collect(),
            None => allowed,
        }
    }

    /// The selection the last edit replaced, for an owner's undo to go back to.
    pub(crate) fn edited_from(&self) -> Range<usize> {
        self.edited_from.clone()
    }

    /// How many edits it took, and whether an input method is composing.
    pub(crate) fn edits(&self) -> (u64, bool) {
        (self.edits, self.marked.is_some())
    }

    pub(crate) fn replace(
        &mut self,
        range: Range<usize>,
        incoming: &str,
        typing: bool,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        self.edited_from = self.selection.clone();
        let incoming = if self.rows.is_none() {
            incoming.replace(['\n', '\r'], " ")
        } else {
            incoming.replace("\r\n", "\n")
        };
        let kept = self.text.chars().count() - self.text[range.clone()].chars().count();
        let incoming = self.admit(&incoming, kept);
        if incoming.is_empty() && range.is_empty() {
            return;
        }
        self.edits += 1;
        let composed = self.composing.take();
        let before = composed.clone().unwrap_or_else(|| self.snapshot());
        let caret = match &self.fit {
            Some(fit) => {
                let (base, range) = match &composed {
                    Some(start) => (start.text.as_str(), start.selection.clone()),
                    None => (self.text.as_str(), range),
                };
                let (text, caret) = fit(base, range, &incoming);
                self.text = text;
                caret
            }
            None => {
                self.text.replace_range(range.clone(), &incoming);
                range.start + incoming.len()
            }
        };
        self.selection = caret..caret;
        self.reversed = false;
        self.marked = None;
        self.commit(before, typing, cx);
        self.restart_blink(cx);
    }

    /// Makes `before` one undo step, if the text moved on from it.
    fn commit(&mut self, before: Snapshot, typing: bool, cx: &mut Context<Self>) {
        if before.text != self.text {
            self.history.record(before, typing);
            cx.emit(InputEvent::Changed);
        }
    }

    /// The state as committed, so undo never keeps an input method's text before it commits.
    pub(super) fn snapshot(&self) -> Snapshot {
        self.composing.clone().unwrap_or_else(|| Snapshot {
            text: self.text.clone(),
            selection: self.selection.clone(),
        })
    }

    pub(crate) fn restore(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.text = snapshot.text;
        self.selection = snapshot.selection;
        self.reversed = false;
        self.marked = None;
        self.composing = None;
        cx.emit(InputEvent::Changed);
        self.restart_blink(cx);
    }

    pub(crate) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selection = offset..offset;
        self.reversed = false;
        self.restart_blink(cx);
    }

    pub(crate) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.reversed {
            self.selection.start = offset;
        } else {
            self.selection.end = offset;
        }
        if self.selection.end < self.selection.start {
            self.reversed = !self.reversed;
            self.selection = self.selection.end..self.selection.start;
        }
        self.restart_blink(cx);
    }

    /// Shows the caret now and restarts its blink.
    pub(super) fn restart_blink(&mut self, cx: &mut Context<Self>) {
        self.caret_on = true;
        self.blink_epoch += 1;
        let epoch = self.blink_epoch;
        let still = cx.theme().reduced_motion;
        self._blink = (!still && self.focused).then(|| {
            cx.spawn(async move |input, cx| {
                loop {
                    cx.background_executor().timer(BLINK).await;
                    let alive = input.update(cx, |input, cx| {
                        if input.blink_epoch == epoch {
                            input.caret_on = !input.caret_on;
                            cx.notify();
                        }
                    });
                    if alive.is_err() {
                        return;
                    }
                }
            })
        });
        cx.notify();
    }

    /// What the element shows: bullets when masked.
    pub(crate) fn display_text(&self) -> String {
        if self.masked {
            BULLET.to_string().repeat(self.grapheme_count())
        } else {
            self.text.clone()
        }
    }

    fn grapheme_count(&self) -> usize {
        unicode_segmentation::UnicodeSegmentation::graphemes(self.text.as_str(), true).count()
    }

    pub(crate) fn display_offset(&self, offset: usize) -> usize {
        if self.masked {
            edit::masked_offset(&self.text, offset, BULLET)
        } else {
            offset
        }
    }

    pub(crate) fn content_offset(&self, display: usize) -> usize {
        if self.masked {
            edit::unmasked_offset(&self.text, display, BULLET)
        } else {
            display
        }
    }

    pub(crate) fn highlights(&self, cx: &App) -> Vec<(Range<usize>, Highlight)> {
        match (&self.highlighter, self.masked) {
            (Some(highlight), false) => highlight(&self.text, cx),
            _ => Vec::new(),
        }
    }

    pub(crate) fn marked(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub(crate) fn selecting(&self) -> bool {
        self.selecting
    }

    pub(crate) fn caret_on(&self) -> bool {
        self.caret_on
    }

    pub(crate) fn focus(&self) -> &FocusHandle {
        &self.focus
    }
}
