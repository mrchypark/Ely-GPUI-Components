use gpui::{
    Bounds, Context, Entity, FocusHandle, IntoElement, Modifiers, ParentElement, Pixels, Render,
    ScrollDelta, ScrollWheelEvent, Styled, TestAppContext, TouchPhase, VisualTestContext, Window,
    point, px, size,
};

use super::{press, settle};
use crate::{
    menus::{DropdownMenu, Menu, MenuItem, SearchableMenu},
    primitives::FocusScope,
    theme::Theme,
};

struct LongMenu {
    focus: FocusHandle,
    searchable: bool,
    picked: bool,
    usable: Option<Bounds<Pixels>>,
}

struct NearBottomMenu {
    focus: FocusHandle,
}

impl Render for NearBottomMenu {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let menu = Menu::new()
            .item(MenuItem::new("First"))
            .item(MenuItem::new("Second"))
            .item(MenuItem::new("Third"))
            .item(MenuItem::new("Fourth"));
        FocusScope::new(&self.focus).size_full().child(
            gpui::div().absolute().top(px(360.)).left(px(20.))
                .child(DropdownMenu::new("near-bottom", "Actions", menu)),
        )
    }
}

#[gpui::test]
fn touch_menu_near_bottom_opens_above_its_host(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
        crate::forms::bind_keys(cx);
    });
    let (_, cx) = cx.add_window_view(|_, cx| NearBottomMenu { focus: cx.focus_handle() });
    cx.simulate_resize(size(px(390.), px(600.)));
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    let panel = cx.debug_bounds("menu-panel-0").expect("open menu");
    assert!(panel.top() >= px(0.) && panel.bottom() <= px(360.), "{panel:?}");
    assert!(panel.left() >= px(0.) && panel.right() <= px(390.), "{panel:?}");
    for selector in ["menu-row-First", "menu-row-Second", "menu-row-Third", "menu-row-Fourth"] {
        let row = cx.debug_bounds(selector).expect(selector);
        assert!(row.size.width >= px(48.) && row.size.height >= px(48.), "{row:?}");
        assert!(row.left() >= panel.left() && row.right() <= panel.right()
            && row.top() >= panel.top() && row.bottom() <= panel.bottom(), "{row:?}");
    }
    press("escape", cx);
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    press("enter", cx);
    let desktop = cx.debug_bounds("menu-panel-0").expect("desktop menu");
    assert!(desktop.top() >= px(388.) && desktop.bottom() <= px(600.), "{desktop:?}");
    let row = cx.debug_bounds("menu-row-First").unwrap();
    assert_eq!(row.size.height, px(28.));
}

impl Render for LongMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = Menu::new();
        let mut child = Menu::new();
        for ix in 0..30 {
            root = root.item(MenuItem::new(format!("Root {ix}")));
            child = child.item(MenuItem::new(format!("Child {ix}")));
        }
        let owner = cx.entity();
        child = child
            .separator()
            .item(MenuItem::new("Last item").on_click(move |_, cx| {
                owner.update(cx, |view, _| view.picked = true);
            }));
        root = root.separator().item(MenuItem::submenu("More", child));
        if let Some(bounds) = self.usable {
            root = root.usable_bounds(bounds);
        }
        let host = if self.searchable {
            SearchableMenu::new("long", "Long", root).into_any_element()
        } else {
            DropdownMenu::new("long", "Long", root).into_any_element()
        };
        FocusScope::new(&self.focus).size_full().child(host)
    }
}

fn open(searchable: bool, cx: &mut TestAppContext) -> (Entity<LongMenu>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
        crate::forms::bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(|_, cx| LongMenu {
        focus: cx.focus_handle(),
        searchable,
        picked: false,
        usable: None,
    });
    cx.simulate_resize(size(px(390.), px(240.)));
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    (view, cx)
}

fn inside(row: Bounds<Pixels>, frame: Bounds<Pixels>) {
    assert!(row.top() >= frame.top() && row.bottom() <= frame.bottom());
}

