use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement,
    Pixels, Point, Render, ScrollDelta, ScrollWheelEvent, SharedString, StatefulInteractiveElement,
    Styled, TestAppContext, TouchPhase, VisualTestContext, Window, div, point, px,
};

use super::{
    DirEntry, DirectoryListing, InfiniteList, ListItem, SelectableList, SortableList, SwipeAction,
    SwipeableListItem, VirtualList,
};
use crate::{
    primitives::{IconName, Measure},
    theme::Theme,
};

mod keys;

fn setup(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
}

/// Frames 2ms apart, past reduced motion's 1ms glides.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

const NAMES: [&str; 4] = ["a", "b", "c", "d"];

/// A multiple-choice list of four rows, one maybe disabled, maybe reversed, that keeps what it reports and what was opened.
struct Picks(
    Vec<SharedString>,
    Vec<SharedString>,
    Option<&'static str>,
    bool,
);

impl Render for Picks {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (view, opened) = (cx.entity(), cx.entity());
        let names: Vec<&&str> = if self.3 {
            NAMES.iter().rev().collect()
        } else {
            NAMES.iter().collect()
        };
        names
            .into_iter()
            .fold(SelectableList::new("picks").multiple(), |list, name| {
                list.row(
                    *name,
                    ListItem::new(*name, *name).disabled(self.2 == Some(*name)),
                )
            })
            .selected(self.0.clone())
            .on_change(move |keys, _, cx| view.update(cx, |picks, _| picks.0 = keys.to_vec()))
            .on_activate(move |key, _, cx| opened.update(cx, |picks, _| picks.1.push(key.clone())))
            .w(px(240.0))
            .h(px(200.0))
    }
}

fn picks(view: &Entity<Picks>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |picks, _| {
        picks.0.iter().map(|key| key.to_string()).collect()
    })
}

/// Row `ix`'s left padding: rows are 32 tall with 2 between.
fn row(ix: usize) -> Point<Pixels> {
    point(px(6.0), px(16.0 + 34.0 * ix as f32))
}

#[gpui::test]
fn presses_pick_one_toggle_with_cmd_and_range_with_shift(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), None, false));
    settle(cx);
    cx.simulate_click(row(0), Modifiers::none());
    settle(cx);
    assert_eq!(picks(&view, cx), ["a"]);
    cx.simulate_click(
        row(2),
        Modifiers {
            shift: true,
            ..Modifiers::none()
        },
    );
    settle(cx);
    assert_eq!(picks(&view, cx), ["a", "b", "c"]);
    cx.simulate_click(
        row(1),
        Modifiers {
            platform: true,
            ..Modifiers::none()
        },
    );
    settle(cx);
    assert_eq!(picks(&view, cx), ["a", "c"]);
    cx.simulate_keystrokes("down shift-down");
    settle(cx);
    assert_eq!(
        picks(&view, cx),
        ["c", "d"],
        "down picks one, shift-down extends from it"
    );
    cx.simulate_keystrokes("cmd-a");
    settle(cx);
    assert_eq!(picks(&view, cx), NAMES);
}

#[gpui::test]
fn a_finger_picks_a_row_and_the_row_keeps_a_finger_sized_target(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| Theme::update(cx, |theme| theme.touch_mode = true));
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), None, false));
    settle(cx);

    // A phone has no hover, so the row must be a finger-sized target of its own.
    let bounds = cx.debug_bounds("list-item-b").expect("the second row");
    assert!(
        bounds.size.height >= px(48.),
        "a list row keeps a finger-sized target, got {:?}",
        bounds.size.height
    );

    // The press shows the row as pressed, and the release picks it.
    cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        picks(&view, cx),
        Vec::<String>::new(),
        "the press has not lifted"
    );
    cx.simulate_mouse_up(bounds.center(), MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(picks(&view, cx), ["b"], "the tap picked the row");
}

#[gpui::test]
fn the_cursor_starts_on_the_first_selected_row(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(vec!["c".into()], Vec::new(), None, false));
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("down");
    settle(cx);
    assert_eq!(picks(&view, cx), ["d"]);
}

