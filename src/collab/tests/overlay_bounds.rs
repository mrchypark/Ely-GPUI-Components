use crate::{
    chat::{CitationBadge, Source},
    collab::{Comment, CommentMarker, CommentSidebar, Peer, PresenceAvatars, Reactions, Thread},
    documents::{Footnote, PageCover, PageIcon, TooltipTerm},
    onboarding::FeedbackWidget,
    primitives::FocusScope,
    theme::Theme,
};
use gpui::{
    Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers,
    ParentElement, Pixels, Render, Styled, TestAppContext, VisualTestContext, Window, div, point,
    px, size,
};

#[derive(Clone, Copy)]
enum Part {
    Feedback,
    Reactions,
    Sidebar,
    Marker,
    Footnote,
    Term,
    Citation,
    Presence,
    Cover,
    Icon,
}

struct BoundsHost {
    root: FocusHandle,
    before: FocusHandle,
    area: Bounds<Pixels>,
    part: Part,
    primary: usize,
}

fn thread() -> Thread {
    Thread {
        key: "a".into(),
        quote: None,
        resolved: false,
        comments: vec![Comment {
            author: Peer::new("ada", "Ada", 0),
            at: jiff::Timestamp::UNIX_EPOCH,
            body: "Comment".into(),
            reactions: Vec::new(),
        }],
    }
}

impl Render for BoundsHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let part = match self.part {
            Part::Cover => div()
                .w(px(320.))
                .child(
                    PageCover::new("same-id", "/ely-overlay-fixture-cover.png")
                        .choices(["/ely-overlay-fixture-choice.png"], move |_, _, cx| {
                            owner.update(cx, |host, _| host.primary += 1)
                        })
                        .usable_bounds(self.area),
                )
                .into_any_element(),
            Part::Icon => PageIcon::new("same-id", "A")
                .on_change(move |_, _, cx| owner.update(cx, |host, _| host.primary += 1))
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Feedback => FeedbackWidget::new("same-id", |_, _, _, _| {})
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Reactions => Reactions::new("same-id", [])
                .on_toggle(|_, _, _| {})
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Sidebar => CommentSidebar::new("same-id", [thread()])
                .on_react(|_, _, _, _, _| {})
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Marker => CommentMarker::new("same-id", thread())
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Footnote => Footnote::new("same-id", 1, "A long explanatory footnote.")
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Term => TooltipTerm::new("same-id", "Term", "A definition.")
                .usable_bounds(self.area)
                .into_any_element(),
            Part::Citation => CitationBadge::new(
                "same-id",
                1,
                Source {
                    site: "Fixture".into(),
                    title: "Source".into(),
                    url: "https://example.invalid".into(),
                    snippet: Some("Preview".into()),
                },
            )
            .on_open(move |_, cx| owner.update(cx, |host, _| host.primary += 1))
            .usable_bounds(self.area)
            .into_any_element(),
            Part::Presence => PresenceAvatars::new(
                "same-id",
                [Peer::new("ada", "Ada", 0), Peer::new("grace", "Grace", 1)],
            )
            .follow(None, move |_, _, cx| {
                owner.update(cx, |host, _| host.primary += 1)
            })
            .usable_bounds(self.area)
            .into_any_element(),
        };
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(div().id("before").track_focus(&self.before))
            .child(
                div()
                    .w(px(320.))
                    .h(px(400.))
                    .flex()
                    .flex_col()
                    .items_start()
                    .child(part),
            )
    }
}

fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    cx.run_until_parked();
}

fn fixture(
    cx: &mut TestAppContext,
    part: Part,
    area: Bounds<Pixels>,
) -> (Entity<BoundsHost>, VisualTestContext) {
    let (view, visual) = cx.add_window_view(|_, cx| BoundsHost {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        area,
        part,
        primary: 0,
    });
    visual.simulate_resize(size(px(390.), px(844.)));
    settle(visual);
    visual.update(|window, cx| {
        let before = view.read(cx).before.clone();
        window.focus(&before, cx);
    });
    let handle = visual.update(|window, _| window.window_handle());
    (view, VisualTestContext::from_window(handle, cx))
}

fn init(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
}

fn key(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_keystrokes(key);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
    });
    settle(cx);
}

fn open(cx: &mut VisualTestContext, part: Part) {
    if matches!(
        part,
        Part::Footnote | Part::Term | Part::Citation | Part::Presence
    ) {
        let trigger = cx.debug_bounds("hover-card-disclosure").unwrap();
        cx.simulate_click(trigger.center(), Modifiers::none());
        settle(cx);
    } else {
        cx.update(|window, cx| {
            window.blur(cx);
            for _ in 0..if matches!(part, Part::Sidebar) { 2 } else { 1 } {
                window.focus_next(cx);
            }
        });
        settle(cx);
        key(cx, "enter");
    }
}

