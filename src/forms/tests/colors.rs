use gpui::{
    Context, Hsla, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px, rgb, rgba,
};

use super::setup;
use crate::forms::{ColorPicker, GradientEditor, GradientStop};

struct Picked {
    color: Hsla,
}

impl Render for Picked {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .size_full()
            .child(ColorPicker::new("picked", self.color).opaque().on_change(
                move |color, _, cx| {
                    view.update(cx, |view, cx| {
                        view.color = color;
                        cx.notify();
                    })
                },
            ))
    }
}

#[gpui::test]
fn an_opaque_picker_commits_opaque_colors(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picked {
        color: rgba(0x3772bb80).into(),
    });
    cx.simulate_click(point(px(60.0), px(60.0)), Modifiers::none());
    assert_eq!(view.read_with(cx, |view, _| view.color.a), 1.0);
}

struct Gradient {
    stops: Vec<GradientStop>,
}

impl Render for Gradient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            GradientEditor::new("gradient", self.stops.clone()).on_change(move |next, _, cx| {
                view.update(cx, |view, cx| {
                    view.stops = next.to_vec();
                    cx.notify();
                })
            }),
        )
    }
}

#[gpui::test]
fn a_dragged_stop_keeps_its_grip_after_passing_another(cx: &mut TestAppContext) {
    setup(cx);
    let stop = |at, hex| GradientStop {
        at,
        color: rgb(hex).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| Gradient {
        stops: vec![
            stop(0.2, 0xff0000),
            stop(0.5, 0x00ff00),
            stop(0.8, 0x0000ff),
        ],
    });
    let width = cx.update(|window, _| window.viewport_size().width);
    let at = |t: f32| point(px(8.0) + (width - px(16.0)) * t, px(44.0));
    cx.simulate_mouse_down(at(0.2), MouseButton::Left, Modifiers::none());
    for t in [0.22, 0.6, 0.65] {
        cx.simulate_mouse_move(at(t), MouseButton::Left, Modifiers::none());
    }
    cx.simulate_mouse_up(at(0.65), MouseButton::Left, Modifiers::none());
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    let place = |hex| {
        let color: Hsla = rgb(hex).into();
        stops.iter().find(|stop| stop.color == color).unwrap().at
    };
    assert!((place(0xff0000) - 0.65).abs() < 0.01, "{stops:?}");
    assert_eq!(place(0x00ff00), 0.5, "{stops:?}");
}

struct TouchPicker {
    color: Hsla,
    changes: Vec<Hsla>,
}

impl Render for TouchPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .w(px(300.))
            .child(
                ColorPicker::new("touch-picker", self.color).on_change(move |color, _, cx| {
                    view.update(cx, |view, cx| {
                        view.color = color;
                        view.changes.push(color);
                        cx.notify();
                    });
                }),
            )
    }
}