#[gpui::test]
fn a_selection_from_the_owner_moves_the_cursor_and_an_echo_does_not(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(vec!["a".into()], Vec::new(), None, false));
    settle(cx);
    view.update(cx, |picks, _| picks.0 = vec!["b".into()]);
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("shift-down shift-down");
    settle(cx);
    assert_eq!(
        picks(&view, cx),
        ["b", "c", "d"],
        "the range starts where the owner put it"
    );
    cx.simulate_keystrokes("up");
    settle(cx);
    assert_eq!(
        picks(&view, cx),
        ["c"],
        "its own range left the cursor at its end"
    );
}

/// A row with two actions, 144 across; it reports where the row sits and which action ran.
struct Swiped {
    left: Rc<Cell<Pixels>>,
    ran: Rc<RefCell<Vec<&'static str>>>,
}

impl Render for Swiped {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (left, archive, delete) = (self.left.clone(), self.ran.clone(), self.ran.clone());
        div().w(px(320.0)).child(
            SwipeableListItem::new(
                "swiped",
                Measure::new("row", move |bounds, _, _| left.set(bounds.origin.x))
                    .h(px(40.0))
                    .child("Invoice 2419"),
            )
            .action(SwipeAction::new(
                "Archive",
                IconName::Archive,
                move |_, _| archive.borrow_mut().push("archive"),
            ))
            .action(SwipeAction::new("Delete", IconName::Trash2, move |_, _| {
                delete.borrow_mut().push("delete")
            })),
        )
    }
}

fn scroll(cx: &mut VisualTestContext, phase: TouchPhase, x: f32, y: f32) {
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(160.0), px(20.0)),
        delta: ScrollDelta::Pixels(point(px(x), px(y))),
        modifiers: Modifiers::none(),
        touch_phase: phase,
    });
    settle(cx);
}

#[gpui::test]
fn a_sideways_swipe_opens_the_row_and_an_action_shuts_it(cx: &mut TestAppContext) {
    setup(cx);
    let (left, ran) = (
        Rc::new(Cell::new(Pixels::ZERO)),
        Rc::new(RefCell::new(Vec::new())),
    );
    let (seen, done) = (left.clone(), ran.clone());
    let (_, cx) = cx.add_window_view(|_, _| Swiped {
        left: seen,
        ran: done,
    });
    settle(cx);
    scroll(cx, TouchPhase::Started, 0.0, 0.0);
    scroll(cx, TouchPhase::Moved, -2.0, -30.0);
    scroll(cx, TouchPhase::Moved, -100.0, 0.0);
    scroll(cx, TouchPhase::Ended, 0.0, 0.0);
    assert_eq!(
        left.get(),
        Pixels::ZERO,
        "a gesture that starts upright is the list's"
    );
    scroll(cx, TouchPhase::Started, 0.0, 0.0);
    scroll(cx, TouchPhase::Moved, -50.0, 4.0);
    assert_eq!(left.get(), px(-50.0), "the row follows the fingers");
    scroll(cx, TouchPhase::Moved, -40.0, 0.0);
    scroll(cx, TouchPhase::Ended, 0.0, 0.0);
    assert_eq!(left.get(), px(-144.0), "past half its reach, it snaps open");
    cx.simulate_click(point(px(300.0), px(20.0)), Modifiers::none());
    settle(cx);
    assert_eq!(*ran.borrow(), ["delete"]);
    assert_eq!(left.get(), Pixels::ZERO, "an action shuts the row");
}

/// Rows in a short scrolling box that count how often they asked for more.
struct Paging {
    rows: usize,
    asked: usize,
}

impl Render for Paging {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().h(px(400.0)).child(
            InfiniteList::new("paging")
                .children((0..self.rows).map(|ix| div().h(px(20.0)).child(format!("Row {ix}"))))
                .on_more(move |_, cx| view.update(cx, |paging, _| paging.asked += 1)),
        )
    }
}

#[gpui::test]
fn the_end_in_view_asks_for_more_once_a_page(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Paging { rows: 3, asked: 0 });
    settle(cx);
    assert_eq!(view.read_with(cx, |paging, _| paging.asked), 1);
    settle(cx);
    assert_eq!(
        view.read_with(cx, |paging, _| paging.asked),
        1,
        "staying in view asks no more"
    );
    view.update(cx, |paging, cx| {
        paging.rows = 6;
        cx.notify();
    });
    settle(cx);
    assert_eq!(
        view.read_with(cx, |paging, _| paging.asked),
        2,
        "a new page asks again"
    );
}

