use gpui::{
    Axis, Bounds, Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollHandle, StatefulInteractiveElement, Styled, TestAppContext, Window, div, point, px, size,
};

use super::reveal_when_focused_in_bounds;

struct SafeDialog;

impl Render for SafeDialog {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            crate::overlays::Dialog::new("safe-dialog", "Safe", |_, _| {})
                .fullscreen()
                .usable_bounds(Bounds::new(
                    point(px(20.), px(40.)),
                    size(px(280.), px(300.)),
                ))
                .child(div().debug_selector(|| "safe-field".into()).h(px(40.))),
        )
    }
}

#[gpui::test]
fn fullscreen_dialog_body_stays_inside_usable_bounds(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::theme::Theme::init(cx);
        crate::theme::Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| SafeDialog);
    cx.run_until_parked();
    std::thread::sleep(std::time::Duration::from_millis(10));
    cx.update(|window, _| window.refresh());
    let field = cx.debug_bounds("safe-field").expect("dialog field");
    assert!(field.left() >= px(20.) && field.right() <= px(300.));
    assert!(field.top() >= px(40.) && field.bottom() <= px(340.));
}

struct KeyboardField {
    scroll: ScrollHandle,
    focus: FocusHandle,
    keyboard: bool,
    tail_height: f32,
}

impl Render for KeyboardField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = Bounds::new(
            point(px(0.), px(0.)),
            size(px(300.), px(if self.keyboard { 200. } else { 400. })),
        );
        div()
            .id("scroll")
            .w(px(300.))
            .h(px(400.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(div().h(px(250.)))
            .child(div().relative().h(px(40.)).track_focus(&self.focus).child(
                reveal_when_focused_in_bounds(
                    "field-reveal",
                    &self.scroll,
                    &self.focus,
                    Axis::Vertical,
                    visible,
                    window,
                    cx,
                ),
            ))
            .child(div().h(px(self.tail_height)))
    }
}

#[gpui::test]
fn keyboard_reveals_same_focused_field_once(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        KeyboardField {
            scroll: ScrollHandle::new(),
            focus,
            keyboard: false,
            tail_height: 510.,
        }
    });
    cx.run_until_parked();
    let offset =
        |cx: &mut gpui::VisualTestContext| view.read_with(cx, |view, _| view.scroll.offset());
    assert_eq!(offset(cx).y, px(0.));
    view.update(cx, |view, cx| {
        view.keyboard = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    let revealed = offset(cx);
    assert_eq!(revealed.y, px(-90.));
    cx.update(|window, _| window.refresh());
    assert_eq!(
        offset(cx),
        revealed,
        "unchanged occlusion must not keep scrolling"
    );
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    assert!(cx.update(|window, _| focus.is_focused(window)));
}

fn limited_scroll_reveal(cx: &mut TestAppContext, tail_height: f32, reach: f32) {
    let (view, cx) = cx.add_window_view(|_, cx| KeyboardField {
        scroll: ScrollHandle::new(),
        focus: cx.focus_handle(),
        keyboard: false,
        tail_height,
    });
    cx.run_until_parked();
    let scroll = view.read_with(cx, |view, _| view.scroll.clone());
    assert_eq!(scroll.max_offset().y, px(reach));
    let frame = scroll.bounds();
    let visible = Bounds::new(frame.origin, size(frame.size.width, px(200.)));
    let field = Bounds::new(
        frame.origin + point(px(0.), px(250.)),
        size(px(100.), px(40.)),
    );
    assert_eq!(super::reveal_in_bounds(&scroll, field, visible), reach > 0.);
    assert_eq!(scroll.offset().y, px(-reach));
    cx.update(|window, _| window.refresh());
    assert_eq!(
        scroll.offset().y,
        px(-reach),
        "prepaint must not undo the reported movement"
    );
    let painted = Bounds::new(field.origin + scroll.offset(), field.size);
    assert!(
        painted.bottom() > visible.bottom(),
        "limited reach cannot fully reveal this field"
    );
    assert!(
        !super::reveal_in_bounds(&scroll, painted, visible),
        "at the scroll limit there is no further movement"
    );
}

#[gpui::test]
fn keyboard_reveal_reports_no_movement_without_scroll_range(cx: &mut TestAppContext) {
    limited_scroll_reveal(cx, 110., 0.);
}

#[gpui::test]
fn keyboard_reveal_clamps_to_insufficient_scroll_range(cx: &mut TestAppContext) {
    limited_scroll_reveal(cx, 140., 30.);
}