fn color_hex_draw(cx: &mut gpui::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

#[gpui::test]
fn color_hex_rail_change_does_not_restore_old_color_on_blur(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (view, cx) = cx.add_window_view(|_, _| TouchPicker {
        color: rgb(0xff0000).into(),
        changes: Vec::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let hex = cx.debug_bounds("input-root").unwrap().center();
    cx.simulate_click(hex, Modifiers::none());
    color_hex_draw(cx);
    let focus = cx.update(|window, cx| window.focused(cx).expect("hex field focused"));
    color_hex_draw(cx);
    let hue = cx.debug_bounds("color-picker-hue").unwrap();
    cx.simulate_click(
        point(hue.left() + hue.size.width * 0.5, hue.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    assert!(cx.update(|window, _| focus.is_focused(window)));
    let (chosen, count) = view.read_with(cx, |view, _| (view.color, view.changes.len()));
    assert!(count > 0 && chosen != Hsla::from(rgb(0xff0000)));
    cx.update(|window, cx| window.blur(cx));
    color_hex_draw(cx);
    assert_eq!(
        view.read_with(cx, |view, _| (view.color, view.changes.len())),
        (chosen, count)
    );
}

#[gpui::test]
fn color_hex_unchanged_blur_does_not_notify(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| TouchPicker {
        color: rgb(0xff0000).into(),
        changes: Vec::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let hex = cx.debug_bounds("input-root").unwrap().center();
    cx.simulate_click(hex, Modifiers::none());
    color_hex_draw(cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_some()));
    cx.update(|window, cx| window.blur(cx));
    color_hex_draw(cx);
    assert_eq!(view.read_with(cx, |view, _| view.changes.len()), 0);
}

#[gpui::test]
fn color_hex_submit_then_blur_notifies_once(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| TouchPicker {
        color: rgb(0xff0000).into(),
        changes: Vec::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let hex = cx.debug_bounds("input-root").unwrap().center();
    cx.simulate_click(hex, Modifiers::none());
    color_hex_draw(cx);
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("#00ff00");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.changes.len()), 1);
    cx.update(|window, cx| window.blur(cx));
    color_hex_draw(cx);
    assert_eq!(view.read_with(cx, |view, _| view.changes.len()), 1);
}

#[gpui::test]
fn color_hex_edited_draft_survives_rail_change_and_wins_on_blur(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (view, cx) = cx.add_window_view(|_, _| TouchPicker {
        color: rgb(0xff0000).into(),
        changes: Vec::new(),
    });
    cx.simulate_resize(gpui::size(px(390.), px(844.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let hex = cx.debug_bounds("input-root").unwrap().center();
    cx.simulate_click(hex, Modifiers::none());
    color_hex_draw(cx);
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("#00ff00");
    let hue = cx.debug_bounds("color-picker-hue").unwrap();
    cx.simulate_click(
        point(hue.left() + hue.size.width * 0.5, hue.center().y),
        Modifiers::none(),
    );
    cx.run_until_parked();
    cx.update(|window, cx| window.blur(cx));
    color_hex_draw(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.color),
        Hsla::from(rgb(0x00ff00))
    );
}

#[gpui::test]
fn color_picker_touch_drags_all_areas_and_releases_on_cancel(cx: &mut TestAppContext) {
    use gpui::{TouchDragEvent, TouchPhase};
    setup(cx);
    cx.update(|cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = true));
    let (view, cx) = cx.add_window_view(|_, _| TouchPicker {
        color: rgb(0xff0000).into(),
        changes: Vec::new(),
    });
    for name in [
        "color-picker-plane",
        "color-picker-hue",
        "color-picker-alpha",
    ] {
        cx.run_until_parked();
        let bounds = cx.debug_bounds(name).expect("picker area");
        if name != "color-picker-plane" {
            assert!(bounds.size.height >= px(48.), "{name}: {bounds:?}");
        }
        let at = |x, y| {
            point(
                bounds.left() + bounds.size.width * x,
                bounds.top() + bounds.size.height * y,
            )
        };
        let start_position = at(0.2, 0.25);
        let initial_count = view.read_with(cx, |view, _| view.changes.len());
        for (phase, position) in [
            (TouchPhase::Started, at(0.3, 0.25)),
            (TouchPhase::Moved, at(0.5, 0.25)),
            (TouchPhase::Ended, at(0.8, 0.25)),
        ] {
            cx.simulate_event(TouchDragEvent {
                phase,
                start_position,
                position,
            });
            cx.run_until_parked();
        }
        let (color, count) = view.read_with(cx, |view, _| (view.color, view.changes.len()));
        assert!(
            count >= initial_count + 2,
            "{name} updates during movement and on end"
        );
        match name {
            "color-picker-plane" => {
                let rgba = color.to_rgb();
                assert!(
                    (rgba.r - 0.75).abs() < 0.02 && (rgba.g - 0.15).abs() < 0.02,
                    "{rgba:?}"
                );
            }
            "color-picker-hue" => assert!((color.h - 0.8).abs() < 0.02, "{color:?}"),
            _ => assert!((color.a - 0.8).abs() < 0.02, "{color:?}"),
        }
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Ended,
            start_position,
            position: at(0.8, 0.25),
        });
        assert_eq!(
            view.read_with(cx, |view, _| view.changes.len()),
            count,
            "release must not repeat"
        );

        let start_position = at(0.3, 0.25);
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Started,
            start_position,
            position: at(0.4, 0.25),
        });
        cx.run_until_parked();
        let before_cancel = view.read_with(cx, |view, _| (view.color, view.changes.len()));
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Moved,
            start_position,
            position: at(0.4, 0.25),
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.changes.len()),
            before_cancel.1,
            "{name} must not repeat unchanged values"
        );
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Cancelled,
            start_position,
            position: at(0.95, 0.25),
        });
        cx.simulate_event(TouchDragEvent {
            phase: TouchPhase::Moved,
            start_position,
            position: at(0.95, 0.25),
        });
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| (view.color, view.changes.len())),
            before_cancel,
            "{name} cancellation"
        );
    }
    cx.update(|_, cx| crate::theme::Theme::update(cx, |theme| theme.touch_mode = false));
    cx.run_until_parked();
    for name in ["color-picker-hue", "color-picker-alpha"] {
        assert!(
            cx.debug_bounds(name).unwrap().size.height < px(48.),
            "desktop {name}"
        );
    }
}