/// Three sortable rows; the moves the owner hears.
struct Sorting(Rc<RefCell<Vec<(usize, usize)>>>);

impl Render for Sorting {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let moves = self.0.clone();
        NAMES[..3]
            .iter()
            .fold(SortableList::new("sorting"), |list, name| {
                list.row(*name, ListItem::new(*name, *name))
            })
            .on_reorder(move |from, to, _, _| moves.borrow_mut().push((from, to)))
    }
}

#[gpui::test]
fn alt_with_arrows_moves_the_current_row(cx: &mut TestAppContext) {
    setup(cx);
    let moves = Rc::new(RefCell::new(Vec::new()));
    let heard = moves.clone();
    let (_, cx) = cx.add_window_view(|_, _| Sorting(heard));
    settle(cx);
    cx.simulate_click(row(0), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("alt-down down alt-up alt-up");
    settle(cx);
    assert_eq!(*moves.borrow(), [(0, 1), (2, 1), (1, 0)]);
}

/// A thousand rows, 20 tall, in a box 200 tall; it keeps the rows it was asked to build.
struct Long(Rc<RefCell<Vec<usize>>>);

impl Render for Long {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let built = self.0.clone();
        VirtualList::new("long", 1000, move |ix, _, _| {
            built.borrow_mut().push(ix);
            div()
                .h(px(20.0))
                .child(format!("Row {ix}"))
                .into_any_element()
        })
        .h(px(200.0))
    }
}

#[gpui::test]
fn a_long_list_builds_only_what_is_near_the_view(cx: &mut TestAppContext) {
    setup(cx);
    let built = Rc::new(RefCell::new(Vec::new()));
    let seen = built.clone();
    let (_, cx) = cx.add_window_view(|_, _| Long(seen));
    settle(cx);
    let most = built.borrow().iter().copied().max().expect("some rows");
    assert!((9..60).contains(&most), "built up to row {most} of 1000");
}

/// Enter, pressed and let go: a row opens on the release.
fn enter(cx: &mut VisualTestContext) {
    cx.simulate_keystrokes("enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("a key"),
    });
}

#[gpui::test]
fn enter_or_a_double_press_opens_a_row(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), None, false));
    settle(cx);
    cx.simulate_click(row(1), Modifiers::none());
    settle(cx);
    enter(cx);
    settle(cx);
    for count in [1, 2] {
        cx.simulate_event(gpui::MouseDownEvent {
            button: gpui::MouseButton::Left,
            position: row(3),
            modifiers: Modifiers::none(),
            click_count: count,
            first_mouse: false,
        });
        cx.simulate_event(gpui::MouseUpEvent {
            button: gpui::MouseButton::Left,
            position: row(3),
            modifiers: Modifiers::none(),
            click_count: count,
        });
    }
    settle(cx);
    let opened: Vec<String> = view.read_with(cx, |picks, _| {
        picks.1.iter().map(|key| key.to_string()).collect()
    });
    assert_eq!(opened, ["b", "d"]);
}

#[gpui::test]
fn keys_pass_over_a_disabled_row_and_never_pick_or_open_it(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), Some("b"), false));
    settle(cx);
    cx.simulate_click(row(0), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("down");
    settle(cx);
    assert_eq!(picks(&view, cx), ["c"], "down steps over the disabled row");
    cx.simulate_keystrokes("up");
    enter(cx);
    cx.simulate_keystrokes("cmd-a");
    settle(cx);
    assert_eq!(
        picks(&view, cx),
        ["a", "c", "d"],
        "all leaves the disabled row out"
    );
    let opened: Vec<String> = view.read_with(cx, |picks, _| {
        picks.1.iter().map(|key| key.to_string()).collect()
    });
    assert_eq!(
        opened,
        ["a"],
        "up landed on the first row, not the disabled one"
    );
}

