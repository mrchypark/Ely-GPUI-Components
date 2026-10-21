use crate::{
    tables::{DataGrid, Spreadsheet},
    theme::Theme,
};
use gpui::{
    Context, Entity, IntoElement, Modifiers, MouseButton, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, point, px,
};
use std::{collections::HashMap, rc::Rc};

mod reveal;

struct Grid {
    width: f32,
    collapse_on_commit: bool,
    height: f32,
    frozen: (usize, usize),
    cells: HashMap<(usize, usize), SharedString>,
    batches: Vec<Vec<(usize, usize, SharedString)>>,
    spreadsheet: bool,
    writable: bool,
    size: (usize, usize),
    merged: bool,
}
impl Render for Grid {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let change =
            move |edits: &[(usize, usize, SharedString)], _: &mut Window, cx: &mut gpui::App| {
                owner.update(cx, |grid, cx| {
                    grid.batches.push(edits.to_vec());
                    for (r, c, text) in edits {
                        grid.cells.insert((*r, *c), text.clone());
                    }
                    if grid.collapse_on_commit {
                        grid.width = 80.;
                        grid.height = 80.;
                    }
                    cx.notify();
                });
            };
        if self.spreadsheet {
            let grid = Spreadsheet::new("touch-grid", self.size.0, self.size.1)
                .freeze(self.frozen.0, self.frozen.1)
                .cells(Rc::new(self.cells.clone()));
            let grid = if self.merged {
                grid.merge((0, 0), (0, 1)).freeze(1, 1)
            } else {
                grid
            };
            let grid = if self.writable {
                grid.on_change(change)
            } else {
                grid
            };
            grid.w(px(self.width)).h(px(self.height)).into_any_element()
        } else {
            let rows: Vec<Vec<SharedString>> = (0..self.size.0)
                .map(|r| {
                    (0..self.size.1)
                        .map(|c| self.cells.get(&(r, c)).cloned().unwrap_or_default())
                        .collect()
                })
                .collect();
            let grid = DataGrid::new("touch-grid", (0..self.size.1).map(|c| c.to_string()), rows);
            let grid = if self.writable {
                grid.on_change(change)
            } else {
                grid
            };
            grid.w(px(self.width)).h(px(self.height)).into_any_element()
        }
    }
}
fn stage(cx: &mut TestAppContext, spreadsheet: bool) -> (Entity<Grid>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (owner, cx) = cx.add_window_view(|window, _| {
        window.set_rem_size(px(16.));
        Grid {
            width: 390.,
            collapse_on_commit: false,
            height: 480.,
            frozen: (0, 0),
            cells: [
                ((0, 0), "2".into()),
                ((1, 0), "4".into()),
                ((4, 2), "keep".into()),
            ]
            .into(),
            batches: Vec::new(),
            spreadsheet,
            writable: true,
            size: (6, 3),
            merged: false,
        }
    });
    super::settle(cx);
    (owner, cx)
}
fn tap(at: gpui::Point<gpui::Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    super::settle(cx);
}
fn action(name: &str, cx: &mut VisualTestContext) {
    let selector = match name {
        "edit" => "grid-touch-edit",
        "done" => "grid-touch-done",
        "cancel" => "grid-touch-cancel",
        "extend" => "grid-touch-extend",
        "clear" => "grid-touch-clear",
        "fill" => "grid-touch-fill",
        _ => panic!("unknown action"),
    };
    let bounds = cx.debug_bounds(selector).expect("visible action");
    assert!(bounds.size.height >= px(48.));
    assert!(bounds.origin.x >= px(0.) && bounds.right() <= px(390.));
    tap(bounds.center(), cx);
}
fn cell(row: usize, col: usize, sheet: bool, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds("grid-surface").unwrap();
    let (wide, tall, label) = cx.update(|window, cx| {
        use crate::theme::{ActiveTheme, ControlSize, Density};
        let theme = cx.theme();
        let rem = window.rem_size();
        (
            theme.grid_column().to_pixels(rem),
            theme.touch_minimum(theme.table_row(Density::Compact).to_pixels(rem)),
            if sheet {
                theme.control_height(ControlSize::Lg).to_pixels(rem) * 1.25
            } else {
                px(0.)
            },
        )
    });
    tap(
        bounds.origin
            + point(
                label + wide * col as f32 + px(10.),
                tall * (row + 1) as f32 + px(10.),
            ),
        cx,
    );
}
#[gpui::test]
fn tap_edit_done_cancel_keeps_exact_owner_values(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, false);
    cell(0, 0, false, cx);
    action("edit", cx);
    cx.simulate_input("changed");
    action("done", cx);
    cx.update(|_, cx| {
        assert_eq!(owner.read(cx).batches, vec![vec![(0, 0, "changed".into())]]);
        assert_eq!(owner.read(cx).cells[&(0, 0)], "changed");
    });
    cell(0, 0, false, cx);
    action("edit", cx);
    cx.simulate_input("discard");
    action("cancel", cx);
    cx.update(|_, cx| {
        assert_eq!(owner.read(cx).batches.len(), 1);
        assert_eq!(owner.read(cx).cells[&(0, 0)], "changed");
    });
}
#[gpui::test]
fn tap_range_clear_reports_rectangle_once(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, false);
    cell(0, 0, false, cx);
    action("extend", cx);
    cell(2, 2, false, cx);
    action("clear", cx);
    cx.update(|_, cx| {
        let expected: Vec<_> = (0..3)
            .flat_map(|r| (0..3).map(move |c| (r, c, SharedString::default())))
            .collect();
        assert_eq!(owner.read(cx).batches, vec![expected]);
        assert_eq!(owner.read(cx).cells[&(4, 2)], "keep");
    });
}
#[gpui::test]
fn tap_fill_uses_source_once_and_cancel_never_replays(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, true);
    cell(0, 0, true, cx);
    action("extend", cx);
    cell(1, 0, true, cx);
    action("fill", cx);
    cell(4, 0, true, cx);
    cx.update(|_, cx| {
        assert_eq!(
            owner.read(cx).batches,
            vec![vec![
                (2, 0, "6".into()),
                (3, 0, "8".into()),
                (4, 0, "10".into())
            ]]
        );
        assert_eq!(owner.read(cx).cells[&(0, 0)], "2");
        assert_eq!(owner.read(cx).cells[&(1, 0)], "4");
    });
    action("fill", cx);
    action("cancel", cx);
    cell(3, 1, true, cx);
    cx.update(|_, cx| assert_eq!(owner.read(cx).batches.len(), 1));
}
#[gpui::test]
fn read_only_and_empty_grids_cannot_mutate(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, false);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.writable = false;
            cx.notify();
        })
    });
    super::settle(cx);
    action("edit", cx);
    action("clear", cx);
    assert!(cx.debug_bounds("grid-touch-done").is_none());
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.writable = true;
            grid.size = (0, 0);
            cx.notify();
        })
    });
    super::settle(cx);
    action("edit", cx);
    action("clear", cx);
    cx.update(|_, cx| assert!(owner.read(cx).batches.is_empty()));
}
#[gpui::test]
fn shrinking_grid_cancels_pending_fill(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, true);
    action("fill", cx);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.size = (3, 3);
            cx.notify();
        })
    });
    super::settle(cx);
    assert!(cx.debug_bounds("grid-touch-cancel").is_none());
    cell(2, 0, true, cx);
    cx.update(|_, cx| assert!(owner.read(cx).batches.is_empty()));
}

