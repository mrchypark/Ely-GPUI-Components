use gpui::{
    App, Bounds, Context, FocusHandle, InteractiveElement, IntoElement, KeyUpEvent, Keystroke,
    Modifiers, MouseButton, ParentElement, Pixels, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, div, point, prelude::*, px, size,
};
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

mod more;
mod overlay;
mod scroll;

use super::{ContextMenu, DropdownMenu, Menu, MenuDismiss, MenuItem, SearchableMenu};
use crate::{primitives::FocusScope, theme::Theme};

/// What the rows did, and the state the check and radio rows show.
struct Desk {
    root: FocusHandle,
    ran: Vec<SharedString>,
    wrap: bool,
    theme: SharedString,
    view_rows: bool,
    enters_above: usize,
    shift: bool,
    asked: Option<(u64, gpui::Point<gpui::Pixels>)>,
}

impl Desk {
    fn menu(&self, cx: &mut Context<Self>) -> Menu {
        let run = |name: &'static str| {
            let view = cx.entity();
            move |_: &mut Window, cx: &mut App| {
                view.update(cx, |desk, cx| {
                    desk.ran.push(name.into());
                    cx.notify();
                })
            }
        };
        let (wrap, light, dark) = (cx.entity(), cx.entity(), cx.entity());
        let theme = Menu::new()
            .item(
                MenuItem::radio("Light", self.theme == "light")
                    .on_click(move |_, cx| light.update(cx, |desk, _| desk.theme = "light".into())),
            )
            .item(
                MenuItem::radio("Dark", self.theme == "dark")
                    .on_click(move |_, cx| dark.update(cx, |desk, _| desk.theme = "dark".into())),
            );
        let edit = Menu::new()
            .item(
                MenuItem::new("Cut")
                    .keys("secondary-x")
                    .on_click(run("cut")),
            )
            .item(MenuItem::new("Copy").disabled(true).on_click(run("copy")))
            .item(MenuItem::new("Paste").on_click(run("paste")));
        if !self.view_rows {
            return edit;
        }
        edit.group(
            "View",
            [
                MenuItem::check("Word wrap", self.wrap)
                    .on_click(move |_, cx| wrap.update(cx, |desk, _| desk.wrap = !desk.wrap)),
                MenuItem::submenu("Theme", theme),
            ],
        )
    }
}

impl Render for Desk {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.menu(cx);
        let above = cx.entity();
        div()
            .size_full()
            .on_key_down(move |event, _, cx| {
                if event.keystroke.key == "enter" {
                    above.update(cx, |desk, _| desk.enters_above += 1);
                }
            })
            .child(
                FocusScope::new(&self.root)
                    .size_full()
                    .when(self.shift, |scope| scope.child(div().h(px(80.0))))
                    .child(DropdownMenu::new("edit", "Edit", menu.clone()))
                    .child(
                        ContextMenu::new("area", menu.clone())
                            .child(div().w(px(200.0)).h(px(120.0))),
                    )
                    .child(SearchableMenu::new("find", "Find", menu.clone()))
                    .child(
                        ContextMenu::new("late", menu).manual(self.asked).child(
                            div()
                                .debug_selector(|| "late".into())
                                .w(px(200.0))
                                .h(px(40.0)),
                        ),
                    ),
            )
    }
}

pub(super) fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// Presses and releases `key`, with a frame in between as on a real keyboard.
pub(super) fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).expect("a key"),
    });
    settle(cx);
}

fn desk(cx: &mut TestAppContext) -> (gpui::Entity<Desk>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Desk {
        root: cx.focus_handle(),
        ran: Vec::new(),
        wrap: false,
        theme: "light".into(),
        view_rows: true,
        enters_above: 0,
        shift: false,
        asked: None,
    });
    settle(cx);
    (view, cx)
}

/// The same desk with touch mode on, where a long press opens the context menu.
fn touch_desk(cx: &mut TestAppContext) -> (gpui::Entity<Desk>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Desk {
        root: cx.focus_handle(),
        ran: Vec::new(),
        wrap: false,
        theme: "light".into(),
        view_rows: true,
        enters_above: 0,
        shift: false,
        asked: None,
    });
    settle(cx);
    (view, cx)
}

fn ran(view: &gpui::Entity<Desk>, cx: &mut VisualTestContext) -> Vec<SharedString> {
    view.read_with(cx, |desk, _| desk.ran.clone())
}

fn click(x: f32, y: f32, cx: &mut VisualTestContext) {
    let at = point(px(x), px(y));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn the_keyboard_opens_walks_past_disabled_rows_and_runs(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"]);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"], "the release left the menu shut");
}

#[gpui::test]
fn submenus_open_right_and_close_left(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("up", cx);
    press("right", cx);
    press("down", cx);
    press("left", cx);
    press("right", cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |desk, _| desk.theme.clone()), "dark");
}