#[gpui::test]
fn the_cursor_stays_on_its_row_when_the_rows_reorder(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), None, false));
    settle(cx);
    cx.simulate_click(row(0), Modifiers::none());
    settle(cx);
    view.update(cx, |picks, cx| {
        picks.3 = true;
        cx.notify();
    });
    settle(cx);
    enter(cx);
    cx.simulate_keystrokes("up");
    settle(cx);
    let opened: Vec<String> = view.read_with(cx, |picks, _| {
        picks.1.iter().map(|key| key.to_string()).collect()
    });
    assert_eq!(opened, ["a"], "enter opens the row the cursor was on");
    assert_eq!(picks(&view, cx), ["b"], "up moves from where that row went");
}

/// A listing 280 wide.
struct Narrow;

impl Render for Narrow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let then = jiff::Timestamp::from_second(1_700_000_000).expect("a time");
        let entries = [
            DirEntry::folder("Design", then),
            DirEntry::file("Brief.md", 4_800, then),
        ];
        div()
            .w(px(280.0))
            .child(DirectoryListing::new("listing", ["Home"], entries))
    }
}

#[gpui::test]
fn a_narrow_listing_keeps_room_for_the_name(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Narrow);
    settle(cx);
    let name = cx
        .debug_bounds("listing-name")
        .expect("the name heads its column");
    let least = cx.update(|window, cx| {
        use crate::theme::ActiveTheme;
        cx.theme().label_width().to_pixels(window.rem_size())
    });
    assert!(name.size.width >= least, "{name:?} under {least:?}");
}

/// A list whose row holds a button kept by `row_action`, which keeps what the list reports and what the button heard.
struct Held(Vec<SharedString>, Rc<Cell<usize>>);

impl Render for Held {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (view, pressed) = (cx.entity(), self.1.clone());
        let button = div()
            .id("held-button")
            .debug_selector(|| "held-button".into())
            .size(px(24.0))
            .on_click(move |_, _, _| pressed.set(pressed.get() + 1));
        SelectableList::new("held")
            .row(
                "a",
                ListItem::new("a", "A").trailing(super::row_action(button)),
            )
            .on_change(move |keys, _, cx| view.update(cx, |held, _| held.0 = keys.to_vec()))
            .w(px(240.0))
    }
}

#[gpui::test]
fn a_press_on_a_rows_button_leaves_the_row_alone(cx: &mut TestAppContext) {
    setup(cx);
    let pressed = Rc::new(Cell::new(0));
    let heard = pressed.clone();
    let (view, cx) = cx.add_window_view(move |_, _| Held(Vec::new(), heard));
    settle(cx);
    let button = cx.debug_bounds("held-button").expect("the row's button");
    cx.simulate_click(button.center(), Modifiers::none());
    settle(cx);
    assert_eq!(pressed.get(), 1, "the button heard its press");
    assert!(
        view.read_with(cx, |held, _| held.0.is_empty()),
        "the row stayed unselected"
    );
}

struct SwipeContainment {
    left: Rc<Cell<Pixels>>,
    actions: Rc<Cell<usize>>,
    parent_wheels: Rc<Cell<usize>>,
    parent_scroll: gpui::ScrollHandle,
}
impl Render for SwipeContainment {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (left, actions, wheels) = (
            self.left.clone(),
            self.actions.clone(),
            self.parent_wheels.clone(),
        );
        div()
            .id("swipe-parent")
            .w(px(320.))
            .h(px(160.))
            .overflow_y_scroll()
            .track_scroll(&self.parent_scroll)
            .on_scroll_wheel(move |_, _, _| wheels.set(wheels.get() + 1))
            .child(
                SwipeableListItem::new(
                    "contained-swipe",
                    Measure::new("contained-row", move |bounds, _, _| {
                        left.set(bounds.origin.x)
                    })
                    .h(px(40.))
                    .child("Invoice"),
                )
                .action(SwipeAction::new(
                    "Delete",
                    IconName::Trash2,
                    move |_, _| actions.set(actions.get() + 1),
                )),
            )
            .child(div().h(px(1000.)))
    }
}

