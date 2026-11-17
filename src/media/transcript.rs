use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::scrubber::OnTime;
use crate::{
    buttons::{Button, ButtonVariant},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

type OnSwap = Rc<dyn Fn(&mut Window, &mut App)>;

/// Which party spoke a diarized transcript turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeakerRole {
    Me,
    Other,
}

/// One spoken word with its `[start_ms, end_ms]` interval in milliseconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimedWord {
    pub text: SharedString,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl TimedWord {
    pub fn new(text: impl Into<SharedString>, start_ms: u64, end_ms: u64) -> Self {
        Self {
            text: text.into(),
            start_ms,
            end_ms,
        }
    }

    fn is_active(&self, at_ms: u64) -> bool {
        if self.start_ms == self.end_ms {
            at_ms == self.start_ms
        } else {
            at_ms >= self.start_ms && at_ms < self.end_ms
        }
    }
}

/// One speaker turn in a diarized transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranscriptTurn {
    pub speaker_label: SharedString,
    pub role: SpeakerRole,
    pub words: Vec<TimedWord>,
}

impl TranscriptTurn {
    pub fn new(
        speaker_label: impl Into<SharedString>,
        role: SpeakerRole,
        words: Vec<TimedWord>,
    ) -> Self {
        Self {
            speaker_label: speaker_label.into(),
            role,
            words,
        }
    }
}

fn format_turn_time(ms: u64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Speaker-diarized transcript with word-level playhead highlighting and tap-to-seek.
#[derive(IntoElement)]
pub struct TranscriptView {
    id: ElementId,
    turns: Vec<TranscriptTurn>,
    at: Duration,
    on_seek: Option<OnTime>,
    on_swap_speakers: Option<OnSwap>,
}

impl TranscriptView {
    pub fn new(id: impl Into<ElementId>, turns: impl IntoIterator<Item = TranscriptTurn>) -> Self {
        Self {
            id: id.into(),
            turns: turns.into_iter().collect(),
            at: Duration::ZERO,
            on_seek: None,
            on_swap_speakers: None,
        }
    }

    /// Current playback position used to highlight the active word.
    pub fn at(mut self, at: Duration) -> Self {
        self.at = at;
        self
    }

    /// Called when the user clicks a word or turn timestamp to seek playback.
    pub fn on_seek(mut self, handler: impl Fn(Duration, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }

    /// Optional hook to swap speaker roles (e.g. Me <-> Other).
    pub fn on_swap_speakers(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_swap_speakers = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TranscriptView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let at_ms = u64::try_from(self.at.as_millis()).unwrap_or(u64::MAX);
        let root_id = self.id.clone();

        let header = self.on_swap_speakers.map(|on_swap| {
            div().flex().items_center().justify_end().child(
                Button::new((root_id.clone(), "swap-speakers"), "Swap speakers")
                    .debug_selector(|| "transcript-swap-speakers".into())
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .icon(IconName::Repeat)
                    .on_click(move |_, window, cx| on_swap(window, cx)),
            )
        });

        let turns = self.turns.into_iter().enumerate().map(|(turn_ix, turn)| {
            let is_me = turn.role == SpeakerRole::Me;
            let first_ms = turn.words.first().map(|w| w.start_ms);
            let on_seek_turn = self.on_seek.clone();

            let timestamp = first_ms.map(|ms| {
                let target = Duration::from_millis(ms);
                div()
                    .id((root_id.clone(), format!("turn-time-{turn_ix}")))
                    .debug_selector(move || format!("transcript-turn-time-{turn_ix}"))
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .when(on_seek_turn.is_some(), |el| el.cursor_pointer())
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(on_seek_turn, |el, seek| {
                        el.on_click(move |_, window, cx| seek(target, window, cx))
                    })
                    .child(format_turn_time(ms))
            });

            let words = turn.words.into_iter().enumerate().map(|(word_ix, word)| {
                let active = word.is_active(at_ms);
                let target = Duration::from_millis(word.start_ms);
                let on_seek_word = self.on_seek.clone();
                div()
                    .id((root_id.clone(), format!("word-{turn_ix}-{word_ix}")))
                    .debug_selector(move || {
                        if active {
                            "transcript-active-word".into()
                        } else {
                            format!("transcript-word-{turn_ix}-{word_ix}")
                        }
                    })
                    .px_1()
                    .rounded(theme.radius(Radius::Sm))
                    .when(active, |el| {
                        el.bg(colors.accent)
                            .text_color(colors.on_accent)
                            .font_weight(FontWeight::MEDIUM)
                    })
                    .when(!active, |el| el.text_color(colors.fg))
                    .when(on_seek_word.is_some(), |el| el.cursor_pointer())
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(on_seek_word, |el, seek| {
                        el.on_click(move |_, window, cx| seek(target, window, cx))
                    })
                    .child(word.text)
            });

            let bubble = div()
                .max_w_4_5()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded(theme.radius(Radius::Lg))
                .border_1()
                .border_color(colors.border)
                .bg(if is_me { colors.sunken } else { colors.surface })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.fg_muted)
                                .child(turn.speaker_label),
                        )
                        .children(timestamp),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_x_0p5()
                        .gap_y_0p5()
                        .text_size(theme.text_size(TextSize::Sm))
                        .children(words),
                );

            div()
                .debug_selector(move || {
                    if is_me {
                        format!("transcript-turn-me-{turn_ix}")
                    } else {
                        format!("transcript-turn-other-{turn_ix}")
                    }
                })
                .w_full()
                .flex()
                .flex_col()
                .when(is_me, |row| row.items_end())
                .when(!is_me, |row| row.items_start())
                .child(bubble)
        });

        div()
            .id(self.id)
            .debug_selector(|| "transcript-view".into())
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .children(header)
            .children(turns)
    }
}