#[gpui::test]
fn tap_formula_fill_right_and_invalid_destination(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, true);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.cells.insert((0, 0), "=A2".into());
            cx.notify();
        })
    });
    super::settle(cx);
    cell(0, 0, true, cx);
    action("fill", cx);
    cell(0, 2, true, cx);
    cx.update(|_, cx| {
        assert_eq!(
            owner.read(cx).batches,
            vec![vec![(0, 1, "=B2".into()), (0, 2, "=C2".into())]]
        );
        assert_eq!(owner.read(cx).cells[&(0, 0)], "=A2");
    });
    cell(1, 1, true, cx);
    action("fill", cx);
    cell(0, 0, true, cx);
    cx.update(|_, cx| assert_eq!(owner.read(cx).batches.len(), 1));
}

#[gpui::test]
fn merged_cell_edit_restores_focus_and_shrink_cancels(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, true);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.merged = true;
            cx.notify();
        })
    });
    super::settle(cx);
    cell(0, 1, true, cx);
    let focus = cx.update(|window, cx| window.focused(cx));
    action("edit", cx);
    assert_ne!(cx.update(|window, cx| window.focused(cx)), focus);
    // A tap inside the input must not bubble into grid selection/blur commit.
    let input = cx.debug_bounds("input-root").unwrap();
    tap(input.center(), cx);
    cx.update(|_, cx| assert!(owner.read(cx).batches.is_empty()));
    // Re-entering selects the raw value, making replacement deterministic after caret tapping.
    action("cancel", cx);
    action("edit", cx);
    cx.simulate_input("merged");
    action("done", cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
    cx.update(|_, cx| assert_eq!(owner.read(cx).batches, vec![vec![(0, 0, "merged".into())]]));
    cell(0, 0, true, cx);
    action("edit", cx);
    cx.simulate_input("stale");
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.size = (1, 3);
            cx.notify();
        })
    });
    super::settle(cx);
    assert!(cx.debug_bounds("grid-touch-done").is_none());
    cx.update(|_, cx| assert_eq!(owner.read(cx).batches.len(), 1));
}

