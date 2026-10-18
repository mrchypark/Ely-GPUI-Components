use crate::{
    primitives::FocusScope,
    tables::TableToolbar,
    theme::{Density, Theme},
};
use gpui::{
    AppContext, Bounds, Context, Entity, FocusHandle, IntoElement, Modifiers, MouseButton,
    ParentElement, Pixels, Render, SharedString, Styled, TestAppContext, VisualTestContext, Window,
    point, px, size,
};
struct Tools {
    full: bool,
    search: Entity<crate::forms::TextInput>,
    root: FocusHandle,
    bounds: Option<Bounds<Pixels>>,
    columns: bool,
    hidden: Vec<SharedString>,
    density: Density,
    changes: usize,
}
impl Render for Tools {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let toolbar = TableToolbar::new("bounded-tools");
        let mut toolbar = toolbar;
        if self.columns || self.full {
            let owner = owner.clone();
            toolbar = toolbar.columns(
                [("a", "Alpha"), ("b", "Beta")],
                self.hidden.clone(),
                move |keys, _, cx| {
                    owner.update(cx, |view, cx| {
                        view.hidden = keys.to_vec();
                        view.changes += 1;
                        cx.notify();
                    })
                },
            )
        }
        if !self.columns || self.full {
            toolbar = toolbar.density(self.density, move |density, _, cx| {
                owner.update(cx, |view, cx| {
                    view.density = density;
                    view.changes += 1;
                    cx.notify();
                })
            });
        }
        if self.full {
            toolbar = toolbar
                .search(&self.search)
                .filter(0, |_, _| {})
                .export(|_, _| {});
        }
        let toolbar = match self.bounds {
            Some(bounds) => toolbar.usable_bounds(bounds),
            None => toolbar,
        };
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(toolbar)
    }
}
fn stage(cx: &mut TestAppContext, columns: bool) -> (Entity<Tools>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        window.set_rem_size(px(16.));
        let root = cx.focus_handle();
        window.focus(&root, cx);
        Tools {
            full: false,
            search: cx.new(|cx| crate::forms::TextInput::new(window, cx)),
            root,
            bounds: Some(Bounds::new(
                point(px(20.), px(65.)),
                size(px(340.), px(220.)),
            )),
            columns,
            hidden: vec![],
            density: Density::Standard,
            changes: 0,
        }
    });
    cx.simulate_resize(size(px(390.), px(480.)));
    super::settle(cx);
    (view, cx)
}
fn tap(selector: &'static str, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds(selector).expect(selector);
    cx.simulate_mouse_down(bounds.center(), MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(bounds.center(), MouseButton::Left, Modifiers::none());
    super::settle(cx);
}
fn contained(view: &Entity<Tools>, cx: &mut VisualTestContext) {
    let panel = cx.debug_bounds("menu-panel-0").expect("menu open");
    let bounds = view.read_with(cx, |view, _| view.bounds.unwrap());
    assert!(panel.left() >= bounds.left() && panel.right() <= bounds.right());
    assert!(panel.top() >= bounds.top() && panel.bottom() <= bounds.bottom());
}
fn exercise(columns: bool, cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, columns);
    let trigger = if columns {
        "table-columns-trigger"
    } else {
        "table-density-trigger"
    };
    tap(trigger, cx);
    contained(&view, cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.bounds = Some(Bounds::new(
                point(px(40.), px(80.)),
                size(px(280.), px(180.)),
            ));
            cx.notify();
        })
    });
    super::settle(cx);
    contained(&view, cx);
    tap(
        if columns {
            "menu-row-Alpha"
        } else {
            "menu-row-Compact"
        },
        cx,
    );
    view.read_with(cx, |view, _| {
        assert_eq!(view.changes, 1);
        if columns {
            assert_eq!(view.hidden, vec![SharedString::from("a")]);
        } else {
            assert_eq!(view.density, Density::Compact);
        }
    });
    let before = cx.update(|window, cx| window.focused(cx));
    tap(trigger, cx);
    contained(&view, cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.bounds = Some(Bounds::new(point(px(0.), px(0.)), size(px(0.), px(0.))));
            cx.notify();
        })
    });
    super::settle(cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    assert_eq!(cx.update(|window, cx| window.focused(cx)), before);
    cx.simulate_keystrokes("down enter space");
    super::settle(cx);
    tap(trigger, cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    view.read_with(cx, |view, _| assert_eq!(view.changes, 1));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.bounds = None;
            cx.notify();
        })
    });
    super::settle(cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    tap(trigger, cx);
    assert!(cx.debug_bounds("menu-panel-0").is_some());
    cx.simulate_mouse_down(
        point(px(380.), px(450.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        point(px(380.), px(450.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    super::settle(cx);
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    view.read_with(cx, |view, _| assert_eq!(view.changes, 1));
}
#[gpui::test]
fn columns_bounds_live_shrink_empty_and_mutation(cx: &mut TestAppContext) {
    exercise(true, cx);
}
#[gpui::test]
fn density_bounds_live_shrink_empty_and_mutation(cx: &mut TestAppContext) {
    exercise(false, cx);
}

#[gpui::test]
fn full_phone_toolbar_keeps_both_menu_triggers_reachable(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, true);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.full = true;
            cx.notify();
        })
    });
    super::settle(cx);
    for trigger in ["table-columns-trigger", "table-density-trigger"] {
        let bounds = cx.debug_bounds(trigger).expect(trigger);
        assert!(
            bounds.left() >= px(0.) && bounds.right() <= px(390.),
            "{trigger}: {bounds:?}"
        );
        assert!(bounds.top() >= px(0.) && bounds.bottom() <= px(480.));
        tap(trigger, cx);
        contained(&view, cx);
        tap(
            if trigger == "table-columns-trigger" {
                "menu-row-Alpha"
            } else {
                "menu-row-Compact"
            },
            cx,
        );
    }
    view.read_with(cx, |view, _| {
        assert_eq!(view.changes, 2);
        assert_eq!(view.hidden, vec![SharedString::from("a")]);
        assert_eq!(view.density, Density::Compact);
    });
}
