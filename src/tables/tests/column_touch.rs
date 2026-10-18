use crate::{
    primitives::FocusScope,
    tables::{Cell, Column, DataTable, Row, SortKey},
    theme::Theme,
};
use gpui::{
    Context, Entity, FocusHandle, IntoElement, Modifiers, MouseButton, ParentElement, Render,
    SharedString, Styled, TestAppContext, VisualTestContext, Window, px, rems, size,
};

struct Columns {
    root: FocusHandle,
    width: f32,
    pinned: bool,
    flexible: bool,
    hidden: bool,
    hide_target: bool,
    writable: bool,
    removed: bool,
    virtualized: bool,
    edits: Vec<(SharedString, SharedString, SharedString)>,
    selects: usize,
    selected: Vec<SharedString>,
    values: [SharedString; 2],
}
impl Render for Columns {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut columns = Vec::new();
        if self.pinned {
            columns.push(Column::new("p", "Pinned").width(rems(6.)).pinned());
        }
        if !self.removed {
            let a = Column::new("a", "Alpha").editable();
            columns.push(if self.flexible { a } else { a.width(rems(8.)) });
        }
        columns.push(Column::new("h", "Hidden slot").width(rems(5.)));
        columns.push(Column::new("b", "Beta").width(rems(8.)));
        let rows = (0..2)
            .map(|ix| {
                let mut cells = Vec::new();
                if self.pinned {
                    cells.push(Cell::Text("pin".into()));
                }
                if !self.removed {
                    cells.push(Cell::Text(self.values[ix].clone()));
                }
                cells.extend([Cell::Text("hidden".into()), Cell::Text("beta".into())]);
                Row::new(format!("r{ix}"), cells)
            })
            .collect::<Vec<_>>();
        let editor = cx.entity();
        let select = cx.entity();
        let mut table = DataTable::new("column-touch", columns)
            .rows(rows)
            .sorts([SortKey {
                column: "a".into(),
                rising: true,
            }])
            .selected(self.selected.clone())
            .on_select(move |keys, _, cx| {
                select.update(cx, |view, cx| {
                    view.selected = keys.to_vec();
                    view.selects += 1;
                    cx.notify();
                })
            })
            .w(px(self.width));
        if self.writable {
            table = table.on_edit(move |row, column, text, _, cx| {
                editor.update(cx, |view, cx| {
                    view.edits.push((row.clone(), column.clone(), text.clone()));
                    view.values[if row.as_ref() == "r0" { 0 } else { 1 }] = text.clone();
                    cx.notify();
                })
            });
        }
        if self.hide_target {
            table = table.hidden(["h", "a"]);
        } else if self.hidden {
            table = table.hidden(["h"]);
        }
        if self.virtualized {
            table = table.virtualized().h(px(480.));
        }
        FocusScope::new(&self.root).root().size_full().child(table)
    }
}
fn stage(cx: &mut TestAppContext, pinned: bool) -> (Entity<Columns>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.set_rem_size(px(16.));
        let root = cx.focus_handle();
        window.focus(&root, cx);
        Columns {
            root,
            width: 390.,
            pinned,
            flexible: false,
            hidden: true,
            hide_target: false,
            writable: true,
            removed: false,
            virtualized: false,
            edits: vec![],
            selects: 0,
            selected: vec![],
            values: ["Z".into(), "A".into()],
        }
    });
    cx.simulate_resize(size(px(390.), px(640.)));
    super::settle(cx);
    (view, cx)
}
fn tap(selector: &'static str, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds(selector).expect(selector);
    cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(bounds.center(), MouseButton::Left, Modifiers::none());
    super::settle(cx);
}
fn width(cx: &mut VisualTestContext) -> gpui::Pixels {
    cx.debug_bounds("table-head a").unwrap().size.width
}

#[gpui::test]
fn move_swaps_visible_slots_preserving_pins_hidden_sort_and_edit_identity(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, true);
    tap("table-columns-toggle", cx);
    let pin = cx.debug_bounds("table-head p").unwrap();
    tap("table-column-Move right", cx);
    assert_eq!(cx.debug_bounds("table-head p").unwrap(), pin);
    tap("table-column-Next", cx); // Alpha, skipping neither pin nor visible key.
    let a = cx.debug_bounds("table-head a").unwrap();
    let b = cx.debug_bounds("table-head b").unwrap();
    let first = cx.debug_bounds("table-cell r1 a").unwrap();
    let second = cx.debug_bounds("table-cell r0 a").unwrap();
    assert!(first.top() < second.top());
    tap("table-column-Move left", cx);
    assert_eq!(cx.debug_bounds("table-head a").unwrap(), a);
    tap("table-column-Move right", cx);
    assert_eq!(cx.debug_bounds("table-head a").unwrap().left(), b.left());
    assert_eq!(cx.debug_bounds("table-head b").unwrap().left(), a.left());
    assert_eq!(
        cx.debug_bounds("table-cell r1 a").unwrap().left() - first.left(),
        b.left() - a.left()
    );
    assert_eq!(cx.debug_bounds("table-head p").unwrap(), pin);
    assert_eq!(
        cx.debug_bounds("table-cell r1 a").unwrap().top(),
        first.top()
    );
    view.read_with(cx, |view, _| {
        assert!(view.edits.is_empty());
        assert_eq!(view.selects, 0);
    });
    tap("table-cell r1 a", cx);
    tap("table-touch-Edit", cx);
    cx.simulate_input("Changed");
    let focus = cx.update(|window, cx| window.focused(cx));
    let selected = view.read_with(cx, |view, _| view.selects);
    tap("table-column-Move left", cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
    tap("table-touch-Done", cx);
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.edits,
            vec![("r1".into(), "a".into(), "Changed".into())]
        );
        assert_eq!(view.selects, selected);
    });
    tap("table-column-Move right", cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.hidden = false;
            cx.notify();
        })
    });
    super::settle(cx);
    let b = cx.debug_bounds("table-head b").unwrap();
    let h = cx.debug_bounds("table-head h").unwrap();
    let a = cx.debug_bounds("table-head a").unwrap();
    assert!(
        b.left() < h.left() && h.left() < a.left(),
        "hidden slot stays between swapped keys"
    );
}

