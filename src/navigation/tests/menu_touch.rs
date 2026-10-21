use super::NavigationMenu;
use crate::{forms::Choice, theme::Theme};
use gpui::{
    Bounds, Context, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Render, Styled,
    TestAppContext, Window, div, point, px, size,
};

struct Menu {
    area: Bounds<Pixels>,
}

impl Render for Menu {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(
            NavigationMenu::new("nav-menu")
                .entry(
                    "File",
                    [
                        Choice::new("new", "New"),
                        Choice::new("open", "Open"),
                        Choice::new("save", "Save"),
                    ],
                )
                .usable_bounds(self.area)
                .on_select(|_, _, _| {}),
        )
    }
}

#[gpui::test]
fn the_open_panel_fits_the_screen_and_its_rows_stay_tappable(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (_, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        Menu {
            area: Bounds::new(point(px(0.), px(0.)), size(px(320.), px(600.))),
        }
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    cx.run_until_parked();
    let title = cx.debug_bounds("navigation-menu-title-0").unwrap();
    let at = title.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    let panel = cx
        .debug_bounds("navigation-menu-panel")
        .expect("the panel is open");
    assert!(
        panel.size.width <= px(320.),
        "the panel is wider than the usable area: {panel:?}"
    );
    assert!(
        panel.size.width <= px(390.),
        "the panel is wider than the screen: {panel:?}"
    );
    for (ix, selector) in [
        (0, "navigation-menu-link-0"),
        (1, "navigation-menu-link-1"),
        (2, "navigation-menu-link-2"),
    ] {
        let row = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("link {ix}"));
        assert!(row.size.height >= px(48.), "link {ix} is {row:?}");
    }
}
