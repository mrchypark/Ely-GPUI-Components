use super::MenuBar;
use crate::{
    menus::tests::{press, settle},
    primitives::FocusScope,
    theme::Theme,
};
use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers, OwnedMenu,
    OwnedMenuItem, ParentElement, Pixels, Render, Styled, TestAppContext, VisualTestContext,
    Window, actions, div, point, px, size,
};

actions!(bounded_menu_bar_test, [Run]);

struct Bar {
    root: FocusHandle,
    before: FocusHandle,
    area: Bounds<Pixels>,
    ran: usize,
}

fn item(name: impl Into<gpui::SharedString>) -> OwnedMenuItem {
    OwnedMenuItem::Action {
        name: name.into().to_string(),
        action: Box::new(Run),
        os_action: None,
        checked: false,
        disabled: false,
    }
}

impl Render for Bar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows: Vec<_> = (0..25).map(|ix| item(format!("Row {ix}"))).collect();
        let mut children: Vec<_> = (0..25).map(|ix| item(format!("Child {ix}"))).collect();
        children.push(item("Last"));
        rows.push(OwnedMenuItem::Submenu(OwnedMenu {
            name: "More".into(),
            items: children,
            disabled: false,
        }));
        let menus = vec![
            OwnedMenu {
                name: "File".into(),
                items: rows,
                disabled: false,
            },
            OwnedMenu {
                name: "Edit".into(),
                items: vec![item("Other")],
                disabled: false,
            },
        ];
        div()
            .size_full()
            .on_action(cx.listener(|bar, _: &Run, _, _| bar.ran += 1))
            .child(
                FocusScope::new(&self.root)
                    .size_full()
                    .child(div().track_focus(&self.before))
                    .child(MenuBar::new("bounded-bar", menus).usable_bounds(self.area)),
            )
    }
}

fn fixture(cx: &mut TestAppContext) -> (Entity<Bar>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.reduced_motion = true;
            theme.touch_mode = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, cx| Bar {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        ran: 0,
        area: Bounds::new(point(px(24.), px(40.)), size(px(330.), px(600.))),
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    settle(cx);
    let before = view.read_with(cx, |view, _| view.before.clone());
    cx.update(|window, cx| window.focus(&before, cx));
    (view, cx)
}

fn click_title(cx: &mut VisualTestContext, selector: &'static str) {
    let position = cx.debug_bounds(selector).unwrap().center();
    cx.simulate_mouse_move(position, None, Modifiers::none());
    cx.simulate_click(position, Modifiers::none());
    settle(cx);
}

fn contained(cx: &mut VisualTestContext, selector: &'static str, area: Bounds<Pixels>) {
    let panel = cx.debug_bounds(selector).expect("open menu");
    assert!(panel.size.width > px(0.) && panel.size.height > px(0.));
    assert!(
        panel.left() >= area.left() && panel.right() <= area.right(),
        "{panel:?}"
    );
    assert!(
        panel.top() >= area.top() && panel.bottom() <= area.bottom(),
        "{panel:?}"
    );
}

#[gpui::test]
fn bar_root_submenu_and_other_title_follow_live_bounds(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx);
    click_title(cx, "menu-bar-title-0");
    let original = view.read_with(cx, |view, _| view.area);
    contained(cx, "menu-panel-0", original);
    press("up", cx);
    press("right", cx);
    press("up", cx);
    contained(cx, "menu-panel-1", original);
    let small = Bounds::new(point(px(31.), px(67.)), size(px(175.), px(120.)));
    view.update(cx, |view, cx| {
        view.area = small;
        cx.notify();
    });
    settle(cx);
    for selector in ["menu-panel-0", "menu-panel-1"] {
        contained(cx, selector, small);
    }
    // Navigate after resizing to reveal the far action in the new scroll viewport.
    press("down", cx);
    press("up", cx);
    let row = cx.debug_bounds("menu-row-Last").unwrap();
    let scroll = cx.debug_bounds("menu-scroll-1").unwrap();
    assert!(row.top() >= scroll.top() && row.bottom() <= scroll.bottom());
    press("enter", cx);
    assert_eq!(view.read_with(cx, |view, _| view.ran), 1);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    click_title(cx, "menu-bar-title-1");
    contained(cx, "menu-panel-0", small);
    press("down", cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |view, _| view.ran), 2);
}

#[gpui::test]
fn bar_no_room_closes_restores_focus_and_requires_fresh_activation(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx);
    let normal = view.read_with(cx, |view, _| view.area);
    let before = view.read_with(cx, |view, _| view.before.clone());
    for area in [
        Bounds::new(point(px(24.), px(40.)), size(px(0.), px(200.))),
        Bounds::new(point(px(24.), px(40.)), size(px(200.), px(0.))),
        Bounds::new(point(px(24.), px(40.)), size(px(1.), px(1.))),
        Bounds::new(point(px(500.), px(900.)), size(px(200.), px(200.))),
    ] {
        click_title(cx, "menu-bar-title-0");
        contained(cx, "menu-panel-0", normal);
        view.update(cx, |view, cx| {
            view.area = area;
            cx.notify();
        });
        settle(cx);
        assert!(cx.debug_bounds("menu-panel-0").is_none());
        assert!(cx.update(|window, _| before.is_focused(window)));
        click_title(cx, "menu-bar-title-0");
        assert!(cx.debug_bounds("menu-panel-0").is_none());
        assert!(cx.update(|window, _| before.is_focused(window)));
        // Keyboard activation on the bar must also be blocked.
        cx.update(|window, cx| window.focus_next(cx));
        press("down", cx);
        assert!(cx.debug_bounds("menu-panel-0").is_none());
        cx.update(|window, cx| window.focus(&before, cx));
        view.update(cx, |view, cx| {
            view.area = normal;
            cx.notify();
        });
        settle(cx);
        assert!(cx.debug_bounds("menu-panel-0").is_none());
    }
    click_title(cx, "menu-bar-title-0");
    contained(cx, "menu-panel-0", normal);
    press("escape", cx);
    assert!(cx.update(|window, _| before.is_focused(window)));
    assert_eq!(view.read_with(cx, |view, _| view.ran), 0);
}
