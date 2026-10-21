use super::{press, settle};
mod no_room;
use crate::{
    buttons::Button,
    overlays::{HoverCard, Popover},
    primitives::FocusScope,
    theme::Theme,
};
use gpui::{
    Bounds, Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, ParentElement,
    Pixels, Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, TouchPhase,
    VisualTestContext, Window, div, point, px, size,
};

struct BoundedOverlay {
    root: FocusHandle,
    before: FocusHandle,
    bounds: Bounds<Pixels>,
    hover: bool,
    primary: usize,
    actions: usize,
    wheels: usize,
    builds: std::rc::Rc<std::cell::Cell<usize>>,
}

impl Render for BoundedOverlay {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let action = cx.entity();
        let builds = self.builds.clone();
        let content = move |_: &mut Window, _: &mut gpui::App| {
            builds.set(builds.get() + 1);
            let action = action.clone();
            div()
                .w(px(900.))
                .h(px(900.))
                .flex_none()
                .flex()
                .flex_col()
                .items_end()
                .justify_end()
                .child(div().debug_selector(|| "overlay-last".into()).child(
                    Button::new("last", "Last action").on_click(move |_, _, cx| {
                        action.update(cx, |view, _| view.actions += 1);
                    }),
                ))
        };
        let primary = cx.entity();
        let overlay = if self.hover {
            HoverCard::new(
                "bounded-hover",
                div().debug_selector(|| "overlay-primary".into()).child(
                    Button::new("primary", "Primary").on_click(move |_, _, cx| {
                        primary.update(cx, |view, _| view.primary += 1);
                    }),
                ),
                content,
            )
            .touch_label("Show definition")
            .usable_bounds(self.bounds)
            .into_any_element()
        } else {
            Popover::new("bounded-pop", "Open", content)
                .usable_bounds(self.bounds)
                .into_any_element()
        };
        let wheels = cx.entity();
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().id("before").track_focus(&self.before))
            .child(
                div()
                    .flex()
                    .items_start()
                    .on_scroll_wheel(move |_, _, cx| {
                        wheels.update(cx, |view, _| view.wheels += 1);
                    })
                    .child(
                        div()
                            .flex_none()
                            .debug_selector(|| "overlay-trigger".into())
                            .child(overlay),
                    ),
            )
    }
}

fn fixture(
    cx: &mut TestAppContext,
    hover: bool,
) -> (gpui::Entity<BoundedOverlay>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| BoundedOverlay {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        bounds: Bounds::new(point(px(20.), px(30.)), size(px(280.), px(300.))),
        hover,
        primary: 0,
        actions: 0,
        wheels: 0,
        builds: Default::default(),
    });
    cx.simulate_resize(size(px(320.), px(500.)));
    settle(cx);
    let before = view.read_with(cx, |view, _| view.before.clone());
    cx.update(|window, cx| window.focus(&before, cx));
    (view, cx)
}

fn open(cx: &mut VisualTestContext, hover: bool) {
    let selector = if hover {
        "hover-card-disclosure"
    } else {
        "overlay-trigger"
    };
    let bounds = cx.debug_bounds(selector).unwrap();
    cx.simulate_click(bounds.center(), Modifiers::none());
    settle(cx);
}

fn contained(cx: &mut VisualTestContext, area: Bounds<Pixels>) -> Bounds<Pixels> {
    let panel = cx
        .debug_bounds("popover-panel")
        .or_else(|| cx.debug_bounds("hover-card-panel"))
        .expect("open panel");
    assert!(
        panel.left() >= area.left() && panel.top() >= area.top(),
        "{panel:?}"
    );
    assert!(
        panel.right() <= area.right() && panel.bottom() <= area.bottom(),
        "{panel:?}"
    );
    panel
}

fn scroll_to_last(cx: &mut VisualTestContext, area: Bounds<Pixels>) {
    let panel = contained(cx, area);
    for delta in [point(px(-2000.), px(0.)), point(px(0.), px(-2000.))] {
        cx.simulate_event(ScrollWheelEvent {
            position: panel.center(),
            delta: ScrollDelta::Pixels(delta),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
        });
        settle(cx);
    }
    let last = cx.debug_bounds("overlay-last").unwrap();
    assert!(
        panel.contains(&last.center()),
        "last action is reachable: {last:?}"
    );
    cx.simulate_click(last.center(), Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn bounded_popover_scrolls_wide_tall_content_and_refits_while_open(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    open(cx, false);
    let area = view.read_with(cx, |view, _| view.bounds);
    scroll_to_last(cx, area);
    assert_eq!(
        view.read_with(cx, |view, _| (view.actions, view.wheels)),
        (1, 0)
    );
    let smaller = Bounds::new(point(px(35.), px(50.)), size(px(240.), px(160.)));
    view.update(cx, |view, cx| {
        view.bounds = smaller;
        cx.notify();
    });
    settle(cx);
    scroll_to_last(cx, smaller);
    assert_eq!(view.read_with(cx, |view, _| view.actions), 2);
    press("escape", cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
    let before = view.read_with(cx, |view, _| view.before.clone());
    assert!(cx.update(|window, _| before.is_focused(window)));
    open(cx, false);
    cx.simulate_click(point(px(310.), px(450.)), Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
}

#[gpui::test]
fn touch_hover_disclosure_preserves_primary_action_and_ignores_hover_leave(
    cx: &mut TestAppContext,
) {
    let (view, cx) = fixture(cx, true);
    let primary = cx.debug_bounds("overlay-primary").unwrap().center();
    cx.simulate_click(primary, Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |view, _| view.primary), 1);
    assert!(cx.debug_bounds("popover-panel").is_none());
    let disclosure = cx.debug_bounds("hover-card-disclosure").unwrap();
    assert!(disclosure.size.width >= px(48.) && disclosure.size.height >= px(48.));
    open(cx, true);
    cx.simulate_mouse_move(point(px(310.), px(450.)), None, Modifiers::none());
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(700));
    settle(cx);
    let area = view.read_with(cx, |view, _| view.bounds);
    scroll_to_last(cx, area);
    assert_eq!(
        view.read_with(cx, |view, _| (view.primary, view.actions)),
        (1, 1)
    );
    press("escape", cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
    open(cx, true);
    cx.simulate_click(point(px(310.), px(450.)), Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
    assert_eq!(
        view.read_with(cx, |view, _| (view.primary, view.actions)),
        (1, 1)
    );
}

#[gpui::test]
fn bounded_desktop_hover_keeps_delay_and_scrolls_without_disclosure(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    assert!(cx.debug_bounds("hover-card-disclosure").is_none());
    let primary = cx.debug_bounds("overlay-primary").unwrap().center();
    cx.simulate_mouse_move(primary, None, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("hover-card-panel").is_none());
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(600));
    settle(cx);
    let area = view.read_with(cx, |view, _| view.bounds);
    scroll_to_last(cx, area);
    assert_eq!(
        view.read_with(cx, |view, _| (view.primary, view.actions)),
        (0, 1)
    );
}

struct Definition;

impl Render for Definition {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        crate::documents::TooltipTerm::new("term", "Retention", "How long records are kept")
    }
}

#[gpui::test]
fn information_only_term_opens_without_hover_and_defaults_to_viewport(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Definition);
    cx.simulate_resize(size(px(320.), px(300.)));
    settle(cx);
    open(cx, true);
    contained(
        cx,
        Bounds::new(point(px(0.), px(0.)), size(px(320.), px(300.))),
    );
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(700));
    settle(cx);
    assert!(cx.debug_bounds("popover-panel").is_some());
    press("escape", cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
}