#[gpui::test]
fn frozen_rows_and_scrolled_cells_keep_hit_coordinates(cx: &mut TestAppContext) {
    let (owner, cx) = stage(cx, true);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.merged = true;
            grid.size = (30, 3);
            cx.notify();
        })
    });
    super::settle(cx);
    let surface = cx.debug_bounds("grid-surface").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: surface.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-96.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    super::settle(cx);
    // The frozen first row stays; the second visible row now addresses row three.
    cell(0, 1, true, cx);
    action("edit", cx);
    cx.simulate_input("frozen");
    action("done", cx);
    // Commit reveals row one; re-establish the scroll before testing viewport hit mapping.
    let fixed = cx.debug_bounds("grid-cell-0-0").unwrap();
    let before = cx.debug_bounds("grid-cell-3-1").unwrap();
    let surface = cx.debug_bounds("grid-surface").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: surface.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(-96.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    super::settle(cx);
    let after = cx.debug_bounds("grid-cell-3-1").unwrap();
    assert_eq!(before.top() - after.top(), px(96.));
    assert_eq!(cx.debug_bounds("grid-cell-0-0").unwrap(), fixed);
    cell(1, 1, true, cx);
    action("edit", cx);
    cx.simulate_input("scrolled");
    action("done", cx);
    cx.update(|_, cx| {
        assert_eq!(
            owner.read(cx).batches,
            vec![
                vec![(0, 0, "frozen".into())],
                vec![(3, 1, "scrolled".into())]
            ]
        )
    });
}

fn short_edit(cx: &mut TestAppContext, frozen: bool) -> (Entity<Grid>, &mut VisualTestContext) {
    let (owner, cx) = stage(cx, frozen);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.height = if frozen { 200. } else { 150. };
            grid.frozen = if frozen { (1, 1) } else { (0, 0) };
            grid.cells.insert((usize::from(frozen), 0), "2400".into());
            cx.notify();
        })
    });
    super::settle(cx);
    cell(usize::from(frozen), 0, frozen, cx);
    action("edit", cx);
    (owner, cx)
}
fn next_row_inside(row: &'static str, frozen: bool, cx: &mut VisualTestContext) {
    let selected = cx.debug_bounds(row).expect("advanced row is drawn");
    let surface = cx.debug_bounds("grid-surface").unwrap();
    assert!(
        selected.top() >= surface.top() + px(if frozen { 96. } else { 48. }),
        "selected row clears headers/frozen rows"
    );
    assert!(
        selected.bottom() <= surface.bottom(),
        "advanced row fully visible: {selected:?} within {surface:?}"
    );
    assert_eq!(selected.size.height, px(48.));
}
#[gpui::test]
fn short_viewport_done_reveals_advanced_row_once(cx: &mut TestAppContext) {
    let (owner, cx) = short_edit(cx, false);
    cx.simulate_input("2500");
    action("done", cx);
    next_row_inside("grid-cell-1-0", false, cx);
    owner.read_with(cx, |grid, _| {
        assert_eq!(grid.batches, vec![vec![(0, 0, "2500".into())]]);
        assert_eq!(grid.cells[&(0, 0)], "2500");
    });
    // A subsequent user scroll must not be undone by a render-time reveal loop.
    let surface = cx.debug_bounds("grid-surface").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: surface.center(),
        delta: gpui::ScrollDelta::Pixels(point(px(0.), px(1000.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    super::settle(cx);
    let top = cx.debug_bounds("grid-cell-0-0").unwrap();
    assert!(top.top() >= surface.top() + px(48.));
    super::settle(cx);
    assert_eq!(cx.debug_bounds("grid-cell-0-0").unwrap(), top);
}
#[gpui::test]
fn short_viewport_cancel_does_not_advance_or_scroll(cx: &mut TestAppContext) {
    let (owner, cx) = short_edit(cx, false);
    let before = cx.debug_bounds("grid-cell-0-0").unwrap();
    cx.simulate_input("discard");
    action("cancel", cx);
    assert_eq!(cx.debug_bounds("grid-cell-0-0").unwrap(), before);
    action("edit", cx);
    cx.simulate_input("2500");
    action("done", cx);
    owner.read_with(cx, |grid, _| {
        assert_eq!(grid.batches, vec![vec![(0, 0, "2500".into())]])
    });
    next_row_inside("grid-cell-1-0", false, cx);
}
#[gpui::test]
fn short_viewport_submit_reveals_below_frozen_row(cx: &mut TestAppContext) {
    let (owner, cx) = short_edit(cx, true);
    let frozen = cx.debug_bounds("grid-cell-0-0").unwrap();
    cx.simulate_input("2500");
    cx.simulate_keystrokes("enter");
    super::settle(cx);
    next_row_inside("grid-cell-2-0", true, cx);
    assert_eq!(cx.debug_bounds("grid-cell-0-0").unwrap(), frozen);
    owner.read_with(cx, |grid, _| {
        assert_eq!(grid.batches, vec![vec![(1, 0, "2500".into())]])
    });
}
