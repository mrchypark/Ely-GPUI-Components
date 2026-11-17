use super::*;

/// A watch panel; it keeps what its remove reports.
struct Watches {
    field: Entity<TextInput>,
}

impl Render for Watches {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let watch = Watch {
            expression: "lift".into(),
            value: Ok(("42".into(), ValueKind::Number)),
        };
        div()
            .w(px(400.0))
            .child(WatchPanel::new("watches", [watch], &self.field))
    }
}

#[gpui::test]
fn a_tap_on_a_watch_row_shows_its_remove_button(cx: &mut TestAppContext) {
    touch_theme(cx);
    let (_, cx) = cx.add_window_view(|window, cx| Watches {
        field: cx.new(|cx| TextInput::new(window, cx)),
    });
    settle(cx);
    assert!(
        cx.debug_bounds("watch-remove-revealed").is_none(),
        "the remove button hides until the row is pressed"
    );
    let row = cx.debug_bounds("watch-row").expect("the row draws");
    let at = row.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("watch-remove-revealed").is_some(),
        "the press opens the remove button right away"
    );
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("watch-remove-revealed").is_some(),
        "the tap reveals the remove button"
    );
}

/// A watch panel of many expressions; it keeps what its remove reports.
struct WatchRows {
    field: Entity<TextInput>,
    watches: Vec<Watch>,
    removed: Rc<RefCell<Vec<usize>>>,
}

impl Render for WatchRows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let removed = self.removed.clone();
        div().w(px(400.0)).child(
            WatchPanel::new("watches", self.watches.clone(), &self.field)
                .on_remove(move |ix, _, _| removed.borrow_mut().push(ix)),
        )
    }
}

fn watched(expression: &str) -> Watch {
    Watch {
        expression: expression.into(),
        value: Ok(("42".into(), ValueKind::Number)),
    }
}

/// Press a row by its own name, the way a finger would.
fn press(cx: &mut VisualTestContext, row: &'static str) {
    let at = cx.debug_bounds(row).expect("the row draws").center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn a_press_on_one_watch_row_leaves_the_other_alone(cx: &mut TestAppContext) {
    touch_theme(cx);
    let removed = Rc::new(RefCell::new(Vec::new()));
    let (_, cx) = cx.add_window_view(|window, cx| WatchRows {
        field: cx.new(|cx| TextInput::new(window, cx)),
        watches: vec![watched("lift"), watched("push")],
        removed: removed.clone(),
    });
    settle(cx);
    press(cx, "watch-row-0");
    assert!(
        cx.debug_bounds("watch-remove-revealed-0").is_some(),
        "the pressed row opens its remove button"
    );
    assert!(
        cx.debug_bounds("watch-remove-revealed-1").is_none(),
        "the other row stays closed"
    );
    press(cx, "watch-row-1");
    assert!(
        cx.debug_bounds("watch-remove-revealed-0").is_some(),
        "the first row stays open"
    );
    assert!(
        cx.debug_bounds("watch-remove-revealed-1").is_some(),
        "the second row opens too"
    );
    press(cx, "watch-row-0");
    assert!(
        cx.debug_bounds("watch-remove-revealed-0").is_none(),
        "the pressed row closes again"
    );
    assert!(
        cx.debug_bounds("watch-remove-revealed-1").is_some(),
        "the other row is untouched"
    );
}

#[gpui::test]
fn a_press_on_a_watch_row_keeps_quiet_on_a_desktop(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|window, cx| WatchRows {
        field: cx.new(|cx| TextInput::new(window, cx)),
        watches: vec![watched("lift"), watched("push")],
        removed: Rc::new(RefCell::new(Vec::new())),
    });
    settle(cx);
    press(cx, "watch-row-0");
    assert!(
        cx.debug_bounds("watch-remove-revealed-0").is_none(),
        "a pointer keeps its hover, so a press reveals nothing"
    );
}

#[gpui::test]
fn a_remove_on_a_watch_row_names_its_place_in_the_list(cx: &mut TestAppContext) {
    touch_theme(cx);
    let removed = Rc::new(RefCell::new(Vec::new()));
    let (owner, cx) = cx.add_window_view(|window, cx| WatchRows {
        field: cx.new(|cx| TextInput::new(window, cx)),
        watches: vec![watched("lift"), watched("push")],
        removed: removed.clone(),
    });
    settle(cx);
    press(cx, "watch-row-1");
    // 목록에서 앞의 watch가 빠지면, 남은 watch의 자리는 앞당겨진다.
    owner.update(cx, |owner, cx| {
        owner.watches = vec![watched("push")];
        cx.notify();
    });
    settle(cx);
    let button = cx
        .debug_bounds("watch-remove-revealed-0")
        .expect("the row stays open");
    cx.simulate_click(button.center(), Modifiers::none());
    settle(cx);
    assert_eq!(*removed.borrow(), [0], "the remove names the current place");
    assert!(
        cx.debug_bounds("watch-remove-revealed-0").is_none(),
        "the remove closes that row's reveal"
    );
}

