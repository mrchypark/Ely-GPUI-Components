use super::*;

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
                ListItem::new("a", "A").trailing(super::super::row_action(button)),
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
            .on_scroll(move |_, _, _| wheels.set(wheels.get() + 1))
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
