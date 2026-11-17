use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};

use super::{Flamegraph, ProfileFrame};
use crate::theme::Theme;

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// A flame graph; it keeps the last frame it was told about.
struct Flame {
    seen: Rc<RefCell<Option<Vec<usize>>>>,
}

impl Render for Flame {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let seen = self.seen.clone();
        let root = ProfileFrame {
            name: "main".into(),
            own: 10,
            calls: vec![ProfileFrame {
                name: "render".into(),
                own: 30,
                calls: vec![],
            }],
        };
        div().w(px(400.0)).child(
            Flamegraph::new("flame", root).on_hover(move |path, _, _| *seen.borrow_mut() = path),
        )
    }
}

#[gpui::test]
fn a_tap_on_a_frame_reports_it_without_a_mouse_move(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let seen = Rc::new(RefCell::new(None));
    let store = seen.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Flame { seen: store });
    settle(cx);
    assert!(
        seen.borrow().is_none(),
        "nothing is reported before the tap"
    );

    let frames = cx.debug_bounds("flame-frames").expect("the frames draw");
    let at = point(
        frames.left() + frames.size.width * 0.1,
        frames.top() + frames.size.height * 0.75,
    );
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    settle(cx);

    assert_eq!(
        seen.borrow().clone(),
        Some(vec![0]),
        "the tap alone reports the frame under the finger"
    );
}

#[gpui::test]
fn a_touch_mouse_move_does_not_report_a_hovered_frame(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let seen = Rc::new(RefCell::new(None));
    let store = seen.clone();
    let (_, cx) = cx.add_window_view(move |_, _| Flame { seen: store });
    settle(cx);

    let frames = cx.debug_bounds("flame-frames").expect("the frames draw");
    let at = point(
        frames.left() + frames.size.width * 0.1,
        frames.top() + frames.size.height * 0.75,
    );
    cx.simulate_mouse_move(at, None, Modifiers::none());
    settle(cx);

    assert!(
        seen.borrow().is_none(),
        "a synthesized touch move must not leave a frame hovered"
    );
}
