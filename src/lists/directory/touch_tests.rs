use super::{DirEntry, DirectoryListing};
use crate::theme::Theme;
use gpui::{
    Context, Entity, IntoElement, Modifiers, ParentElement, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, div, px, size,
};

struct Host {
    entries: Vec<DirEntry>,
    selected: Option<SharedString>,
    enabled: bool,
    selected_calls: Vec<SharedString>,
    opened: Vec<DirEntry>,
}
impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (pick, open) = (cx.entity(), cx.entity());
        let mut list = DirectoryListing::new("listing", ["Home"], self.entries.clone()).on_select(
            move |name, _, cx| {
                pick.update(cx, |host, cx| {
                    host.selected = Some(name.clone());
                    host.selected_calls.push(name.clone());
                    cx.notify();
                })
            },
        );
        if let Some(name) = &self.selected {
            list = list.selected(name.clone());
        }
        if self.enabled {
            list = list.on_open(move |entry, _, cx| {
                open.update(cx, |host, _| host.opened.push(entry.clone()))
            });
        }
        div().w_full().child(list)
    }
}
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}
fn fixture(cx: &mut TestAppContext, touch: bool) -> (Entity<Host>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = touch;
            theme.reduced_motion = true;
        });
    });
    let at = jiff::Timestamp::from_second(1_700_000_000).unwrap();
    let (view, cx) = cx.add_window_view(|_, _| Host {
        entries: vec![DirEntry::file("a", 10, at), DirEntry::file("b", 20, at)],
        selected: None,
        enabled: true,
        selected_calls: vec![],
        opened: vec![],
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    settle(cx);
    (view, cx)
}
fn tap(selector: &'static str, control: bool, cx: &mut VisualTestContext) {
    let hit = cx.debug_bounds(selector).expect(selector);
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(
        hit.left() >= px(0.)
            && hit.top() >= px(0.)
            && hit.right() <= viewport.width
            && hit.bottom() <= viewport.height,
        "{selector}: {hit:?}"
    );
    if control {
        assert!(
            hit.size.width >= px(48.) && hit.size.height >= px(48.),
            "{selector}: {hit:?}"
        );
    }
    cx.simulate_click(hit.center(), Modifiers::none());
    settle(cx);
}
#[gpui::test]
fn touch_open_tracks_selected_identity_across_sort_and_owner_updates(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    for width in [390., 320.] {
        cx.simulate_resize(size(px(width), px(844.)));
        settle(cx);
        tap("directory-open", true, cx);
    }
    view.read_with(cx, |host, _| assert!(host.opened.is_empty()));
    tap("directory-entry-a", false, cx);
    let before = cx.debug_bounds("directory-entry-a").unwrap();
    tap("directory-sort-Name", true, cx);
    assert!(cx.debug_bounds("directory-entry-a").unwrap().top() > before.top());
    view.read_with(cx, |host, _| {
        assert!(host.opened.is_empty());
        assert_eq!(host.selected_calls, [SharedString::from("a")]);
    });
    tap("directory-open", true, cx);
    view.update(cx, |host, cx| {
        host.entries.reverse();
        host.entries
            .iter_mut()
            .find(|entry| entry.name().as_ref() == "a")
            .unwrap()
            .size = Some(99);
        cx.notify();
    });
    settle(cx);
    tap("directory-open", true, cx);
    view.read_with(cx, |host, _| {
        assert_eq!(
            host.opened
                .iter()
                .map(|entry| (entry.name().as_ref(), entry.size()))
                .collect::<Vec<_>>(),
            [("a", Some(10)), ("a", Some(99))]
        );
        assert_eq!(host.selected_calls, [SharedString::from("a")]);
    });
}
#[gpui::test]
fn missing_callback_stale_selection_and_empty_entries_disable_open(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    tap("directory-entry-a", false, cx);
    view.update(cx, |host, cx| {
        host.enabled = false;
        cx.notify();
    });
    settle(cx);
    tap("directory-open", true, cx);
    view.update(cx, |host, cx| {
        host.enabled = true;
        host.entries.retain(|entry| entry.name().as_ref() != "a");
        cx.notify();
    });
    settle(cx);
    tap("directory-open", true, cx);
    view.update(cx, |host, cx| {
        host.entries.clear();
        cx.notify();
    });
    settle(cx);
    tap("directory-open", true, cx);
    view.read_with(cx, |host, _| {
        assert!(host.opened.is_empty());
        assert_eq!(host.selected_calls.len(), 1);
    });
}
#[gpui::test]
fn desktop_double_click_and_enter_still_activate_without_touch_button(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    assert!(cx.debug_bounds("directory-open").is_none());
    tap("directory-entry-a", false, cx);
    view.read_with(cx, |host, _| assert!(host.opened.is_empty()));
    let position = cx.debug_bounds("directory-entry-a").unwrap().center();
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
    cx.simulate_keystrokes("enter");
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").unwrap(),
    });
    settle(cx);
    view.read_with(cx, |host, _| {
        assert_eq!(
            host.opened
                .iter()
                .map(|entry| entry.name().as_ref())
                .collect::<Vec<_>>(),
            ["a", "a"]
        );
    });
}
