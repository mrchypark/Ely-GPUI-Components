use super::*;

fn run(cx: &mut TestAppContext, frozen: bool) {
    let (owner, cx) = stage(cx, frozen);
    let edited = (usize::from(frozen), if frozen { 2 } else { 0 });
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.collapse_on_commit = true;
            grid.frozen = if frozen { (1, 1) } else { (0, 0) };
            grid.cells.insert(edited, "2400".into());
            cx.notify();
        })
    });
    super::super::settle(cx);
    cell(edited.0, edited.1, frozen, cx);
    action("edit", cx);
    cx.simulate_input("2500");
    // Commit itself collapses the controlled owner, before the reveal prepaint.
    action("done", cx);
    owner.read_with(cx, |grid, _| {
        assert_eq!(
            grid.batches,
            vec![vec![(edited.0, edited.1, "2500".into())]]
        );
        assert_eq!(grid.cells[&edited], "2500");
    });
    super::super::settle(cx);
    super::super::settle(cx);
    // Restore X first while there is still no room below the header/frozen row.
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.width = 300.;
            cx.notify();
        })
    });
    super::super::settle(cx);
    cx.update(|_, cx| {
        owner.update(cx, |grid, cx| {
            grid.height = if frozen { 200. } else { 150. };
            cx.notify();
        })
    });
    super::super::settle(cx);
    let selected = cx
        .debug_bounds(if frozen {
            "grid-cell-2-2"
        } else {
            "grid-cell-1-0"
        })
        .expect("successor drawn after restore");
    let surface = cx.debug_bounds("grid-surface").unwrap();
    let (label, wide, tall) = cx.update(|window, cx| {
        use crate::theme::{ActiveTheme, ControlSize, Density};
        let theme = cx.theme();
        let rem = window.rem_size();
        (
            if frozen {
                theme.control_height(ControlSize::Lg).to_pixels(rem) * 1.25
            } else {
                px(0.)
            },
            theme.grid_column().to_pixels(rem),
            theme.touch_minimum(theme.table_row(Density::Compact).to_pixels(rem)),
        )
    });
    assert!(selected.left() >= surface.left() + label + if frozen { wide } else { px(0.) });
    assert!(
        selected.right() <= surface.right(),
        "horizontal successor containment: {selected:?} within {surface:?}"
    );
    assert!(selected.top() >= surface.top() + tall * if frozen { 2. } else { 1. });
    assert!(
        selected.bottom() <= surface.bottom(),
        "vertical successor containment: {selected:?} within {surface:?}"
    );
    if frozen {
        let fixed = cx.debug_bounds("grid-cell-0-0").unwrap();
        assert!(fixed.left() >= surface.left() + label && fixed.top() >= surface.top() + tall);
    }
    owner.read_with(cx, |grid, _| assert_eq!(grid.batches.len(), 1));
}

#[gpui::test]
fn commit_collapse_restore_reveals_successor_once(cx: &mut TestAppContext) {
    run(cx, false);
}

#[gpui::test]
fn commit_collapse_restore_respects_frozen_rows_and_horizontal_region(cx: &mut TestAppContext) {
    run(cx, true);
}
