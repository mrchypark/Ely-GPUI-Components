use gpui::{
    Context, IntoElement, Modifiers, Render, SharedString, TestAppContext, VisualTestContext,
    Window, px,
};

use super::setup;
use crate::navigation::{EditorTab, EditorTabs};

/// The editor tabs a phone shows, and the tab that was closed.
struct Docs {
    closed: Option<SharedString>,
}

impl Render for Docs {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        EditorTabs::new(
            "touch-docs",
            [
                EditorTab::new("dirty", "Dirty").dirty(true),
                EditorTab::new("clean", "Clean"),
            ],
        )
        .selected("dirty")
        .on_close(move |id, _, cx| {
            let id = id.clone();
            view.update(cx, |view, _| view.closed = Some(id));
        })
    }
}

/// The test platform never runs next-frame callbacks; a refresh stands in for the display link.
fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

#[gpui::test]
fn a_touch_tab_always_shows_its_dirty_dot(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (_, cx) = cx.add_window_view(|_, _| Docs { closed: None });
    settle(cx);
    let dot = cx
        .debug_bounds("editor-touch-dirty-dirty")
        .expect("a dirty tab shows its dot without a hover");
    assert!(dot.size.width > px(0.) && dot.size.height > px(0.));
    // The desktop keeps the dot for a hover, so it is not there to begin with.
    cx.update(|_, cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    assert!(
        cx.debug_bounds("editor-touch-dirty-dirty").is_none(),
        "the desktop leaves the dirty dot to a hover"
    );
}

#[gpui::test]
fn a_touch_tab_close_target_is_a_thumb(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (_, cx) = cx.add_window_view(|_, _| Docs { closed: None });
    settle(cx);
    let close = cx
        .debug_bounds("editor-close-clean")
        .expect("a touch tab keeps its close button");
    assert!(
        close.size.width >= px(48.) && close.size.height >= px(48.),
        "the close button is a touch target: {close:?}"
    );
    cx.update(|_, cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    let desktop = cx
        .debug_bounds("editor-close-clean")
        .expect("the slot draws");
    assert!(
        desktop.size.height < px(48.),
        "the desktop keeps its own close button: {desktop:?}"
    );
}

#[gpui::test]
fn a_touch_tab_closes_without_a_hover(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (view, cx) = cx.add_window_view(|_, _| Docs { closed: None });
    settle(cx);
    let close = cx
        .debug_bounds("editor-close-clean")
        .expect("a touch tab keeps its close button");
    // No simulated hover: the invisible desktop cover would swallow this click.
    cx.simulate_click(close.center(), Modifiers::none());
    assert_eq!(
        view.read_with(cx, |view, _| view.closed.clone()),
        Some(SharedString::from("clean")),
        "the close button works without a hover"
    );
}
