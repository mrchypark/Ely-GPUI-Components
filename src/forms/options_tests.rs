use gpui::{Bounds, point, px, size};

use super::{Choice, opens_up, step};

#[test]
fn a_panel_opens_up_only_when_it_does_not_fit_below_and_above_has_more() {
    let visible = Bounds::new(point(px(0.0), px(0.0)), size(px(320.0), px(790.0)));
    let low = Bounds::new(point(px(0.0), px(600.0)), size(px(80.0), px(28.0)));
    assert!(opens_up(low, px(260.0), visible));
    assert!(!opens_up(low, px(120.0), visible));
    let high = Bounds::new(point(px(0.0), px(100.0)), size(px(80.0), px(28.0)));
    assert!(!opens_up(high, px(700.0), visible));

    let keyboard = Bounds::new(point(px(0.0), px(60.0)), size(px(320.0), px(340.0)));
    assert!(opens_up(low, px(260.0), keyboard));
}

#[test]
fn stepping_wraps_and_skips_disabled_rows() {
    let rows = [
        Choice::new("a", "A"),
        Choice::new("b", "B").disabled(),
        Choice::new("c", "C"),
    ];
    assert_eq!(step(&rows, 0, 1), 2);
    assert_eq!(step(&rows, 2, 1), 0);
    assert_eq!(step(&rows, 0, -1), 2);
    let none = [Choice::new("a", "A").disabled()];
    assert_eq!(step(&none, 0, 1), 0);
}

#[cfg(feature = "test-support")]
mod popup {
    use std::rc::Rc;

    use gpui::{
        Bounds, Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render, Styled,
        TestAppContext, Window, div, point, px, size,
    };

    use crate::forms::options::{Choice, Pick, Popup};
    use crate::theme::Theme;

    struct Stage {
        picks: Vec<usize>,
        behind: usize,
    }

    impl Render for Stage {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let view = cx.entity();
            let pick: Pick = Rc::new(move |ix, _, cx| {
                view.update(cx, |stage, cx| {
                    stage.picks.push(ix);
                    cx.notify();
                });
            });
            let behind = cx.entity();
            let rows = [Choice::new("a", "A"), Choice::new("b", "B").disabled()];
            div()
                .size_full()
                .child(div().id("behind").absolute().inset_0().on_mouse_down(
                    gpui::MouseButton::Left,
                    move |_, _, cx| {
                        behind.update(cx, |stage, cx| {
                            stage.behind += 1;
                            cx.notify();
                        });
                    },
                ))
                .child(
                    Popup {
                        id: "popup".into(),
                        anchor: Bounds::new(point(px(20.0), px(20.0)), size(px(120.0), px(24.0))),
                        rows: &rows,
                        highlighted: None,
                        checked: None,
                        pick,
                        dismiss: None,
                        scroll: None,
                        reveal: None,
                    }
                    .render(window, cx),
                )
        }
    }

    #[gpui::test]
    fn popup_blocks_clicks_on_disabled_rows_and_padding(cx: &mut TestAppContext) {
        cx.update(Theme::init);
        let (view, cx) = cx.add_window_view(|window, _| {
            window.set_rem_size(px(8.0));
            Stage {
                picks: Vec::new(),
                behind: 0,
            }
        });
        cx.simulate_resize(size(px(320.0), px(480.0)));
        cx.run_until_parked();

        let enabled = cx.debug_bounds("option-row-option-0").expect("first row");
        cx.simulate_click(enabled.center(), Modifiers::none());
        assert_eq!(view.read_with(cx, |stage, _| stage.picks.clone()), [0]);

        let disabled = cx
            .debug_bounds("option-row-option-1")
            .expect("disabled row");
        cx.simulate_click(disabled.center(), Modifiers::none());

        let list = cx.debug_bounds("option-list").expect("popup stays open");
        cx.simulate_click(list.origin + point(px(1.0), px(1.0)), Modifiers::none());

        assert_eq!(
            view.read_with(cx, |stage, _| (stage.picks.clone(), stage.behind)),
            (vec![0], 0)
        );
        assert!(cx.debug_bounds("option-list").is_some());
    }
}
