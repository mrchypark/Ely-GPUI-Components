use super::{Favorite, Favorites, favorite_tools};
use crate::theme::Theme;
use gpui::{
    AppContext, Context, Entity, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px, size,
};

struct Host {
    pages: Vec<Favorite>,
    enabled: bool,
    removed: Vec<String>,
    opened: Vec<String>,
    moves: usize,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (remove, open, moved) = (cx.entity(), cx.entity(), cx.entity());
        let mut favorites = Favorites::new("favorites", self.pages.clone())
            .on_open(move |key, _, cx| open.update(cx, |host, _| host.opened.push(key.to_string())))
            .on_move(move |_, _, _, cx| moved.update(cx, |host, _| host.moves += 1));
        if self.enabled {
            favorites = favorites.on_unpin(move |key, _, cx| {
                remove.update(cx, |host, cx| {
                    host.removed.push(key.to_string());
                    host.pages.retain(|page| page.key != *key);
                    cx.notify();
                })
            });
        }
        div().w(px(390.)).child(favorites)
    }
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

fn fixture(cx: &mut TestAppContext, enabled: bool) -> (Entity<Host>, &mut VisualTestContext) {
    cx.update(|cx| {
        crate::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, _| Host {
        pages: ["a", "b"]
            .map(|key| Favorite {
                key: key.into(),
                title: key.into(),
                icon: "*".into(),
            })
            .to_vec(),
        enabled,
        removed: vec![],
        opened: vec![],
        moves: 0,
    });
    cx.simulate_resize(size(px(390.), px(600.)));
    settle(cx);
    (view, cx)
}

fn tap(key: &str, cx: &mut VisualTestContext) {
    let selector = match key {
        "a" => "favorite-remove-a",
        "b" => "favorite-remove-b",
        _ => panic!("unknown fixture key: {key}"),
    };
    let bounds = cx.debug_bounds(selector).expect("remove action");
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(bounds.size.width >= px(48.) && bounds.size.height >= px(48.));
    assert!(
        bounds.left() >= px(0.)
            && bounds.top() >= px(0.)
            && bounds.right() <= viewport.width
            && bounds.bottom() <= viewport.height,
        "{selector}: {bounds:?} outside {viewport:?}"
    );
    // Deliberately do not send a pointer move/hover before activation.
    cx.simulate_click(bounds.center(), Modifiers::none());
    settle(cx);
}

#[test]
fn actual_remove_wrapper_is_visible_without_hover_only_in_touch_mode() {
    assert_eq!(
        favorite_tools("group".into(), true).style().opacity,
        Some(1.0)
    );
    assert_eq!(
        favorite_tools("group".into(), false).style().opacity,
        Some(0.0)
    );
}

#[gpui::test]
fn touch_remove_updates_owner_once_without_open_or_reorder(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, true);
    tap("a", cx);
    view.read_with(cx, |host, _| {
        assert_eq!(host.removed, ["a"]);
        assert_eq!(
            host.pages
                .iter()
                .map(|p| p.key.as_ref())
                .collect::<Vec<_>>(),
            ["b"]
        );
        assert!(host.opened.is_empty());
        assert_eq!(host.moves, 0);
    });
    assert!(cx.debug_bounds("favorite-remove-a").is_none());
    tap("b", cx);
    view.read_with(cx, |host, _| {
        assert_eq!(host.removed, ["a", "b"]);
        assert!(host.pages.is_empty() && host.opened.is_empty());
        assert_eq!(host.moves, 0);
    });
}

#[gpui::test]
fn removal_without_callback_stays_inert_after_rerender(cx: &mut TestAppContext) {
    let (view, cx) = fixture(cx, false);
    tap("a", cx);
    view.update(cx, |host, cx| {
        host.pages.reverse();
        cx.notify();
    });
    settle(cx);
    tap("b", cx);
    view.read_with(cx, |host, _| {
        assert_eq!(host.pages.len(), 2);
        assert!(host.removed.is_empty());
        assert!(
            host.opened.is_empty(),
            "disabled remove must not open its row"
        );
        assert_eq!(host.moves, 0);
    });
}