#[gpui::test]
fn a_check_row_reports_to_its_owner(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("up", cx);
    press("up", cx);
    press("enter", cx);
    assert!(view.read_with(cx, |desk, _| desk.wrap));
}

#[gpui::test]
fn escape_backs_out_one_level_then_closes(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("up", cx);
    press("right", cx);
    press("escape", cx);
    press("up", cx);
    press("enter", cx);
    assert!(
        view.read_with(cx, |desk, _| desk.wrap),
        "one escape left the root open"
    );
    press("enter", cx);
    press("escape", cx);
    press("down", cx);
    press("enter", cx);
    assert!(
        ran(&view, cx).is_empty(),
        "the second escape closed the menu"
    );
}

#[gpui::test]
fn a_right_click_opens_the_context_menu_where_it_lands(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    let at = point(px(60.0), px(80.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    settle(cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
}

#[gpui::test]
fn a_long_press_opens_the_context_menu_on_touch(cx: &mut TestAppContext) {
    let (view, cx) = touch_desk(cx);
    let at = point(px(60.0), px(80.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.executor().advance_clock(Duration::from_millis(600));
    settle(cx);
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
}

#[gpui::test]
fn a_short_press_leaves_the_context_menu_shut_on_touch(cx: &mut TestAppContext) {
    let (view, cx) = touch_desk(cx);
    let at = point(px(60.0), px(80.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    press("down", cx);
    press("enter", cx);
    assert!(
        ran(&view, cx).is_empty(),
        "a short press opened the context menu"
    );
}

#[gpui::test]
fn a_manual_context_menu_opens_only_from_its_owner(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    let at = cx.debug_bounds("late").expect("the host").center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    settle(cx);
    press("down", cx);
    press("enter", cx);
    assert!(ran(&view, cx).is_empty(), "a right click opens nothing");
    view.update(cx, |desk, cx| {
        desk.asked = Some((1, point(px(60.0), px(80.0))));
        cx.notify();
    });
    settle(cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
}

#[gpui::test]
fn the_trigger_toggles_and_a_press_outside_closes(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    click(12.0, 12.0, cx);
    click(12.0, 12.0, cx);
    press("down", cx);
    press("enter", cx);
    assert!(
        ran(&view, cx).is_empty(),
        "the second press on the trigger closed it"
    );
    click(12.0, 12.0, cx);
    click(600.0, 400.0, cx);
    press("down", cx);
    press("enter", cx);
    assert!(ran(&view, cx).is_empty(), "the press outside closed it");
}

#[gpui::test]
fn rows_the_owner_removes_leave_the_menu_usable(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("up", cx);
    view.update(cx, |desk, cx| {
        desk.view_rows = false;
        cx.notify();
    });
    settle(cx);
    press("enter", cx);
    assert!(ran(&view, cx).is_empty(), "the removed row ran");
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
}

#[gpui::test]
fn closing_from_the_trigger_hands_focus_back(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    click(12.0, 12.0, cx);
    press("enter", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"], "focus came back to the trigger");
}

#[gpui::test]
fn confirm_keys_stay_inside_the_menu(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    click(12.0, 12.0, cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
    assert_eq!(view.read_with(cx, |desk, _| desk.enters_above), 0);
}

#[gpui::test]
fn modified_confirm_keys_do_not_pick(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("ctrl-enter", cx);
    press("shift-space", cx);
    assert!(ran(&view, cx).is_empty(), "a modified key picked");
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
}

#[gpui::test]
fn a_second_right_click_moves_the_context_menu(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    cx.update(|window, cx| window.focus_next(cx));
    for at in [point(px(150.0), px(120.0)), point(px(20.0), px(40.0))] {
        cx.simulate_mouse_move(at, None, Modifiers::none());
        cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
        settle(cx);
    }
    press("down", cx);
    press("enter", cx);
    assert_eq!(
        ran(&view, cx),
        ["cut"],
        "the menu stayed open at the second click"
    );
}

#[gpui::test]
fn a_left_press_in_the_region_closes_the_context_menu(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    let at = point(px(150.0), px(120.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    settle(cx);
    click(20.0, 40.0, cx);
    press("down", cx);
    press("enter", cx);
    assert!(ran(&view, cx).is_empty(), "the left press closed the menu");
}

fn open_find(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
    });
    press("enter", cx);
}

#[gpui::test]
fn the_filter_runs_the_best_fit(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    open_find(cx);
    cx.simulate_input("pa");
    settle(cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"]);
}

#[gpui::test]
fn space_types_into_the_filter(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    open_find(cx);
    cx.simulate_input("pa");
    settle(cx);
    press("space", cx);
    assert!(ran(&view, cx).is_empty(), "space picked a row");
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"]);
}

#[gpui::test]
fn arrows_walk_rows_from_the_filter(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    open_find(cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"]);
}

#[gpui::test]
fn an_open_dropdown_follows_its_host(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    click(12.0, 12.0, cx);
    view.update(cx, |desk, cx| {
        desk.shift = true;
        cx.notify();
    });
    settle(cx);
    settle(cx);
    cx.simulate_mouse_move(point(px(40.0), px(131.0)), None, Modifiers::none());
    settle(cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"], "the panel moved with its button");
}

#[gpui::test]
fn clearing_the_filter_marks_the_first_row_again(cx: &mut TestAppContext) {
    let (view, cx) = desk(cx);
    open_find(cx);
    cx.simulate_input("t");
    settle(cx);
    press("down", cx);
    press("backspace", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"]);
}

#[test]
fn filtering_preserves_open_observer_even_without_matches() {
    use std::rc::Rc;
    let menu = Menu::new()
        .on_open_change(|_| {})
        .item(MenuItem::new("Alpha"));
    for query in ["", "Al", "zzzz"] {
        let filtered = menu.filtered(query);
        assert!(Rc::ptr_eq(
            menu.on_open_change.as_ref().unwrap(),
            filtered.on_open_change.as_ref().unwrap(),
        ));
    }
}

#[gpui::test]
fn open_observer_tracks_sessions_not_submenus_or_restarts(cx: &mut TestAppContext) {
    use super::menu::{Open, Spot, mark};
    use std::{cell::RefCell, rc::Rc};
    let (_, cx) = desk(cx);
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let menu = Menu::new()
        .on_open_change(move |open| observed.borrow_mut().push(open))
        .item(MenuItem::submenu(
            "Child",
            Menu::new().item(MenuItem::new("Leaf")),
        ));
    let state = cx.update(|_, cx| cx.new(|_| Open::default()));
    cx.update(|window, cx| {
        Open::close(&state, window, cx);
        Open::show(&state, &menu, Spot::Under, true, cx);
        mark(&state, &menu, 0, 0, true, cx);
        Open::restart(&state, &menu, cx);
        // Repositioning or replacing rendered rows retains the session's observer.
        Open::show(&state, &Menu::new(), Spot::Under, false, cx);
        assert_eq!(*events.borrow(), [true]);
        Open::close(&state, window, cx);
        Open::close(&state, window, cx);
        assert_eq!(*events.borrow(), [true, false]);
        Open::show(&state, &menu, Spot::Under, true, cx);
        Open::close(&state, window, cx);
    });
    drop(state);
    cx.run_until_parked();
    assert_eq!(*events.borrow(), [true, false, true, false]);
}

#[gpui::test]
fn dropping_open_menu_releases_observer_once(cx: &mut TestAppContext) {
    use super::menu::{Open, Spot};
    use std::{cell::RefCell, rc::Rc};
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let menu = Menu::new().on_open_change(move |open| observed.borrow_mut().push(open));
    let state = cx.new(|_| Open::default());
    cx.update(|cx| Open::show(&state, &menu, Spot::Under, false, cx));
    assert_eq!(*events.borrow(), [true]);
    drop(state);
    cx.run_until_parked();
    // Entity values are released at App update completion, not by executor draining.
    cx.update(|_| {});
    assert_eq!(*events.borrow(), [true, false]);
    cx.update(|_| {});
    cx.run_until_parked();
    assert_eq!(*events.borrow(), [true, false]);
}

/// The panel boxes the observer was told about, in order.
type Seen = Rc<RefCell<Vec<Vec<Bounds<Pixels>>>>>;

/// A menu whose panel boxes are recorded, with a handle to close it.
fn watched_menu() -> (Menu, Seen, MenuDismiss) {
    let seen: Seen = Rc::new(RefCell::new(Vec::new()));
    let observed = seen.clone();
    let menu = Menu::new()
        .on_panels_change(move |panels| observed.borrow_mut().push(panels.to_vec()))
        .item(MenuItem::submenu(
            "Child",
            Menu::new().item(MenuItem::new("Leaf")),
        ));
    let dismiss = menu.dismiss_handle();
    (menu, seen, dismiss)
}

/// A panel box of its own.
fn panel_box(x: f32, y: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(120.0), px(80.0)))
}

#[gpui::test]
fn the_panel_observer_follows_the_open_panels(cx: &mut TestAppContext) {
    use super::menu::{Open, Spot};
    let (menu, seen, _) = watched_menu();
    let cx = cx.add_empty_window();
    let state = cx.update(|_, cx| cx.new(|_| Open::default()));
    cx.update(|window, cx| {
        Open::show(&state, &menu, Spot::Under, false, cx);
        // The panel is measured where it is drawn; here its box is set directly.
        let root = panel_box(10.0, 20.0);
        state.update(cx, |open, _| {
            open.levels[0].panel = root;
            open.notify_panels();
        });
        assert_eq!(*seen.borrow(), [vec![root]], "the panel was not reported");
        Open::close(&state, window, cx);
        assert_eq!(
            *seen.borrow(),
            [vec![root], vec![]],
            "the close went unseen"
        );
    });
}

#[gpui::test]
fn the_panel_observer_is_quiet_while_the_bounds_hold(cx: &mut TestAppContext) {
    use super::menu::{Open, Spot};
    let (menu, seen, _) = watched_menu();
    let cx = cx.add_empty_window();
    let state = cx.update(|_, cx| cx.new(|_| Open::default()));
    cx.update(|window, cx| {
        Open::show(&state, &menu, Spot::Under, false, cx);
        let root = panel_box(10.0, 20.0);
        state.update(cx, |open, _| {
            open.levels[0].panel = root;
            open.notify_panels();
        });
        state.update(cx, |open, _| open.notify_panels());
        assert_eq!(seen.borrow().len(), 1, "the same panel was reported twice");
        let moved = panel_box(30.0, 40.0);
        state.update(cx, |open, _| {
            open.levels[0].panel = moved;
            open.notify_panels();
        });
        assert_eq!(
            *seen.borrow(),
            [vec![root], vec![moved]],
            "a move went unseen"
        );
        Open::close(&state, window, cx);
    });
}

#[gpui::test]
fn a_submenu_adds_and_drops_a_panel(cx: &mut TestAppContext) {
    use super::menu::{Open, Spot, mark};
    let (menu, seen, _) = watched_menu();
    let cx = cx.add_empty_window();
    let state = cx.update(|_, cx| cx.new(|_| Open::default()));
    cx.update(|window, cx| {
        Open::show(&state, &menu, Spot::Under, false, cx);
        let root = panel_box(10.0, 20.0);
        state.update(cx, |open, _| {
            open.levels[0].panel = root;
            open.notify_panels();
        });
        mark(&state, &menu, 0, 0, true, cx);
        let sub = panel_box(140.0, 20.0);
        state.update(cx, |open, _| {
            open.levels[1].panel = sub;
            open.notify_panels();
        });
        assert_eq!(
            *seen.borrow(),
            [vec![root], vec![root, sub]],
            "the submenu panel was not reported"
        );
        state.update(cx, |open, _| {
            open.levels.pop();
            open.notify_panels();
        });
        assert_eq!(
            *seen.borrow(),
            [vec![root], vec![root, sub], vec![root]],
            "the submenu panel stayed reported"
        );
        Open::close(&state, window, cx);
    });
}

#[test]
fn filtering_keeps_the_panel_observer_and_the_dismiss_handle() {
    let (menu, _, dismiss) = watched_menu();
    for query in ["", "Ch", "zzzz"] {
        let filtered = menu.filtered(query);
        assert!(Rc::ptr_eq(
            menu.on_panels_change.as_ref().unwrap(),
            filtered.on_panels_change.as_ref().unwrap(),
        ));
        assert!(!dismiss.is_requested(), "a fresh handle asks for nothing");
    }
    // The rebuilt menu keeps the same dismiss state, so a request still reaches it.
    dismiss.request();
    assert!(menu.filtered("Ch").dismiss.is_requested());
}

/// A host whose menu reports the boxes of its panels.
struct Panels {
    menu: Menu,
}

impl Render for Panels {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(DropdownMenu::new("panels", "Edit", self.menu.clone()))
    }
}

#[gpui::test]
fn the_measured_panel_reaches_the_observer_and_a_dismiss_closes_it(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let seen: Seen = Rc::new(RefCell::new(Vec::new()));
    let observed = seen.clone();
    let menu = Menu::new()
        .on_panels_change(move |panels| observed.borrow_mut().push(panels.to_vec()))
        .item(MenuItem::new("Cut"));
    let dismiss = menu.dismiss_handle();
    let (view, cx) = cx.add_window_view(|_, _| Panels { menu: menu.clone() });
    settle(cx);

    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    let panel = cx.debug_bounds("menu-panel-0").expect("the open menu");
    let reported = seen.borrow()[0].clone();
    assert_eq!(reported.len(), 1, "the observer did not get the one panel");
    assert!(
        !reported[0].is_empty() && panel.contains(&reported[0].center()),
        "the measured panel {:?} is not inside the drawn panel {panel:?}",
        reported[0]
    );

    // A dismiss asked for from outside closes the menu on its next render.
    dismiss.request();
    view.update(cx, |_, cx| cx.notify());
    settle(cx);
    assert!(
        cx.debug_bounds("menu-panel-0").is_none(),
        "the dismissed menu stayed open"
    );
    assert_eq!(
        *seen.borrow(),
        [reported.clone(), vec![]],
        "the dismissed menu did not report its empty panels"
    );
}
