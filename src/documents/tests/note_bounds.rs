use super::settle;
use crate::{
    documents::{DocPage, DocumentViewer, PageNote},
    theme::{ActiveTheme, ControlSize, Theme},
};
use gpui::{
    Bounds, Context, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Point, Render,
    ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, TouchPhase, Window, div, point, px,
    rems, size,
};

struct Notes {
    area: Bounds<Pixels>,
    calls: Vec<(usize, Point<f32>)>,
}

impl Render for Notes {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().size_full().child(
            DocumentViewer::new(
                "notes",
                "Notes",
                vec![DocPage {
                    source: "note-bounds-fixture.png".into(),
                    width: 400.,
                    height: 1000.,
                }],
            )
            .zoom(1.5, |_, _, _| {})
            .notes(
                vec![PageNote {
                    page: 0,
                    at: point(120., 180.),
                    text: "Note detail. ".repeat(80).into(),
                }],
                move |page, at, _, cx| {
                    owner.update(cx, |view, _| view.calls.push((page, at)));
                },
            )
            .usable_bounds(self.area),
        )
    }
}

#[gpui::test]
fn note_disclosure_keeps_window_bounds_after_zoom_scroll_and_does_not_pin(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let area = Bounds::new(point(px(70.), px(80.)), size(px(330.), px(360.)));
    let (view, cx) = cx.add_window_view(|_, _| Notes {
        area,
        calls: Vec::new(),
    });
    cx.simulate_resize(size(px(900.), px(700.)));
    settle(cx);
    let original = cx.debug_bounds("hover-card-disclosure").unwrap();
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(450.), px(500.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-120.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    let moved = cx.debug_bounds("hover-card-disclosure").unwrap();
    assert_eq!(
        original.top() - moved.top(),
        px(120.),
        "the page actually scrolled"
    );

    let pin = cx.update(|window, cx| {
        let theme = cx.theme();
        let half = theme.touch_minimum(
            theme
                .control_height(ControlSize::Sm)
                .to_pixels(window.rem_size()),
        ) / 2.;
        point(
            window.viewport_size().width - px(1.) - rems(0.75).to_pixels(window.rem_size()) - half,
            px(1.)
                + theme
                    .control_height(ControlSize::Lg)
                    .to_pixels(window.rem_size())
                    / 2.,
        )
    });
    cx.simulate_mouse_down(pin, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(pin, MouseButton::Left, Modifiers::none());
    settle(cx);

    // A real page press proves pinning is enabled and checks the inverse zoom/scroll transform.
    let disclosure = cx.debug_bounds("hover-card-disclosure").unwrap();
    let note_origin = cx.update(|window, cx| {
        disclosure.origin
            - point(
                rems(1.5).to_pixels(window.rem_size()) + cx.theme().touch_spacing(),
                px(0.),
            )
    });
    let page_press = disclosure.center() + point(px(90.), px(80.));
    cx.simulate_mouse_down(page_press, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(page_press, MouseButton::Left, Modifiers::none());
    settle(cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.calls.len(), 1, "pinning positive control");
        let (page, at) = view.calls[0];
        assert_eq!(page, 0);
        let expected = point(
            120. + f32::from(page_press.x - note_origin.x) / 1.5,
            180. + f32::from(page_press.y - note_origin.y) / 1.5,
        );
        assert!((at.x - expected.x).abs() < 1., "{at:?} vs {expected:?}");
        assert!((at.y - expected.y).abs() < 1., "{at:?} vs {expected:?}");
    });
    let at = disclosure.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.calls.len()),
        1,
        "disclosure must not pin"
    );
    for bounds in [
        area,
        Bounds::new(point(px(95.), px(110.)), size(px(240.), px(130.))),
    ] {
        view.update(cx, |view, cx| {
            view.area = bounds;
            cx.notify();
        });
        settle(cx);
        let panel = cx
            .debug_bounds("popover-panel")
            .expect("note preview remains open");
        assert!(panel.left() >= bounds.left() && panel.top() >= bounds.top());
        assert!(panel.right() <= bounds.right() && panel.bottom() <= bounds.bottom());
        assert_eq!(view.read_with(cx, |view, _| view.calls.len()), 1);
    }
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
}