/// A breakpoint list; each row keeps what it shows.
struct Listed {
    point: Breakpoint,
}

impl Render for Listed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(BreakpointList::new("points", [self.point.clone()]))
    }
}

#[gpui::test]
fn a_tap_on_a_breakpoint_row_shows_its_remove_button(cx: &mut TestAppContext) {
    touch_theme(cx);
    let point = Breakpoint {
        path: "src/lift.rs".into(),
        line: 3,
        enabled: true,
        condition: None,
        hits: 0,
    };
    let (_, cx) = cx.add_window_view(move |_, _| Listed { point });
    settle(cx);
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed").is_none(),
        "the remove button hides until the row is pressed"
    );
    let row = cx.debug_bounds("breakpoint-row").expect("the row draws");
    let at = row.center();
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed").is_some(),
        "the press opens the remove button right away"
    );
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed").is_some(),
        "the tap reveals the remove button"
    );
}

/// A breakpoint list of many points; it keeps what its remove reports.
struct PointRows {
    points: Vec<Breakpoint>,
    removed: Rc<RefCell<Vec<usize>>>,
}

impl Render for PointRows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let removed = self.removed.clone();
        div().w(px(400.0)).child(
            BreakpointList::new("points", self.points.clone())
                .on_remove(move |ix, _, _| removed.borrow_mut().push(ix)),
        )
    }
}

fn a_point(path: &str, line: usize) -> Breakpoint {
    Breakpoint {
        path: path.into(),
        line,
        enabled: true,
        condition: None,
        hits: 0,
    }
}

#[gpui::test]
fn a_press_on_one_breakpoint_row_leaves_the_other_alone(cx: &mut TestAppContext) {
    touch_theme(cx);
    let (_, cx) = cx.add_window_view(|_, _| PointRows {
        points: vec![a_point("src/lift.rs", 3), a_point("src/push.rs", 7)],
        removed: Rc::new(RefCell::new(Vec::new())),
    });
    settle(cx);
    press(cx, "breakpoint-row-0");
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-0").is_some(),
        "the pressed row opens its remove button"
    );
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-1").is_none(),
        "the other row stays closed"
    );
    press(cx, "breakpoint-row-1");
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-0").is_some(),
        "the first row stays open"
    );
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-1").is_some(),
        "the second row opens too"
    );
    press(cx, "breakpoint-row-0");
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-0").is_none(),
        "the pressed row closes again"
    );
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-1").is_some(),
        "the other row is untouched"
    );
}

#[gpui::test]
fn a_press_on_a_breakpoint_row_keeps_quiet_on_a_desktop(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| PointRows {
        points: vec![a_point("src/lift.rs", 3), a_point("src/push.rs", 7)],
        removed: Rc::new(RefCell::new(Vec::new())),
    });
    settle(cx);
    press(cx, "breakpoint-row-0");
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-0").is_none(),
        "a pointer keeps its hover, so a press reveals nothing"
    );
}

#[gpui::test]
fn a_remove_on_a_breakpoint_row_names_its_place_in_the_list(cx: &mut TestAppContext) {
    touch_theme(cx);
    let removed = Rc::new(RefCell::new(Vec::new()));
    let (owner, cx) = cx.add_window_view(|_, _| PointRows {
        points: vec![a_point("src/lift.rs", 3), a_point("src/push.rs", 7)],
        removed: removed.clone(),
    });
    settle(cx);
    press(cx, "breakpoint-row-1");
    // 목록에서 앞의 breakpoint가 빠지면, 남은 breakpoint의 자리는 앞당겨진다.
    owner.update(cx, |owner, cx| {
        owner.points = vec![a_point("src/push.rs", 7)];
        cx.notify();
    });
    settle(cx);
    let button = cx
        .debug_bounds("breakpoint-remove-revealed-0")
        .expect("the row stays open");
    cx.simulate_click(button.center(), Modifiers::none());
    settle(cx);
    assert_eq!(*removed.borrow(), [0], "the remove names the current place");
    assert!(
        cx.debug_bounds("breakpoint-remove-revealed-0").is_none(),
        "the remove closes that row's reveal"
    );
}