#[gpui::test]
fn long_menu_reveals_keyboard_rows_and_submenu_items(cx: &mut TestAppContext) {
    let (view, cx) = open(false, cx);
    let panel = cx.debug_bounds("menu-panel-0").expect("root panel");
    assert!(panel.top() >= px(0.) && panel.bottom() <= px(240.));
    press("up", cx);
    inside(
        cx.debug_bounds("menu-row-More").expect("last root row"),
        cx.debug_bounds("menu-scroll-0").expect("root scroll"),
    );
    press("right", cx);
    press("up", cx);
    let child = cx.debug_bounds("menu-panel-1").expect("child panel");
    assert!(child.top() >= px(0.) && child.bottom() <= px(240.));
    inside(
        cx.debug_bounds("menu-row-Last item")
            .expect("last child item"),
        cx.debug_bounds("menu-scroll-1").expect("child scroll"),
    );
    press("left", cx);
    assert!(cx.debug_bounds("menu-panel-1").is_none());
    inside(
        cx.debug_bounds("menu-row-More")
            .expect("parent anchor retained"),
        cx.debug_bounds("menu-scroll-0").expect("root scroll"),
    );
    press("right", cx);
    press("up", cx);
    press("enter", cx);
    assert!(view.read_with(cx, |view, _| view.picked));
}

