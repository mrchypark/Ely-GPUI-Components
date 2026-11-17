use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext, Window, div, px,
};

use super::super::{SpeakerRole, TimedWord, TranscriptTurn, TranscriptView};
use super::{settle, setup};

struct Host {
    at: Duration,
    seeked: Vec<Duration>,
    swapped: usize,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let seek_view = cx.entity();
        let swap_view = cx.entity();
        let turns = [
            TranscriptTurn::new(
                "Alex",
                SpeakerRole::Other,
                vec![
                    TimedWord::new("Hello", 0, 500),
                    TimedWord::new("there", 500, 1000),
                ],
            ),
            TranscriptTurn::new(
                "Me",
                SpeakerRole::Me,
                vec![TimedWord::new("Hi!", 1000, 1600)],
            ),
        ];
        div().w(px(360.)).h(px(300.)).child(
            TranscriptView::new("transcript", turns)
                .at(self.at)
                .on_seek(move |pos, _, cx| {
                    seek_view.update(cx, |host, _| host.seeked.push(pos));
                })
                .on_swap_speakers(move |_, cx| {
                    swap_view.update(cx, |host, _| host.swapped += 1);
                }),
        )
    }
}

#[gpui::test]
fn transcript_view_highlights_active_word_seeks_and_swaps(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Host {
        at: Duration::from_millis(650),
        seeked: Vec::new(),
        swapped: 0,
    });
    settle(cx);

    let other_turn = cx
        .debug_bounds("transcript-turn-other-0")
        .expect("other turn");
    let me_turn = cx.debug_bounds("transcript-turn-me-1").expect("me turn");
    assert_eq!(other_turn.origin.x, me_turn.origin.x);

    let active_word = cx
        .debug_bounds("transcript-active-word")
        .expect("active word at 650ms");
    cx.simulate_click(active_word.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        host.read_with(cx, |host, _| host.seeked.clone()),
        vec![Duration::from_millis(500)]
    );

    let first_word = cx.debug_bounds("transcript-word-0-0").expect("first word");
    cx.simulate_click(first_word.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        host.read_with(cx, |host, _| host.seeked.clone()),
        vec![Duration::from_millis(500), Duration::ZERO]
    );

    let swap = cx
        .debug_bounds("transcript-swap-speakers")
        .expect("swap speakers button");
    cx.simulate_click(swap.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(host.read_with(cx, |host, _| host.swapped), 1);
}
