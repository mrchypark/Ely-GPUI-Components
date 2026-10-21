use super::{ActionBar, Button, IconButton};
use crate::{
    primitives::IconName,
    theme::{ControlSize, Density, Theme},
};
use gpui::{
    Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px,
};

struct TouchControls {
    clicks: usize,
}
impl Render for TouchControls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ActionBar::new()
            .child(
                div().flex().debug_selector(|| "touch-button".into()).child(
                    Button::new("touch-button", "x")
                        .size(ControlSize::Sm)
                        .on_click(cx.listener(|view, _, _, _| view.clicks += 1)),
                ),
            )
            .child(
                div()
                    .flex()
                    .debug_selector(|| "touch-icon".into())
                    .child(IconButton::new("touch-icon", IconName::Check).size(ControlSize::Sm)),
            )
    }
}

#[gpui::test]
fn touch_targets_survive_small_rem_and_restore_desktop_density(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.density = Density::Compact;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        TouchControls { clicks: 0 }
    });
    cx.run_until_parked();
    let desktop = cx.debug_bounds("touch-icon").expect("desktop icon");
    assert_eq!(desktop.size.height, px(10.));
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    cx.run_until_parked();
    let button = cx.debug_bounds("touch-button").expect("touch button");
    let icon = cx.debug_bounds("touch-icon").expect("touch icon");
    for bounds in [button, icon] {
        assert!(bounds.size.width >= px(48.) && bounds.size.height >= px(48.));
    }
    assert!(icon.left() - button.right() >= px(8.));
    cx.simulate_click(
        point(button.right() - px(2.), button.bottom() - px(2.)),
        Modifiers::none(),
    );
    assert_eq!(view.read_with(cx, |view, _| view.clicks), 1);
    cx.update(|window, _| {
        window.set_rem_size(px(48.));
        window.refresh();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("touch-icon").unwrap().size.height >= px(60.));
    cx.update(|window, cx| {
        window.set_rem_size(px(8.));
        Theme::update(cx, |theme| theme.touch_mode = false);
    });
    cx.run_until_parked();
    assert_eq!(cx.debug_bounds("touch-icon").unwrap().size, desktop.size);
}

struct TouchToggles;
impl Render for TouchToggles {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use super::{ToggleButton, ToggleGroup, ToggleItem};
        div()
            .flex()
            .flex_col()
            .items_start()
            .child(
                ToggleButton::new(
                    "single",
                    ToggleItem::new("single").icon(IconName::Check),
                    false,
                )
                .size(ControlSize::Sm),
            )
            .child(
                ToggleGroup::new("group")
                    .size(ControlSize::Sm)
                    .item(ToggleItem::new("first").icon(IconName::Check))
                    .item(ToggleItem::new("second").icon(IconName::Check)),
            )
    }
}

#[gpui::test]
fn touch_toggles_preserve_large_rem_minimums(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.density = Density::Compact;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|_, _| TouchToggles);
    for (rem, minimum) in [(8., 48.), (48., 60.)] {
        cx.update(|window, _| {
            window.set_rem_size(px(rem));
            window.refresh();
        });
        cx.run_until_parked();
        for selector in ["toggle single", "toggle first", "toggle second"] {
            let bounds = cx.debug_bounds(selector).expect("toggle target");
            assert!(
                bounds.size.width >= px(minimum),
                "{selector} width at rem={rem}"
            );
            assert!(
                bounds.size.height >= px(minimum),
                "{selector} height at rem={rem}"
            );
        }
        let first = cx.debug_bounds("toggle first").unwrap();
        let second = cx.debug_bounds("toggle second").unwrap();
        assert!(second.left() - first.right() >= px(8.));
    }
}
