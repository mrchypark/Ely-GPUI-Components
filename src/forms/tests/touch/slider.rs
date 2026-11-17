use super::*;

#[gpui::test]
fn touch_drag_tracks_values_when_the_parent_has_not_rendered(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let changes = Rc::new(RefCell::new(Vec::new()));
    let (_view, cx) = cx.add_window_view(|_, _| CallbackOnly(changes.clone()));
    cx.run_until_parked();
    let track = cx.debug_bounds("slider-track").unwrap();
    let at = |fraction| point(track.left() + track.size.width * fraction, track.center().y);
    let start_position = at(0.25);
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Started,
        start_position,
        position: start_position,
    });
    for value in [0.80, 0.25] {
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Moved,
            start_position,
            position: at(value),
        });
    }
    cx.run_until_parked();
    assert_eq!(*changes.borrow(), [80., 25.]);
}

#[gpui::test]
fn touch_range_drags_clamped_values_once_before_the_parent_renders(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let changes = Rc::new(RefCell::new(Vec::new()));
    let (_view, cx) = cx.add_window_view(|_, _| RangeCallbackOnly(changes.clone()));
    cx.run_until_parked();
    let track = cx.debug_bounds("slider-track").unwrap();
    let at = |fraction| point(track.left() + track.size.width * fraction, track.center().y);
    let start_position = at(0.75);
    cx.simulate_event(TouchDragEvent {
        phase: TouchPhase::Started,
        start_position,
        position: start_position,
    });
    for value in [0.20, 0.10, 0.75] {
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Moved,
            start_position,
            position: at(value),
        });
    }
    cx.run_until_parked();
    assert_eq!(*changes.borrow(), [(25., 25.), (25., 75.)]);
}
impl Render for TrackProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let slider = if self.range {
            let slider = RangeSlider::new("track", self.span.0, self.span.1).on_change({
                let view = cx.entity();
                move |value, _, cx| {
                    view.update(cx, |v, cx| {
                        v.span = value;
                        v.changes.push(value);
                        cx.notify();
                    })
                }
            });
            if self.vertical {
                slider.vertical().into_any_element()
            } else {
                slider.into_any_element()
            }
        } else {
            let slider = Slider::new("track", self.value).on_change({
                let view = cx.entity();
                move |value, _, cx| {
                    view.update(cx, |v, cx| {
                        v.value = value;
                        v.changes.push((value, 0.));
                        cx.notify();
                    })
                }
            });
            if self.vertical {
                slider.vertical().into_any_element()
            } else {
                slider.into_any_element()
            }
        };
        div().w(px(240.)).h(px(240.)).child(slider)
    }
}

#[gpui::test]
fn touch_slider_track_and_thumb_share_coordinates_in_both_orientations(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        TrackProbe {
            value: 50.,
            span: (25., 75.),
            changes: Vec::new(),
            vertical: false,
            range: false,
        }
    });
    for vertical in [false, true] {
        view.update(cx, |v, cx| {
            v.vertical = vertical;
            cx.notify();
        });
        cx.run_until_parked();
        let root = cx.debug_bounds("slider-root").unwrap();
        let track = cx.debug_bounds("slider-track").unwrap();
        let thumb = cx.debug_bounds("slider-thumb-0").unwrap();
        assert!(thumb.size.width >= px(48.) && thumb.size.height >= px(48.));
        assert_eq!(
            if vertical {
                root.size.width
            } else {
                root.size.height
            },
            px(48.)
        );
        for (fraction, expected) in [(0.01, 1.), (0.99, 99.)] {
            let at = if vertical {
                point(
                    track.left() + px(2.),
                    track.bottom() - track.size.height * fraction,
                )
            } else {
                point(
                    track.left() + track.size.width * fraction,
                    track.top() + px(2.),
                )
            };
            cx.simulate_click(at, Modifiers::none());
            assert_eq!(view.read_with(cx, |v, _| v.value), expected);
        }
    }
}

#[gpui::test]
fn touch_range_slider_keyboard_cannot_cross_other_thumb(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| TrackProbe {
        value: 50.,
        span: (25., 75.),
        changes: Vec::new(),
        vertical: false,
        range: true,
    });
    cx.run_until_parked();
    let low = cx.debug_bounds("slider-thumb-0").unwrap();
    cx.simulate_click(low.center(), Modifiers::none());
    cx.simulate_keystrokes("end");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |v, _| v.span), (75., 75.));
}

