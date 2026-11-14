use super::*;

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
    use super::super::menu::{Open, Spot, mark};
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
    use super::super::menu::{Open, Spot};
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
    use super::super::menu::{Open, Spot};
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
    use super::super::menu::{Open, Spot};
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
    use super::super::menu::{Open, Spot, mark};
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
