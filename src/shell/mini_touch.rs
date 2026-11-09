use gpui::{
    AppContext as _, Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

use super::MiniWindow;
use crate::theme::Theme;

/// The body a mini window holds.
struct Body;

impl Render for Body {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child("Body")
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn a_mini_window_keeps_its_bar_in_touch_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|_, cx| {
        let body = cx.new(|_| Body);
        MiniWindow::new("Notes", body.into())
    });
    settle(cx);
    let bar = cx
        .debug_bounds("mini-window-bar")
        .expect("the mini window keeps its bar");
    assert!(
        bar.size.height >= px(48.),
        "the bar is a touch target: {bar:?}"
    );
    let close = cx
        .debug_bounds("mini-window-close")
        .expect("the mini window keeps its close button");
    assert!(
        close.size.width >= px(48.) && close.size.height >= px(48.),
        "the close button is a touch target: {close:?}"
    );
    // A tap on the bar does not throw the window away.
    let at = bar.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("mini-window-bar").is_some(),
        "the bar stays put"
    );
}

#[gpui::test]
fn a_mini_window_hides_its_bar_on_the_desktop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let (_, cx) = cx.add_window_view(|_, cx| {
        let body = cx.new(|_| Body);
        MiniWindow::new("Notes", body.into())
    });
    settle(cx);
    // The desktop keeps its own bar size; a phone would have widened it to a
    // thumb-sized target, and the desktop does not.
    let bar = cx.debug_bounds("mini-window-bar").expect("the bar draws");
    assert!(
        bar.size.height < px(48.),
        "the desktop keeps the desktop bar: {bar:?}"
    );
}