#[gpui::test]
fn touch_range_drag_keeps_the_thumb_selected_on_press(cx: &mut TestAppContext) {
    use gpui::MouseButton;
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| {
            t.touch_mode = true;
            t.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, _| TrackProbe {
        value: 50.,
        span: (49., 51.),
        changes: Vec::new(),
        vertical: false,
        range: true,
    });
    for vertical in [false, true] {
        for (span, start, end, expected) in [
            ((49., 51.), 0.49, 0.20, (20., 51.)),
            ((49., 51.), 0.51, 0.80, (49., 80.)),
            ((50., 50.), 0.50, 0.20, (20., 50.)),
            ((50., 50.), 0.50, 0.80, (50., 80.)),
            ((50., 50.), 0.49, 0.20, (20., 50.)),
            ((50., 50.), 0.51, 0.80, (50., 80.)),
            ((25., 75.), 0.75, 0.90, (25., 90.)),
            ((49., 51.), 0.49, 0.80, (51., 51.)),
        ] {
            view.update(cx, |v, cx| {
                v.span = span;
                v.vertical = vertical;
                cx.notify();
            });
            cx.run_until_parked();
            let track = cx.debug_bounds("slider-track").expect("track");
            let at = |fraction| {
                if vertical {
                    point(
                        track.center().x,
                        track.bottom() - track.size.height * fraction,
                    )
                } else {
                    point(track.left() + track.size.width * fraction, track.center().y)
                }
            };
            cx.simulate_mouse_down(at(start), MouseButton::Left, Modifiers::none());
            // First move starts GPUI drag; a subsequent move delivers its value update.
            cx.simulate_mouse_move(at((start + end) / 2.), MouseButton::Left, Modifiers::none());
            cx.simulate_mouse_move(at(end), MouseButton::Left, Modifiers::none());
            cx.simulate_mouse_up(at(end), MouseButton::Left, Modifiers::none());
            cx.run_until_parked();
            assert_eq!(
                view.read_with(cx, |v, _| v.span),
                expected,
                "drag from {span:?}"
            );
        }
    }
}

#[gpui::test]
fn touch_range_endpoint_overhang_resets_press_selection(cx: &mut TestAppContext) {
    use gpui::MouseButton;
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| TrackProbe {
        value: 50.,
        span: (25., 100.),
        changes: Vec::new(),
        vertical: false,
        range: true,
    });
    for vertical in [false, true] {
        for (span, high, expected) in [
            ((25., 100.), true, (25., 80.)),
            ((0., 75.), false, (20., 75.)),
        ] {
            view.update(cx, |view, cx| {
                view.span = span;
                view.changes.clear();
                view.vertical = vertical;
                cx.notify();
            });
            cx.run_until_parked();
            let track = cx.debug_bounds("slider-track").unwrap();
            let at = |fraction| {
                if vertical {
                    point(
                        track.center().x,
                        track.bottom() - track.size.height * fraction,
                    )
                } else {
                    point(track.left() + track.size.width * fraction, track.center().y)
                }
            };
            let outward = if vertical {
                point(px(0.), if high { px(-12.) } else { px(12.) })
            } else {
                point(if high { px(12.) } else { px(-12.) }, px(0.))
            };
            let press = at(if high { 1. } else { 0. }) + outward;
            cx.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
            assert!(view.read_with(cx, |view, _| view.changes.is_empty()));
            cx.simulate_mouse_move(
                at(if high { 0.9 } else { 0.1 }),
                MouseButton::Left,
                Modifiers::none(),
            );
            cx.simulate_mouse_move(
                at(if high { 0.8 } else { 0.2 }),
                MouseButton::Left,
                Modifiers::none(),
            );
            cx.simulate_mouse_up(
                at(if high { 0.8 } else { 0.2 }),
                MouseButton::Left,
                Modifiers::none(),
            );
            cx.run_until_parked();
            assert_eq!(view.read_with(cx, |view, _| view.span), expected);
            assert_eq!(
                view.read_with(cx, |view, _| view.changes.clone()),
                vec![expected]
            );
        }
    }
}
