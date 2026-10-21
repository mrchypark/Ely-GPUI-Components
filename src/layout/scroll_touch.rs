use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};

use super::ScrollArea;
use crate::theme::Theme;

/// A scroll area whose body is far taller than its box, so a bar is needed.
struct Long;

impl Render for Long {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(200.)).h(px(200.)).child(
            ScrollArea::new("touch-scroll")
                .size_full()
                .child(div().h(px(800.)).w(px(800.))),
        )
    }
}

/// Settles the window into a drawn frame, so the scroll area is measured.
fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// The scrollbar of a scrolling area, on a phone or on a desktop.
#[gpui::test]
fn a_touch_scrollbar_stays_out_and_grabs_wide(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = false);
    });
    let (_, cx) = cx.add_window_view(|_, _| Long);
    cx.run_until_parked();
    // The bar needs the measured box of the scroll area, which comes on the
    // second frame.
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    // The desktop keeps its thin bar and waits for a hover to call it in.
    let desktop_bar = cx.debug_bounds("scrollbar-bar").expect("the scrollbar bar");
    let desktop_thumb = cx
        .debug_bounds("scrollbar-thumb")
        .expect("the scrollbar thumb");
    assert!(
        desktop_thumb.size.width < px(48.),
        "the desktop keeps its thin thumb: {desktop_thumb:?}"
    );

    // A phone has no hover to call the bar in, so it is always there.
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let grab = cx
        .debug_bounds("scrollbar-thumb")
        .expect("the scrollbar thumb");
    assert!(
        grab.size.width >= px(48.),
        "the thumb is a finger wide: {grab:?}"
    );
    let bar = cx.debug_bounds("scrollbar-bar").expect("the scrollbar bar");
    assert!(
        bar.size.width < px(48.),
        "the bar stays thin while the thumb is wide: {bar:?}"
    );
    assert!(
        bar.origin == desktop_bar.origin,
        "the bar sits where it always sat: {bar:?} against {desktop_bar:?}"
    );
    // The wide target reaches past the visible bar toward the edge of the
    // screen, where a thumb lands.
    assert!(
        grab.right() > bar.right(),
        "the grab target reaches past the bar to the edge: {grab:?} against {bar:?}"
    );
}

/// The horizontal scrollbar, on a phone or on a desktop.
#[gpui::test]
fn a_touch_horizontal_scrollbar_grabs_wide(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = false);
    });
    let (_, cx) = cx.add_window_view(|_, _| Long);
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let desktop_bar = cx
        .debug_bounds("scrollbar-bar-x")
        .expect("the horizontal scrollbar bar");
    assert!(
        desktop_bar.size.height < px(48.),
        "the desktop keeps its thin bar: {desktop_bar:?}"
    );

    // A phone has no hover to call the bar in, so it is always there.
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let grab = cx
        .debug_bounds("scrollbar-thumb-x")
        .expect("the horizontal scrollbar thumb");
    assert!(
        grab.size.height >= px(48.),
        "the thumb is a finger wide: {grab:?}"
    );
    let bar = cx
        .debug_bounds("scrollbar-bar-x")
        .expect("the horizontal scrollbar bar");
    assert!(
        bar.size.height < px(48.),
        "the bar stays thin while the thumb is wide: {bar:?}"
    );
    assert!(
        bar.origin == desktop_bar.origin,
        "the bar sits where it always sat: {bar:?} against {desktop_bar:?}"
    );
    assert!(
        grab.bottom() > bar.bottom(),
        "the grab target reaches past the bar to the edge: {grab:?} against {bar:?}"
    );
}

/// The scrollbar of a scrolling area nobody hovers, on a phone or on a desktop.
#[gpui::test]
fn a_touch_scrollbar_stays_out_when_nothing_is_hovered(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = false);
    });
    let (_, cx) = cx.add_window_view(|_, _| Long);
    settle(cx);

    // The test mouse rests at the corner, so the scroll area is hovered.
    assert!(
        cx.debug_bounds("scrollbar-bar").is_some(),
        "the hovered scroll area shows its bar"
    );

    // A mouse that leaves the area takes the bar with it.
    cx.simulate_mouse_move(point(px(1000.), px(1000.)), None, Modifiers::none());
    settle(cx);
    // The desktop keeps its bar out of the way until a mouse asks for it.
    assert!(
        cx.debug_bounds("scrollbar-bar").is_none(),
        "the desktop bar waits for a hover: {:?}",
        cx.debug_bounds("scrollbar-bar")
    );

    // A phone has no hover to call the bar in, so it is always there.
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    settle(cx);
    assert!(
        cx.debug_bounds("scrollbar-bar").is_some(),
        "the phone keeps its bar out: {:?}",
        cx.debug_bounds("scrollbar-bar")
    );
}
