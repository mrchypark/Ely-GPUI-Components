use crate::ui::keep;
use ely_gpui_component::{documents::Favorite, theme::Theme};
use gpui::{
    AppContext, Context, Entity, IntoElement, Modifiers, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, point, px, size,
};

struct Story {
    title: &'static str,
    looking: Option<Entity<Option<SharedString>>>,
    preview_count: Option<Entity<usize>>,
    grid_opened: Option<Entity<(usize, Option<SharedString>)>>,
    listing: Option<Entity<(usize, SharedString)>>,
    actions: Option<Entity<(usize, SharedString)>>,
    favorites: Option<Entity<Vec<Favorite>>>,
    removed: Option<Entity<Vec<SharedString>>>,
}
impl Render for Story {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        cx.set_global(crate::ui::Story::new(self.title.into()));
        let bounds = gpui::Bounds::new(point(px(0.), px(0.)), window.viewport_size());
        let body = super::render(window, cx, Some(bounds));
        assert!(
            cx.remove_global::<crate::ui::Story>()
                .missing("files")
                .is_none()
        );
        // Read the same retained fixture entities after the actual page initializes them.
        self.looking = Some(keep("files-looking", || None, window, cx));
        self.preview_count = Some(keep("files-look-opens", || 0, window, cx));
        self.grid_opened = Some(keep("files-grid-opened", || (0, None), window, cx));
        self.listing = Some(keep(
            "files-listing-opened",
            || (0, SharedString::default()),
            window,
            cx,
        ));
        self.actions = Some(keep(
            "files-action-result",
            || (0, SharedString::default()),
            window,
            cx,
        ));
        self.favorites = Some(keep("files-favorites-pages", Vec::new, window, cx));
        self.removed = Some(keep("files-favorites-removed", Vec::new, window, cx));
        div().w_full().child(body)
    }
}
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}
fn stage<'a>(cx: &'a mut TestAppContext, title: &'static str) -> (Entity<Story>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, _| Story {
        title,
        looking: None,
        preview_count: None,
        grid_opened: None,
        listing: None,
        actions: None,
        favorites: None,
        removed: None,
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    settle(cx);
    (view, cx)
}
fn tap(selector: &'static str, minimum: bool, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds(selector).expect(selector);
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
    assert!(
        bounds.left() >= px(0.)
            && bounds.top() >= px(0.)
            && bounds.right() <= viewport.width
            && bounds.bottom() <= viewport.height,
        "{selector}: {bounds:?} outside {viewport:?}"
    );
    if minimum {
        assert!(
            bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
            "{selector}: {bounds:?}"
        );
    }
    cx.simulate_click(bounds.center(), Modifiers::none());
    settle(cx);
}
#[gpui::test]
fn file_grid_records_callback_identity_only_when_opened(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, "FileGrid");
    tap("file-grid-open", true, cx);
    tap("file-grid-tile-0", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(*view.grid_opened.as_ref().unwrap().read(cx), (0, None));
    });
    tap("file-grid-open", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.grid_opened.as_ref().unwrap().read(cx),
            (1, Some(SharedString::from("Design")))
        );
    });
    assert!(cx.debug_bounds("gallery-file-opened-Design").is_some());
    tap("file-grid-tile-1", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.grid_opened.as_ref().unwrap().read(cx),
            (1, Some(SharedString::from("Design")))
        );
    });
    assert!(cx.debug_bounds("gallery-file-opened-Design").is_some());
    assert!(cx.debug_bounds("gallery-file-opened-Recordings").is_none());
    tap("file-grid-open", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.grid_opened.as_ref().unwrap().read(cx),
            (2, Some(SharedString::from("Recordings")))
        );
    });
    assert!(cx.debug_bounds("gallery-file-opened-Design").is_none());
    assert!(cx.debug_bounds("gallery-file-opened-Recordings").is_some());
    settle(cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.grid_opened.as_ref().unwrap().read(cx),
            (2, Some(SharedString::from("Recordings")))
        );
    });
}
#[gpui::test]
fn file_list_select_then_open_records_one_local_callback(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, "FileList");
    tap("directory-open", true, cx);
    tap("directory-entry-Design", false, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(view.listing.as_ref().unwrap().read(cx).0, 0)
    });
    tap("directory-open", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.listing.as_ref().unwrap().read(cx),
            (1, SharedString::from("Design"))
        )
    });
    cx.simulate_resize(size(px(320.), px(844.)));
    settle(cx);
    tap("directory-open", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.listing.as_ref().unwrap().read(cx),
            (2, SharedString::from("Design"))
        )
    });
}
#[gpui::test]
fn quick_look_opens_selected_preview_and_closes_by_touch(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, "FilePreview / QuickLook");
    tap("file-grid-open", true, cx);
    view.read_with(cx, |view, cx| {
        assert!(view.looking.as_ref().unwrap().read(cx).is_none())
    });
    tap("file-grid-tile-1", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(*view.preview_count.as_ref().unwrap().read(cx), 0)
    });
    tap("file-grid-open", true, cx);
    assert!(cx.debug_bounds("files-preview-Brief.md").is_some());
    view.read_with(cx, |view, cx| {
        assert_eq!(
            view.looking.as_ref().unwrap().read(cx).as_deref(),
            Some("Brief.md")
        );
        assert_eq!(*view.preview_count.as_ref().unwrap().read(cx), 1);
    });
    tap("files-preview-close", true, cx);
    assert!(cx.debug_bounds("files-preview-Brief.md").is_none());
    cx.simulate_resize(size(px(320.), px(844.)));
    settle(cx);
    tap("file-grid-tile-2", true, cx);
    tap("file-grid-open", true, cx);
    assert!(cx.debug_bounds("files-preview-Archive.zip").is_some());
    view.read_with(cx, |view, cx| {
        assert_eq!(
            view.looking.as_ref().unwrap().read(cx).as_deref(),
            Some("Archive.zip")
        );
        assert_eq!(*view.preview_count.as_ref().unwrap().read(cx), 2);
    });
    tap("files-preview-close", true, cx);
}
#[gpui::test]
fn visible_file_actions_dispatch_same_menu_once_and_reopen(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, "FileContextMenu");
    for (index, (width, row, label)) in [
        (390., "menu-row-Duplicate", "Duplicate"),
        (320., "menu-row-Rename", "Rename"),
    ]
    .into_iter()
    .enumerate()
    {
        cx.simulate_resize(size(px(width), px(844.)));
        settle(cx);
        tap("files-actions", true, cx);
        let panel = cx.debug_bounds("menu-panel-0").unwrap();
        assert!(
            panel.left() >= px(0.)
                && panel.right() <= px(width)
                && panel.top() >= px(0.)
                && panel.bottom() <= px(844.)
        );
        tap(row, true, cx);
        assert!(cx.debug_bounds("menu-panel-0").is_none());
        view.read_with(cx, |view, cx| {
            assert_eq!(
                *view.actions.as_ref().unwrap().read(cx),
                (index + 1, SharedString::from(label))
            )
        });
    }
}
#[gpui::test]
fn files_favorites_remove_correct_keys_once_across_rerender(cx: &mut TestAppContext) {
    let (view, cx) = stage(cx, "Favorites / Bookmarks");
    tap("favorite-remove-atrium", true, cx);
    assert!(cx.debug_bounds("favorite-remove-atrium").is_none());
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.removed.as_ref().unwrap().read(cx),
            [SharedString::from("atrium")]
        );
        assert_eq!(
            view.favorites
                .as_ref()
                .unwrap()
                .read(cx)
                .iter()
                .map(|page| page.key.as_ref())
                .collect::<Vec<_>>(),
            ["roadmap", "music"]
        );
    });
    cx.simulate_resize(size(px(320.), px(844.)));
    settle(cx);
    tap("favorite-remove-roadmap", true, cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            *view.removed.as_ref().unwrap().read(cx),
            [SharedString::from("atrium"), SharedString::from("roadmap")]
        );
        assert_eq!(
            view.favorites
                .as_ref()
                .unwrap()
                .read(cx)
                .iter()
                .map(|page| page.key.as_ref())
                .collect::<Vec<_>>(),
            ["music"]
        );
    });
}
