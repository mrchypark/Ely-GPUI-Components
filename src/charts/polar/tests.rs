use super::*;
use crate::theme::Theme;

#[gpui::test]
fn polar_chart_exports_csv_and_svg(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let chart = PolarChart::new("wind")
            .slice("N", 12.0)
            .slice("E", 24.0)
            .slice("S", 8.0);
        assert_eq!(chart.csv(), "name,value\nN,12\nE,24\nS,8");
        let svg = chart.svg(240.0, 240.0, cx);
        assert!(svg.starts_with("<svg") && svg.matches("<path").count() == 3);
    });
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn polar_chart_supports_desktop_and_touch_pin_rotate_pinch_and_reset(
    cx: &mut gpui::TestAppContext,
) {
    use crate::charts::tests::settle;
    use gpui::{Context, Modifiers, PinchEvent, Render, TouchPhase, Window, point, px};

    struct Harness;
    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(320.0)).h(px(280.0)).child(
                PolarChart::new("polar-ui")
                    .slice("A", 10.0)
                    .slice("B", 20.0)
                    .slice("C", 15.0),
            )
        }
    }

    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |t| t.touch_mode = true);
    });
    let (_, vc) = cx.add_window_view(|_, _| Harness);
    settle(vc);

    let plot = vc.debug_bounds("polar-plot").expect("polar plot");
    let tap = point(plot.center().x, plot.center().y - px(30.0));
    vc.simulate_mouse_down(tap, MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(tap, MouseButton::Left, Modifiers::none());
    settle(vc);

    vc.simulate_event(PinchEvent {
        position: plot.center(),
        delta: 0.3,
        modifiers: Modifiers::none(),
        phase: TouchPhase::Moved,
    });
    settle(vc);

    vc.simulate_mouse_down(plot.center(), MouseButton::Left, Modifiers::none());
    vc.simulate_mouse_up(plot.center(), MouseButton::Left, Modifiers::none());
}
