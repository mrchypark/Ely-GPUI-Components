use std::time::Duration;

use gpui::{
    Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

use super::{across, settle, setup};
use crate::media::{AudioWaveform, Scrubber};
use crate::theme::Theme;

const SECOND: Duration = Duration::from_secs(1);

/// The theme a finger finds: touch mode on, nothing moving.
fn touch(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| Theme::update(cx, |theme| theme.touch_mode = true));
}

/// A waveform of a hundred seconds, and the times it was sent to.
struct Waving {
    at: Duration,
    sought: Vec<Duration>,
}

impl Render for Waving {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let peaks: Vec<f32> = (0..120).map(|ix| (ix % 7) as f32 / 7.0).collect();
        div().w(px(400.0)).mt(px(40.0)).child(
            AudioWaveform::new("wave", peaks, SECOND * 100, self.at).on_seek(move |time, _, cx| {
                owner.update(cx, |view, cx| {
                    view.at = time;
                    view.sought.push(time);
                    cx.notify();
                })
            }),
        )
    }
}

fn waving(cx: &mut TestAppContext) -> (Entity<Waving>, &mut VisualTestContext) {
    touch(cx);
    let (view, cx) = cx.add_window_view(|_, _| Waving {
        at: Duration::ZERO,
        sought: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_press_alone_puts_the_tip_over_the_waveform(cx: &mut TestAppContext) {
    let (view, cx) = waving(cx);
    assert!(
        cx.debug_bounds("time-tip").is_none(),
        "no tip before the press"
    );

    let at = across("audio-waveform", 0.25, cx);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    let first_tip = cx
        .debug_bounds("time-tip")
        .expect("a press alone shows the tip");
    assert!(
        first_tip.left() <= at.x && at.x <= first_tip.right(),
        "{first_tip:?} under the press at {at:?}"
    );
    let sought = view.read_with(cx, |view, _| view.sought.clone());
    let first = sought.first().expect("the press seeks");
    assert!(first.abs_diff(SECOND * 25) < SECOND, "{sought:?}");

    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("time-tip").is_some(),
        "the tip stays after the finger lifts"
    );

    let later = across("audio-waveform", 0.75, cx);
    cx.simulate_mouse_down(later, MouseButton::Left, Modifiers::none());
    settle(cx);

    let tip = cx.debug_bounds("time-tip").expect("the tip stays");
    assert!(
        tip.left() <= later.x && later.x <= tip.right(),
        "{tip:?} under the press at {later:?}"
    );
    assert!(
        tip.left() > first_tip.left() + px(50.0),
        "the tip follows the press: {tip:?} after {first_tip:?}"
    );
}

/// A scrubber of two hundred seconds, and the times it was sent to.
struct Scrubbing {
    at: Duration,
    sought: Vec<Duration>,
}

impl Render for Scrubbing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(400.0)).mt(px(40.0)).child(
            Scrubber::new("scrub", SECOND * 200, self.at).on_seek(move |time, _, cx| {
                owner.update(cx, |view, cx| {
                    view.at = time;
                    view.sought.push(time);
                    cx.notify();
                })
            }),
        )
    }
}

fn scrubbing(cx: &mut TestAppContext) -> (Entity<Scrubbing>, &mut VisualTestContext) {
    touch(cx);
    let (view, cx) = cx.add_window_view(|_, _| Scrubbing {
        at: Duration::ZERO,
        sought: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_press_alone_puts_the_tip_over_the_scrubber(cx: &mut TestAppContext) {
    let (view, cx) = scrubbing(cx);
    assert!(
        cx.debug_bounds("time-tip").is_none(),
        "no tip before the press"
    );

    let at = across("scrubber-rail", 0.25, cx);
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    let tip = cx
        .debug_bounds("time-tip")
        .expect("a press alone shows the tip");
    assert!(
        tip.left() <= at.x && at.x <= tip.right(),
        "{tip:?} under the press at {at:?}"
    );
    let sought = view.read_with(cx, |view, _| view.sought.clone());
    let first = sought.first().expect("the press seeks");
    assert!(first.abs_diff(SECOND * 50) < SECOND * 2, "{sought:?}");
}
