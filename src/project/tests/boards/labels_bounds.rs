use super::super::{press, settle};
use crate::{
    mail::Label,
    primitives::FocusScope,
    project::LabelManager,
    theme::{ActiveTheme, ControlSize, IconSize, Theme},
};
use gpui::{
    Bounds, Context, FocusHandle, IntoElement, Modifiers, MouseButton, ParentElement, Pixels,
    Render, ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, TouchPhase, Window, div, point,
    px, rems, size,
};

struct Labels {
    root: FocusHandle,
    area: Bounds<Pixels>,
    events: Vec<String>,
}

impl Render for Labels {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (recolor, rename, create, delete) =
            (cx.entity(), cx.entity(), cx.entity(), cx.entity());
        FocusScope::new(&self.root).root().size_full().child(
            div().w(px(340.)).child(
                LabelManager::new(
                    "labels",
                    [
                        Label::new("first", "First", 0),
                        Label::new("second", "Second", 1),
                    ],
                )
                .usable_bounds(self.area)
                .on_recolor(move |key, hue, _, cx| {
                    recolor.update(cx, |view, cx| {
                        view.events.push(format!("recolor {key} {hue}"));
                        cx.notify();
                    });
                })
                .on_rename(move |_, _, _, cx| {
                    rename.update(cx, |view, _| view.events.push("rename".into()));
                })
                .on_create(move |_, _, cx| {
                    create.update(cx, |view, _| view.events.push("create".into()));
                })
                .on_delete(move |_, _, cx| {
                    delete.update(cx, |view, _| view.events.push("delete".into()));
                }),
            ),
        )
    }
}

#[gpui::test]
fn second_labels_clipped_palette_selects_once_after_live_shrink(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Labels {
        root: cx.focus_handle(),
        area: Bounds::new(point(px(20.), px(25.)), size(px(320.), px(300.))),
        events: Vec::new(),
    });
    cx.simulate_resize(size(px(390.), px(600.)));
    settle(cx);
    // Each label has swatch, name and delete stops; focus the second swatch before touch.
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..4 {
            window.focus_next(cx);
        }
    });
    let before = cx.update(|window, cx| window.focused(cx));
    let row = cx.debug_bounds("label-row second").unwrap();
    let at = cx.update(|window, cx| {
        point(
            row.left()
                + rems(0.25).to_pixels(window.rem_size())
                + px(1.)
                + cx.theme()
                    .icon_size(IconSize::Sm)
                    .to_pixels(window.rem_size())
                    / 2.,
            row.center().y,
        )
    });
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("popover-panel").is_some());
    let small = Bounds::new(point(px(45.), px(70.)), size(px(180.), px(65.)));
    view.update(cx, |view, cx| {
        view.area = small;
        cx.notify();
    });
    settle(cx);
    let panel = cx.debug_bounds("popover-panel").unwrap();
    assert!(panel.left() >= small.left() && panel.top() >= small.top());
    assert!(panel.right() <= small.right() && panel.bottom() <= small.bottom());
    let (frame, swatch, gap) = cx.update(|window, cx| {
        (
            rems(0.75).to_pixels(window.rem_size()) + px(1.),
            cx.theme()
                .control_height(ControlSize::Sm)
                .to_pixels(window.rem_size()),
            rems(0.375).to_pixels(window.rem_size()),
        )
    });
    assert!(
        frame * 2. + swatch * 2. + gap > panel.size.height,
        "last row is clipped"
    );
    cx.simulate_event(ScrollWheelEvent {
        position: panel.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(-1000.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    // Four columns, eight hues: the bottom-right swatch is Cyan (index 7).
    let panel = cx.debug_bounds("popover-panel").unwrap();
    let at = point(
        panel.right() - frame - swatch / 2.,
        panel.bottom() - frame - swatch / 2.,
    );
    assert!(panel.contains(&at));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.events.clone()),
        ["recolor second 7"]
    );
    press("escape", cx);
    assert!(cx.debug_bounds("popover-panel").is_none());
    assert_eq!(cx.update(|window, cx| window.focused(cx)), before);
    assert_eq!(
        view.read_with(cx, |view, _| view.events.clone()),
        ["recolor second 7"]
    );
}