#[gpui::test]
fn resize_uses_measured_width_minimum_and_fixed_reset_tracks_rem(cx: &mut TestAppContext) {
    let (_, cx) = stage(cx, false);
    tap("table-columns-toggle", cx);
    let original = width(cx);
    tap("table-column-Wider", cx);
    assert_eq!(width(cx), original + px(24.));
    assert_eq!(
        cx.debug_bounds("table-cell r1 a").unwrap().size.width,
        width(cx)
    );
    for _ in 0..10 {
        tap("table-column-Narrower", cx);
    }
    assert_eq!(width(cx), px(80.));
    tap("table-column-Narrower", cx);
    assert_eq!(width(cx), px(80.));
    cx.update(|window, _| window.set_rem_size(px(20.)));
    super::settle(cx);
    tap("table-column-Reset width", cx);
    assert_eq!(width(cx), px(160.));
    cx.update(|window, _| window.set_rem_size(px(16.)));
    super::settle(cx);
    assert_eq!(width(cx), original);
}

#[gpui::test]
fn flexible_reset_returns_to_viewport_allocation(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, false);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.flexible = true;
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-columns-toggle", cx);
    let original = width(cx);
    tap("table-column-Wider", cx);
    assert_eq!(width(cx), original + px(24.));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.width = 320.;
            cx.notify();
        })
    });
    super::settle(cx);
    assert_eq!(width(cx), original + px(24.));
    tap("table-column-Reset width", cx);
    let reset = width(cx);
    assert!(reset < original);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.width = 390.;
            cx.notify();
        })
    });
    super::settle(cx);
    assert_eq!(width(cx), original);
}

#[gpui::test]
fn removed_target_is_inert_until_explicit_choice(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, false);
    tap("table-columns-toggle", cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.removed = true;
            cx.notify();
        })
    });
    super::settle(cx);
    let before = cx.debug_bounds("table-head b").unwrap();
    for action in [
        "table-column-Move right",
        "table-column-Wider",
        "table-column-Reset width",
    ] {
        tap(action, cx);
    }
    assert_eq!(cx.debug_bounds("table-head b").unwrap(), before);
    view.read_with(cx, |view, _| {
        assert!(view.edits.is_empty());
        assert_eq!(view.selects, 0);
    });
    tap("table-column-Next", cx);
    tap("table-column-Wider", cx);
    assert_eq!(
        cx.debug_bounds("table-head b").unwrap().size.width,
        before.size.width + px(24.)
    );
}

#[gpui::test]
fn all_column_buttons_are_48_and_contained_at_phone_widths_in_virtual_table(
    cx: &mut TestAppContext,
) {
    let (view, cx) = stage(cx, false);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.virtualized = true;
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-columns-toggle", cx);
    for wide in [390., 320.] {
        cx.simulate_resize(size(px(wide), px(640.)));
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.width = wide;
                cx.notify();
            })
        });
        super::settle(cx);
        for selector in [
            "table-columns-toggle",
            "table-column-Previous",
            "table-column-Next",
            "table-column-Move left",
            "table-column-Move right",
            "table-column-Narrower",
            "table-column-Wider",
            "table-column-Reset width",
        ] {
            let bounds = cx.debug_bounds(selector).expect(selector);
            assert!(
                bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
                "actual target {selector}: {bounds:?}"
            );
            assert!(
                bounds.left() >= px(0.) && bounds.right() <= px(wide),
                "{selector}: {bounds:?}"
            );
            assert!(
                bounds.top() >= px(0.) && bounds.bottom() <= px(480.),
                "{selector}: {bounds:?}"
            );
        }
        tap("table-column-Wider", cx);
        tap("table-column-Reset width", cx);
        let row = cx.debug_bounds("table-cell r1 a").unwrap();
        assert!(row.top() >= px(0.) && row.bottom() <= px(480.));
    }
}

#[gpui::test]
fn hidden_target_stays_inert_and_column_controls_work_without_edit_callback(
    cx: &mut TestAppContext,
) {
    let (view, cx) = stage(cx, false);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.writable = false;
            cx.notify();
        })
    });
    super::settle(cx);
    assert!(cx.debug_bounds("table-touch-Edit").is_none());
    tap("table-columns-toggle", cx);
    let original = width(cx);
    tap("table-column-Wider", cx);
    assert_eq!(width(cx), original + px(24.));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.hide_target = true;
            cx.notify();
        })
    });
    super::settle(cx);
    let beta = cx.debug_bounds("table-head b").unwrap();
    tap("table-column-Wider", cx);
    assert_eq!(cx.debug_bounds("table-head b").unwrap(), beta);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.hide_target = false;
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-column-Reset width", cx);
    assert_eq!(
        width(cx),
        original + px(24.),
        "restoring a hidden column does not retarget it"
    );
    tap("table-column-Next", cx);
    tap("table-column-Reset width", cx);
    assert_eq!(width(cx), original);
    view.read_with(cx, |view, _| {
        assert!(view.edits.is_empty());
        assert_eq!(view.selects, 0);
    });
}