#[gpui::test]
fn searchable_menu_scrolls_rows_without_moving_its_header(cx: &mut TestAppContext) {
    let (_, cx) = open(true, cx);
    let frame = cx.debug_bounds("menu-scroll-0").expect("root scroll");
    let panel = cx.debug_bounds("menu-panel-0").expect("root panel");
    press("up", cx);
    inside(
        cx.debug_bounds("menu-row-More").expect("last root row"),
        frame,
    );
    assert_eq!(cx.debug_bounds("menu-scroll-0"), Some(frame));
    assert_eq!(cx.debug_bounds("menu-panel-0"), Some(panel));
    let more = cx
        .debug_bounds("menu-row-More")
        .expect("submenu trigger")
        .center();
    cx.simulate_click(more, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("menu-panel-1").is_some());
    cx.simulate_event(ScrollWheelEvent {
        position: point(frame.left() + px(2.), frame.top() + px(2.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(2000.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    assert!(cx.debug_bounds("menu-panel-1").is_none());
    inside(
        cx.debug_bounds("menu-row-Root 0").expect("first root row"),
        frame,
    );
    assert_eq!(cx.debug_bounds("menu-scroll-0"), Some(frame));
    press("down", cx);
    inside(
        cx.debug_bounds("menu-row-Root 0")
            .expect("wrapped keyboard row"),
        frame,
    );
}
struct ShortPanels;
struct TinyPanelMask;
impl Render for TinyPanelMask {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::{canvas, div};
        div().children([0., 1.].map(|extent| {
            let bounds = Bounds::new(point(px(20.), px(24.)), size(px(extent), px(extent)));
            let check = canvas(
                move |_, window, _| {
                    assert_eq!(window.content_mask().bounds, bounds);
                },
                move |_, _, window, _| {
                    assert_eq!(window.content_mask().bounds, bounds);
                },
            )
            .size(px(40.));
            crate::menus::placement::bounded(
                bounds,
                bounds.origin,
                div().p_1().border_1().child(check),
                window,
                cx,
            )
        }))
    }
}
#[gpui::test]
fn tiny_and_zero_panels_mask_prepaint_and_paint(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(|_, _| TinyPanelMask);
    settle(cx);
}
#[gpui::test]
fn tiny_and_zero_menu_preserve_escape_close(cx: &mut TestAppContext) {
    let (view, cx) = open(false, cx);
    for extent in [1., 0.] {
        view.update(cx, |view, cx| {
            view.usable = Some(Bounds::new(
                point(px(20.), px(24.)),
                size(px(extent), px(extent)),
            ));
            cx.notify();
        });
        settle(cx);
        assert!(cx.debug_bounds("menu-panel-0").is_some());
        press("escape", cx);
        assert!(cx.debug_bounds("menu-panel-0").is_none());
        press("enter", cx);
    }
    press("escape", cx);
}
impl Render for ShortPanels {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::{InteractiveElement, div};
        let usable = Bounds::new(point(px(20.), px(24.)), size(px(350.), px(300.)));
        div().size_full().children([
            crate::menus::placement::bounded(
                usable,
                point(px(20.), px(150.)),
                div()
                    .w(px(100.))
                    .h(px(60.))
                    .debug_selector(|| "short-root".into()),
                window,
                cx,
            ),
            crate::menus::placement::bounded(
                usable,
                point(px(210.), px(180.)),
                div()
                    .w(px(100.))
                    .h(px(60.))
                    .debug_selector(|| "short-sub".into()),
                window,
                cx,
            ),
        ])
    }
}
#[gpui::test]
fn short_root_and_submenu_keep_actual_layout_anchor(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(|_, _| ShortPanels);
    cx.simulate_resize(size(px(390.), px(400.)));
    settle(cx);
    assert_eq!(
        cx.debug_bounds("short-root").expect("root").origin,
        point(px(20.), px(150.))
    );
    assert_eq!(
        cx.debug_bounds("short-sub").expect("submenu").origin,
        point(px(210.), px(180.))
    );
}
#[gpui::test]
fn menu_and_submenu_respect_host_usable_bounds(cx: &mut TestAppContext) {
    let (view, cx) = open(false, cx);
    let usable = Bounds::new(point(px(20.), px(24.)), size(px(170.), px(150.)));
    view.update(cx, |view, cx| {
        view.usable = Some(usable);
        cx.notify();
    });
    settle(cx);
    press("up", cx);
    press("right", cx);
    press("up", cx);
    for selector in ["menu-panel-0", "menu-panel-1"] {
        let panel = cx.debug_bounds(selector).expect("panel");
        inside(panel, usable);
        assert!(panel.left() >= usable.left() && panel.right() <= usable.right());
    }
    inside(
        cx.debug_bounds("menu-row-Last item")
            .expect("last child item"),
        cx.debug_bounds("menu-scroll-1").expect("child scroll"),
    );
}

struct NestedMenuScroll {
    focus: FocusHandle,
    scroll: gpui::ScrollHandle,
    wheels: usize,
}

impl Render for NestedMenuScroll {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::{InteractiveElement, StatefulInteractiveElement, div};
        let mut menu = Menu::new();
        for ix in 0..30 {
            menu = menu.item(MenuItem::new(format!("Nested {ix}")));
        }
        FocusScope::new(&self.focus).size_full().child(
            div()
                .id("nested-controls")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.scroll)
                .on_scroll_wheel(cx.listener(|view, _, _, _| view.wheels += 1))
                .child(DropdownMenu::new("nested-menu", "Menu", menu))
                .child(div().h(px(1200.))),
        )
    }
}

#[gpui::test]
fn menu_scroll_does_not_scroll_ancestor_even_at_boundary(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
        crate::forms::bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(|_, cx| NestedMenuScroll {
        focus: cx.focus_handle(),
        scroll: gpui::ScrollHandle::new(),
        wheels: 0,
    });
    cx.simulate_resize(size(px(390.), px(240.)));
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    let frame = cx.debug_bounds("menu-scroll-0").expect("menu rows");
    let before = cx.debug_bounds("menu-row-Nested 29").expect("last row");
    let outer = view.read_with(cx, |view, _| view.scroll.offset());
    for _ in 0..2 {
        cx.simulate_event(ScrollWheelEvent {
            position: frame.center(),
            delta: ScrollDelta::Pixels(point(px(0.), px(-2000.))),
            modifiers: Modifiers::none(),
            touch_phase: TouchPhase::Moved,
        });
        settle(cx);
        assert_eq!(view.read_with(cx, |view, _| view.scroll.offset()), outer);
        assert_eq!(view.read_with(cx, |view, _| view.wheels), 0);
    }
    let after = cx.debug_bounds("menu-row-Nested 29").expect("last row");
    assert!(after.top() < before.top(), "menu itself must still scroll");
    inside(after, frame);
    let panel = cx.debug_bounds("menu-panel-0").expect("panel");
    cx.simulate_event(ScrollWheelEvent {
        position: point(panel.left() + px(1.), panel.top() + px(1.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(-100.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    assert_eq!(view.read_with(cx, |view, _| view.scroll.offset()), outer);
    assert_eq!(view.read_with(cx, |view, _| view.wheels), 0);
    press("escape", cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    cx.simulate_event(ScrollWheelEvent {
        position: frame.center(),
        delta: ScrollDelta::Pixels(point(px(0.), px(-100.))),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    });
    settle(cx);
    assert!(view.read_with(cx, |view, _| view.scroll.offset().y) < outer.y);
    assert!(view.read_with(cx, |view, _| view.wheels) > 0);
}