#[gpui::test]
fn swipe_first_delta_cancel_and_terminals_preserve_parent(cx: &mut TestAppContext) {
    setup(cx);
    let left = Rc::new(Cell::new(Pixels::ZERO));
    let actions = Rc::new(Cell::new(0));
    let wheels = Rc::new(Cell::new(0));
    let parent = gpui::ScrollHandle::new();
    let (_, cx) = cx.add_window_view(|_, _| SwipeContainment {
        left: left.clone(),
        actions: actions.clone(),
        parent_wheels: wheels.clone(),
        parent_scroll: parent.clone(),
    });
    settle(cx);
    let initial = parent.offset();
    scroll(cx, TouchPhase::Started, -40., 2.);
    assert_eq!(left.get(), px(-40.), "first nonzero delta is consumed");
    scroll(cx, TouchPhase::Moved, 0., 0.);
    scroll(cx, TouchPhase::Cancelled, 0., 0.);
    assert_eq!(left.get(), px(0.), "cancel restores closed start");
    assert_eq!(wheels.get(), 0);
    assert_eq!(parent.offset(), initial);
    assert_eq!(actions.get(), 0);
    scroll(cx, TouchPhase::Started, -60., 1.);
    scroll(cx, TouchPhase::Ended, 0., 0.);
    let opened = left.get();
    assert!(opened < px(-60.));
    scroll(cx, TouchPhase::Started, 25., 0.);
    assert!(left.get() > opened);
    scroll(cx, TouchPhase::Cancelled, 0., 0.);
    assert_eq!(
        left.get(),
        opened,
        "cancel restores the open start, not zero"
    );
    assert_eq!(wheels.get(), 0, "both end and cancel remain contained");
    assert_eq!(parent.offset(), initial);
    assert_eq!(actions.get(), 0);
    scroll(cx, TouchPhase::Started, 0., -30.);
    scroll(cx, TouchPhase::Ended, 0., 0.);
    assert!(
        wheels.get() > 0,
        "a new vertical gesture belongs to the parent"
    );
    assert!(parent.offset().y < initial.y);
    assert_eq!(actions.get(), 0);
}

struct TouchSorting {
    moves: Vec<(usize, usize)>,
    selections: usize,
}
impl Render for TouchSorting {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let reorder = cx.entity();
        (0..3)
            .fold(SortableList::new("touch-sorting"), |list, ix| {
                let selected = cx.entity();
                list.row(
                    format!("row-{ix}"),
                    ListItem::new(format!("row-{ix}"), format!("Row {ix}")).on_click(
                        move |_, _, cx| selected.update(cx, |view, _| view.selections += 1),
                    ),
                )
            })
            .on_reorder(move |from, to, _, cx| {
                reorder.update(cx, |view, _| view.moves.push((from, to)))
            })
    }
}

#[gpui::test]
fn touch_sort_controls_move_once_and_disable_boundaries(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(|cx| Theme::update(cx, |theme| theme.touch_mode = true));
    let (view, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(8.));
        TouchSorting {
            moves: Vec::new(),
            selections: 0,
        }
    });
    settle(cx);
    for selector in [
        "sortable-0-up",
        "sortable-2-down",
        "sortable-0-down",
        "sortable-2-up",
    ] {
        let bounds = cx.debug_bounds(selector).expect("visible move control");
        assert!(bounds.size.width >= px(48.) && bounds.size.height >= px(48.));
        cx.simulate_click(bounds.center(), Modifiers::none());
        settle(cx);
        assert!(!cx.update(|_, cx| cx.has_active_drag()));
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.moves.clone()),
        vec![(0, 1), (2, 1)]
    );
    assert_eq!(view.read_with(cx, |view, _| view.selections), 0);
    // A move-control press must not arm the enclosing drag recognizer.
    let bounds = cx.debug_bounds("sortable-1-up").unwrap();
    cx.simulate_mouse_down(bounds.center(), gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        bounds.center() + point(px(0.), px(25.)),
        Some(gpui::MouseButton::Left),
        Modifiers::none(),
    );
    assert!(!cx.update(|_, cx| cx.has_active_drag()));
    cx.simulate_mouse_up(
        bounds.center() + point(px(0.), px(25.)),
        gpui::MouseButton::Left,
        Modifiers::none(),
    );
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    assert!(cx.debug_bounds("sortable-0-up").is_none());
}
