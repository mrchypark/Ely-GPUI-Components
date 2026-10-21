use crate::{
    primitives::FocusScope,
    tables::{Column, DataTable, Row},
    theme::Theme,
};
use gpui::{
    Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers, MouseButton,
    ParentElement, Render, SharedString, Styled, TestAppContext, VisualTestContext, Window, div,
    px, rems,
};

struct Tasks {
    virtualized: bool,
    width: f32,
    redirect: bool,
    focus: FocusHandle,
    values: Vec<(SharedString, SharedString)>,
    edits: Vec<(SharedString, SharedString, SharedString)>,
    selected: Vec<SharedString>,
    writable: bool,
    hidden: bool,
    editable: bool,
    reverse: bool,
}
impl Render for Tasks {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let select = owner.clone();
        let column = Column::new("name", "Name").width(rems(12.));
        let column = if self.editable {
            column.editable()
        } else {
            column
        };
        let mut values = self.values.clone();
        if self.reverse {
            values.reverse();
        }
        let mut table = DataTable::new(
            "touch-tasks",
            [column, Column::new("other", "Other").width(rems(6.))],
        )
        .rows(
            values
                .into_iter()
                .map(|(key, value)| {
                    Row::new(key, [crate::tables::Cell::Text(value), "fixed".into()])
                })
                .collect::<Vec<_>>(),
        )
        .selected(self.selected.clone())
        .on_select(move |keys, _, cx| {
            select.update(cx, |view, cx| {
                view.selected = keys.to_vec();
                cx.notify();
            })
        });
        table = if self.virtualized {
            table.virtualized().h(px(220.))
        } else {
            table.detail(|key, _, _| {
                div()
                    .debug_selector(|| "table-touch-detail".into())
                    .child(key.clone())
                    .into_any_element()
            })
        };
        if self.hidden {
            table = table.hidden(["name"]);
        }
        if self.writable {
            let destination = self.redirect.then(|| self.focus.clone());
            table = table.on_edit(move |key, at, text, window, cx| {
                owner.update(cx, |view, cx| {
                    view.edits.push((key.clone(), at.clone(), text.clone()));
                    view.values.iter_mut().find(|row| row.0 == *key).unwrap().1 = text.clone();
                    cx.notify();
                });
                if let Some(destination) = &destination {
                    window.focus(destination, cx);
                }
            });
        }
        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .child(table.w(px(self.width)))
    }
}
fn stage(cx: &mut TestAppContext) -> (Entity<Tasks>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.set_rem_size(px(16.));
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Tasks {
            virtualized: false,
            width: 390.,
            redirect: false,
            focus,
            values: vec![("a".into(), "Ada".into()), ("b".into(), "Alan".into())],
            edits: vec![],
            selected: vec![],
            writable: true,
            hidden: false,
            editable: true,
            reverse: false,
        }
    });
    cx.simulate_resize(gpui::size(px(390.), px(640.)));
    super::settle(cx);
    (view, cx)
}
fn tap(selector: &'static str, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds(selector).expect(selector);
    if selector.starts_with("table-touch-") && selector != "table-touch-detail" {
        assert!(bounds.size.height >= px(48.));
        assert!(bounds.left() >= px(0.) && bounds.right() <= px(390.));
    }
    cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(bounds.center(), MouseButton::Left, Modifiers::none());
    super::settle(cx);
}
#[gpui::test]
fn done_cancel_preserve_owner_selection_and_focus(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    tap("table-cell a name", cx);
    let selected = view.read_with(cx, |view, _| view.selected.clone());
    tap("table-touch-Edit", cx);
    cx.simulate_input("Ada edited");
    tap("table-touch-Done", cx);
    let focus = cx.update(|window, cx| window.focused(cx));
    assert!(focus.is_some());
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.edits,
            vec![("a".into(), "name".into(), "Ada edited".into())]
        );
        assert_eq!(view.values[0].1, "Ada edited");
        assert_eq!(view.selected, selected);
    });
    tap("table-touch-Edit", cx);
    cx.simulate_input("discard");
    tap("table-touch-Cancel", cx);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), focus);
    view.read_with(cx, |view, _| {
        assert_eq!(view.edits.len(), 1);
        assert_eq!(view.selected, selected);
    });
    tap("table-expand a", cx);
    assert!(cx.debug_bounds("table-touch-detail").is_some());
    tap("table-expand a", cx);
    assert!(cx.debug_bounds("table-touch-detail").is_none());
}
#[gpui::test]
fn target_uses_keys_after_row_reorder(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    tap("table-cell b name", cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.reverse = true;
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-touch-Edit", cx);
    cx.simulate_input("key b");
    tap("table-touch-Done", cx);
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.edits,
            vec![("b".into(), "name".into(), "key b".into())]
        )
    });
}
#[gpui::test]
fn removing_edit_target_cancels_without_callback(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    tap("table-cell a name", cx);
    tap("table-touch-Edit", cx);
    cx.simulate_input("stale");
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.values.remove(0);
            cx.notify();
        })
    });
    super::settle(cx);
    assert!(cx.debug_bounds("table-touch-Done").is_none());
    view.read_with(cx, |view, _| assert!(view.edits.is_empty()));
}
#[gpui::test]
fn hidden_column_and_revoked_editability_cancel(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    for revoke in [false, true] {
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.hidden = false;
                view.editable = true;
                cx.notify();
            })
        });
        super::settle(cx);
        tap("table-cell a name", cx);
        tap("table-touch-Edit", cx);
        cx.simulate_input("stale");
        cx.update(|_, cx| {
            view.update(cx, |view, cx| {
                if revoke {
                    view.editable = false;
                } else {
                    view.hidden = true;
                }
                cx.notify();
            })
        });
        super::settle(cx);
        assert!(cx.debug_bounds("table-touch-Done").is_none());
    }
    view.read_with(cx, |view, _| assert!(view.edits.is_empty()));
}
#[gpui::test]
fn read_only_cells_and_missing_callback_do_not_edit(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    tap("table-cell a other", cx);
    tap("table-touch-Edit", cx);
    assert!(cx.debug_bounds("table-touch-Done").is_none());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.writable = false;
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-cell a name", cx);
    assert!(cx.debug_bounds("table-touch-Edit").is_none());
    view.read_with(cx, |view, _| assert!(view.edits.is_empty()));
}