fn contained(cx: &mut VisualTestContext, area: Bounds<Pixels>) {
    let panel = cx
        .debug_bounds("popover-panel")
        .expect("wrapper opened a panel");
    assert!(
        panel.left() >= area.left() && panel.top() >= area.top(),
        "{panel:?}"
    );
    assert!(
        panel.right() <= area.right() && panel.bottom() <= area.bottom(),
        "{panel:?}"
    );
}

fn change(view: &Entity<BoundsHost>, cx: &mut VisualTestContext, area: Bounds<Pixels>) {
    view.update(cx, |host, cx| {
        host.area = area;
        cx.notify();
    });
    settle(cx);
}

#[gpui::test]
fn compound_wrappers_refit_after_keyboard_shrink_and_reopen_after_empty(cx: &mut TestAppContext) {
    init(cx);
    let normal = Bounds::new(point(px(20.), px(30.)), size(px(340.), px(450.)));
    let keyboard = Bounds::new(point(px(45.), px(65.)), size(px(230.), px(120.)));
    for part in [
        Part::Feedback,
        Part::Reactions,
        Part::Sidebar,
        Part::Marker,
        Part::Footnote,
        Part::Term,
        Part::Citation,
        Part::Presence,
        Part::Cover,
        Part::Icon,
    ] {
        let (view, mut cx) = fixture(cx, part, normal);
        open(&mut cx, part);
        contained(&mut cx, normal);
        change(&view, &mut cx, keyboard);
        contained(&mut cx, keyboard);
        for extent in [
            size(px(0.), px(120.)),
            size(px(230.), px(0.)),
            size(px(1.), px(1.)),
        ] {
            change(&view, &mut cx, Bounds::new(keyboard.origin, extent));
            assert!(cx.debug_bounds("popover-panel").is_none());
            open(&mut cx, part);
            assert!(
                cx.debug_bounds("popover-panel").is_none(),
                "empty is not unrestricted"
            );
            change(&view, &mut cx, keyboard);
            assert!(
                cx.debug_bounds("popover-panel").is_none(),
                "restore requires explicit open"
            );
            open(&mut cx, part);
            contained(&mut cx, keyboard);
        }
        key(&mut cx, "escape");
        assert!(cx.debug_bounds("popover-panel").is_none());
        assert_eq!(view.read_with(&cx, |host, _| host.primary), 0);
    }
}

#[gpui::test]
fn same_wrapper_ids_in_two_windows_keep_independent_live_bounds(cx: &mut TestAppContext) {
    init(cx);
    let first = Bounds::new(point(px(15.), px(25.)), size(px(180.), px(160.)));
    let second = Bounds::new(point(px(130.), px(240.)), size(px(240.), px(280.)));
    let (a, mut acx) = fixture(cx, Part::Feedback, first);
    let (_, mut bcx) = fixture(cx, Part::Feedback, second);
    open(&mut acx, Part::Feedback);
    open(&mut bcx, Part::Feedback);
    contained(&mut acx, first);
    contained(&mut bcx, second);
    change(
        &a,
        &mut acx,
        Bounds::new(first.origin, size(px(0.), px(0.))),
    );
    assert!(acx.debug_bounds("popover-panel").is_none());
    settle(&mut bcx);
    contained(&mut bcx, second);
    change(&a, &mut acx, first);
    open(&mut acx, Part::Feedback);
    contained(&mut acx, first);
    contained(&mut bcx, second);
}

#[gpui::test]
fn contextual_disclosure_keeps_citation_and_presence_primary_actions(cx: &mut TestAppContext) {
    init(cx);
    let area = Bounds::new(point(px(20.), px(30.)), size(px(340.), px(450.)));
    for part in [Part::Citation, Part::Presence] {
        let (view, mut cx) = fixture(cx, part, area);
        let selector = if matches!(part, Part::Citation) {
            "citation-primary"
        } else {
            "presence-primary"
        };
        let primary = cx.debug_bounds(selector).unwrap();
        let disclosure = cx.debug_bounds("hover-card-disclosure").unwrap();
        assert!(!primary.contains(&disclosure.center()));
        cx.simulate_click(primary.center(), Modifiers::none());
        settle(&mut cx);
        assert_eq!(view.read_with(&cx, |host, _| host.primary), 1);
        assert!(cx.debug_bounds("popover-panel").is_none());
        open(&mut cx, part);
        contained(&mut cx, area);
        assert_eq!(view.read_with(&cx, |host, _| host.primary), 1);
        key(&mut cx, "escape");
    }
}
