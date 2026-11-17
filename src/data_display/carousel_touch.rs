//! The carousel on a phone: an arrow has no hover to show it, so it stays out and keeps a finger-sized hit area.

use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, px,
};

use super::Carousel;
use crate::{primitives::Measure, theme::Theme};

/// A three-slide carousel, 200 wide, that reports where each slide sits.
struct Deck(Rc<RefCell<[Pixels; 3]>>);

impl Render for Deck {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let slides = (0..3).map(|ix| {
            let seen = self.0.clone();
            Measure::new(("slide", ix), move |bounds, _, _| {
                seen.borrow_mut()[ix] = bounds.origin.x
            })
            .size_full()
        });
        Carousel::new("deck")
            .w(px(200.0))
            .h(px(100.0))
            .children(slides)
    }
}

/// A deck that advances on its own, for pointer handling checks.
struct AutoplayDeck(Rc<RefCell<[Pixels; 3]>>);

impl Render for AutoplayDeck {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let slides = (0..3).map(|ix| {
            let seen = self.0.clone();
            Measure::new(("slide", ix), move |bounds, _, _| {
                seen.borrow_mut()[ix] = bounds.origin.x
            })
            .size_full()
        });
        Carousel::new("deck")
            .autoplay(Duration::from_millis(50))
            .w(px(200.0))
            .h(px(100.0))
            .children(slides)
    }
}

/// Frames 2ms apart, past reduced motion's 1ms folds.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        std::thread::sleep(Duration::from_millis(2));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// The slide in view: the one at the viewport's left edge, inside its border.
fn front(places: &Rc<RefCell<[Pixels; 3]>>) -> Option<usize> {
    places.borrow().iter().position(|x| *x == px(1.0))
}

/// A deck on a phone: touch mode on, motion stilled so the slide lands at once.
fn deck(cx: &mut TestAppContext) -> (Rc<RefCell<[Pixels; 3]>>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let places = Rc::new(RefCell::new([Pixels::ZERO; 3]));
    let seen = places.clone();
    let (_, cx) = cx.add_window_view(|_, _| Deck(seen));
    settle(cx);
    (places, cx)
}

fn autoplay_deck(
    touch_mode: bool,
    cx: &mut TestAppContext,
) -> (Rc<RefCell<[Pixels; 3]>>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = touch_mode;
            theme.reduced_motion = true;
        });
    });
    let places = Rc::new(RefCell::new([Pixels::ZERO; 3]));
    let seen = places.clone();
    let (_, cx) = cx.add_window_view(|_, _| AutoplayDeck(seen));
    settle(cx);
    (places, cx)
}

fn wait_for_autoplay(cx: &mut VisualTestContext) {
    for _ in 0..4 {
        // The ticker runs on the test clock, not the wall clock.
        cx.executor().advance_clock(Duration::from_millis(30));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

#[gpui::test]
fn a_carousel_arrow_keeps_a_48_pixel_hit_area_on_touch(cx: &mut TestAppContext) {
    let (_, cx) = deck(cx);
    for name in ["carousel-arrow-back", "carousel-arrow-next"] {
        let bounds = cx.debug_bounds(name).expect(name);
        assert!(
            bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
            "{name} is {bounds:?}"
        );
    }
}

#[gpui::test]
fn a_tap_on_the_next_arrow_turns_the_carousel(cx: &mut TestAppContext) {
    let (places, cx) = deck(cx);
    assert_eq!(front(&places), Some(0), "the first slide shows");
    let arrow = cx
        .debug_bounds("carousel-arrow-next")
        .expect("the next arrow");
    cx.simulate_mouse_down(arrow.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(arrow.center(), MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(front(&places), Some(1), "the tap turned it once");
}

#[gpui::test]
fn a_touch_move_does_not_hold_carousel_autoplay(cx: &mut TestAppContext) {
    let (places, cx) = autoplay_deck(true, cx);
    let carousel = cx
        .debug_bounds("carousel-arrow-next")
        .expect("the next arrow");
    cx.simulate_mouse_move(carousel.center(), None, Modifiers::none());
    settle(cx);
    wait_for_autoplay(cx);
    assert_ne!(
        front(&places),
        Some(0),
        "a touch move does not hold autoplay"
    );
}

#[gpui::test]
fn a_desktop_move_holds_carousel_autoplay(cx: &mut TestAppContext) {
    let (places, cx) = autoplay_deck(false, cx);
    let carousel = cx
        .debug_bounds("carousel-arrow-next")
        .expect("the next arrow");
    cx.simulate_mouse_move(carousel.center(), None, Modifiers::none());
    settle(cx);
    wait_for_autoplay(cx);
    assert_eq!(front(&places), Some(0), "a desktop pointer holds autoplay");
}

#[gpui::test]
fn a_desktop_carousel_arrow_keeps_its_small_hit_area(cx: &mut TestAppContext) {
    let (_, cx) = deck(cx);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    cx.run_until_parked();
    let bounds = cx
        .debug_bounds("carousel-arrow-next")
        .expect("the next arrow");
    assert!(
        bounds.size.width < px(48.) && bounds.size.height < px(48.),
        "a mouse does not need a finger-sized arrow: {bounds:?}"
    );
}