#[gpui::test]
fn horizontal_scroll_keeps_the_cell_key(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.width = 280.;
            cx.notify();
        })
    });
    super::settle(cx);
    let row = cx.debug_bounds("table-cell b name").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: row.center(),
        delta: gpui::ScrollDelta::Pixels(gpui::point(px(-96.), px(0.))),
        modifiers: Modifiers::none(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    super::settle(cx);
    let moved = cx.debug_bounds("table-cell b name").unwrap();
    assert!(
        moved.left() < row.left(),
        "horizontal scroll must actually move cells"
    );
    tap("table-cell b name", cx);
    tap("table-touch-Edit", cx);
    cx.simulate_input("scrolled");
    tap("table-touch-Done", cx);
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.edits,
            vec![("b".into(), "name".into(), "scrolled".into())]
        )
    });
}

#[gpui::test]
fn done_preserves_callback_focus_destination(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.redirect = true;
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-cell a name", cx);
    tap("table-touch-Edit", cx);
    cx.simulate_input("redirect");
    tap("table-touch-Done", cx);
    let destination = view.read_with(cx, |view, _| view.focus.clone());
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        Some(destination)
    );
    view.read_with(cx, |view, _| assert_eq!(view.edits.len(), 1));
}

#[gpui::test]
fn fixed_height_virtualized_edit_controls_and_input_are_contained(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.virtualized = true;
            view.values = (0..40)
                .map(|ix| (format!("r{ix}").into(), format!("Task {ix}").into()))
                .collect();
            cx.notify();
        })
    });
    super::settle(cx);
    tap("table-cell r0 name", cx);
    tap("table-touch-Edit", cx);
    for selector in ["table-touch-Done", "table-touch-Cancel", "input-root"] {
        let bounds = cx.debug_bounds(selector).expect(selector);
        assert!(bounds.left() >= px(0.) && bounds.right() <= px(390.));
        assert!(
            bounds.top() >= px(0.) && bounds.bottom() <= px(220.),
            "{selector}: {bounds:?}"
        );
    }
    cx.simulate_input("virtual edit");
    tap("table-touch-Done", cx);
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.edits,
            vec![("r0".into(), "name".into(), "virtual edit".into())]
        )
    });
}
