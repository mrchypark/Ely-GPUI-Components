use super::FileGrid;
use crate::{
    files::tests::{press, settle, then},
    lists::DirEntry,
    theme::Theme,
};
use gpui::{
    AppContext, Context, Entity, IntoElement, Modifiers, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, px, size,
};

struct Host {
    names: Vec<&'static str>,
    selected: Option<SharedString>,
    writable: bool,
    picks: Vec<String>,
    opens: Vec<String>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pick = cx.entity();
        let open = cx.entity();
        let mut grid = FileGrid::new(
            "grid",
            self.names
                .iter()
                .map(|name| DirEntry::file(*name, 10, then())),
        )
        .on_select(move |name, _, cx| {
            pick.update(cx, |host, cx| {
                host.selected = Some(name.clone());
                host.picks.push(name.to_string());
                cx.notify();
            })
        });
        if let Some(name) = &self.selected {
            grid = grid.selected(name.clone());
        }
        if self.writable {
            grid = grid.on_open(move |entry, _, cx| {
                open.update(cx, |host, _| host.opens.push(entry.name().to_string()))
            });
        }
        div().w(px(390.)).child(grid)
    }
}

fn fixture(cx: &mut TestAppContext, touch: bool) -> (Entity<Host>, &mut VisualTestContext) {
    crate::files::tests::setup(cx);
    cx.update(|cx| Theme::update(cx, |theme| theme.touch_mode = touch));
    let (view, cx) = cx.add_window_view(|_, _| Host {
        names: vec!["a", "b"],
        selected: None,
        writable: true,
        picks: vec![],
        opens: vec![],
    });
    cx.simulate_resize(size(px(390.), px(600.)));
    settle(cx);
    (view, cx)
}

fn tap(selector: &'static str, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds(selector).expect("visible control");
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(bounds.size.width >= px(48.) && bounds.size.height >= px(48.));
    assert!(
        bounds.left() >= px(0.)
            && bounds.top() >= px(0.)
            && bounds.right() <= viewport.width
            && bounds.bottom() <= viewport.height,
        "{selector}: {bounds:?} outside {viewport:?}"
    );
    cx.simulate_click(bounds.center(), Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn touch_selects_then_opens_once_and_tracks_reordered_entries(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    tap("file-grid-open", cx);
    tap("file-grid-tile-1", cx);
    view.read_with(cx, |host, _| {
        assert_eq!(host.picks, ["b"]);
        assert!(host.opens.is_empty());
    });
    tap("file-grid-open", cx);
    view.update(cx, |host, cx| {
        host.names.reverse();
        cx.notify();
    });
    settle(cx);
    tap("file-grid-open", cx);
    view.read_with(cx, |host, _| {
        assert_eq!(host.opens, ["b", "b"]);
        assert_eq!(host.picks, ["b"], "Open must not select again");
        assert_eq!(host.selected.as_deref(), Some("b"));
    });
}

#[gpui::test]
fn touch_open_rejects_missing_callback_stale_selection_and_empty_entries(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    tap("file-grid-tile-0", cx);
    view.update(cx, |host, cx| {
        host.writable = false;
        cx.notify();
    });
    settle(cx);
    tap("file-grid-open", cx);
    view.update(cx, |host, cx| {
        host.writable = true;
        host.names = vec!["b"];
        cx.notify();
    });
    settle(cx);
    tap("file-grid-open", cx);
    view.update(cx, |host, cx| {
        host.names.clear();
        cx.notify();
    });
    settle(cx);
    tap("file-grid-open", cx);
    view.read_with(cx, |host, _| {
        assert!(host.opens.is_empty());
        assert_eq!(host.picks, ["a"]);
    });
}

#[gpui::test]
fn touch_double_press_still_only_selects(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    let position = cx.debug_bounds("file-grid-tile-0").unwrap().center();
    cx.simulate_event(gpui::MouseDownEvent {
        button: gpui::MouseButton::Left,
        position,
        modifiers: Modifiers::none(),
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        button: gpui::MouseButton::Left,
        position,
        modifiers: Modifiers::none(),
        click_count: 2,
    });
    settle(cx);
    view.read_with(cx, |host, _| {
        assert_eq!(host.picks, ["a"]);
        assert!(host.opens.is_empty());
    });
    tap("file-grid-open", cx);
    view.read_with(cx, |host, _| assert_eq!(host.opens, ["a"]));
}

#[gpui::test]
fn desktop_keeps_enter_open_without_touch_action(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    assert!(cx.debug_bounds("file-grid-open").is_none());
    cx.update(|window, cx| window.focus_next(cx));
    press("home", cx);
    press("enter", cx);
    view.read_with(cx, |host, _| assert_eq!(host.opens, ["a"]));
}
