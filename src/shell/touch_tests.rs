use crate::{
    buttons::IconButton,
    primitives::IconName,
    shell::{ColumnShell, ShellColumn, TitleBar, ZoomControl},
    theme::{ActiveTheme, Platform, Theme},
};
use gpui::{
    Context, Entity, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, prelude::*, px, size,
};

struct Headers {
    platform: Option<Platform>,
    columns: bool,
    leading: usize,
    actions: usize,
    closes: usize,
}

impl Render for Headers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (lead, action, close) = (cx.entity(), cx.entity(), cx.entity());
        let leading = IconButton::new("lead", IconName::ArrowLeft)
            .tooltip("Previous document")
            .debug_selector(|| "touch-leading".into())
            .on_click(move |_, _, cx| lead.update(cx, |state, _| state.leading += 1));
        let action = IconButton::new("action", IconName::Info)
            .tooltip("Document information")
            .debug_selector(|| "touch-action".into())
            .on_click(move |_, _, cx| action.update(cx, |state, _| state.actions += 1));
        let title = "A long document title that must yield to the real actions";
        let content = if self.columns {
            ColumnShell::new("shell")
                .when_some(self.platform, |shell, platform| shell.platform(platform))
                .on_close(move |_, cx| close.update(cx, |state, _| state.closes += 1))
                .column(
                    ShellColumn::new("main")
                        .title(title)
                        .leading(leading)
                        .action(action)
                        .child(
                            div()
                                .debug_selector(|| "touch-body".into())
                                .child("Document"),
                        ),
                )
                .into_any_element()
        } else {
            TitleBar::new("bar")
                .when_some(self.platform, |bar, platform| bar.platform(platform))
                .title(title)
                .child(leading)
                .action(action)
                .on_close(move |_, cx| close.update(cx, |state, _| state.closes += 1))
                .into_any_element()
        };
        div().w_full().h(px(240.)).flex().flex_col().child(content)
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn tap(selector: &'static str, cx: &mut VisualTestContext) {
    let hit = cx.debug_bounds(selector).expect(selector);
    let viewport = cx.update(|window, _| window.viewport_size());
    assert!(
        hit.size.width >= px(48.) && hit.size.height >= px(48.),
        "{hit:?}"
    );
    assert!(hit.left() >= px(0.) && hit.top() >= px(0.));
    assert!(hit.right() <= viewport.width && hit.bottom() <= viewport.height);
    cx.simulate_mouse_down(hit.center(), gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(hit.center(), gpui::MouseButton::Left, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn touch_headers_preserve_actions_without_desktop_controls(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx): (Entity<Headers>, _) = cx.add_window_view(|_, _| Headers {
        platform: None,
        columns: false,
        leading: 0,
        actions: 0,
        closes: 0,
    });
    cx.simulate_resize(size(px(280.), px(400.)));
    for columns in [false, true] {
        for platform in [
            None,
            Some(Platform::Mac),
            Some(Platform::Windows),
            Some(Platform::Linux),
        ] {
            view.update(cx, |state, cx| {
                state.columns = columns;
                state.platform = platform;
                state.leading = 0;
                state.actions = 0;
                cx.notify();
            });
            settle(cx);
            assert!(cx.debug_bounds("titlebar-controls").is_none());
            assert!(cx.debug_bounds("column-shell-controls").is_none());
            assert!(cx.debug_bounds("caption-Close").is_none());
            tap("touch-leading", cx);
            tap("touch-action", cx);
            let header = cx
                .debug_bounds(if columns {
                    "column-shell-header-0"
                } else {
                    "titlebar-touch"
                })
                .unwrap();
            assert!(header.size.height >= px(48.));
            // Empty header presses must not become Back or a close request.
            let blank = gpui::point(header.left() + px(1.), header.center().y);
            cx.simulate_click(blank, Modifiers::none());
            cx.simulate_click(blank, Modifiers::none());
            cx.simulate_mouse_down(blank, gpui::MouseButton::Left, Modifiers::none());
            cx.simulate_mouse_move(
                blank + gpui::point(px(4.), px(0.)),
                gpui::MouseButton::Left,
                Modifiers::none(),
            );
            cx.simulate_mouse_up(blank, gpui::MouseButton::Left, Modifiers::none());
            settle(cx);
            view.read_with(cx, |state, _| {
                assert_eq!((state.leading, state.actions, state.closes), (1, 1, 0));
            });
            if columns {
                assert!(cx.debug_bounds("touch-body").is_some());
            }
        }
    }
    // Changing presentation must restore desktop controls, then suppress them again.
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    assert!(cx.debug_bounds("column-shell-controls").is_some());
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = true));
    settle(cx);
    assert!(cx.debug_bounds("column-shell-controls").is_none());
    // Existing header overflow must still expose the real trailing action.
    cx.simulate_resize(size(px(100.), px(400.)));
    settle(cx);
    let items = cx.debug_bounds("column-shell-items-0").unwrap();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: items.center(),
        delta: gpui::ScrollDelta::Pixels(gpui::point(px(-400.), px(0.))),
        ..Default::default()
    });
    settle(cx);
    tap("touch-action", cx);
    assert_eq!(view.read_with(cx, |state, _| state.actions), 2);
}

struct Zoom;
struct OwnedZoom {
    percent: u32,
    calls: Vec<crate::shell::ZoomStep>,
}
impl Render for OwnedZoom {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().flex().child(ZoomControl::new("owned-zoom").owned(
            self.percent,
            move |step, _, cx| {
                owner.update(cx, |state, _| state.calls.push(step));
            },
        ))
    }
}

#[gpui::test]
fn owned_zoom_targets_scale_with_rem_and_only_dispatch_owner_callbacks(cx: &mut TestAppContext) {
    use crate::shell::ZoomStep;
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (view, cx) = cx.add_window_view(|_, _| OwnedZoom {
        percent: 100,
        calls: Vec::new(),
    });
    cx.simulate_resize(size(px(390.), px(240.)));
    for percent in [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200] {
        view.update(cx, |state, cx| {
            state.percent = percent;
            state.calls.clear();
            cx.notify();
        });
        // Owned content zoom is independent of UI rem; exercise both together,
        // then assert callbacks never apply the default window-rem mutation.
        cx.update(|window, cx| {
            window.set_rem_size(cx.theme().base_rem() * (percent as f32 / 100.))
        });
        settle(cx);
        let before = cx.update(|window, _| window.rem_size());
        let reset = cx.debug_bounds("shell-owned-zoom-reset").unwrap();
        let out = cx.debug_bounds("shell-owned-zoom-out").unwrap();
        let into = cx.debug_bounds("shell-owned-zoom-in").unwrap();
        assert!(out.right() <= reset.left() && reset.right() <= into.left());
        for selector in [
            "shell-owned-zoom-out",
            "shell-owned-zoom-reset",
            "shell-owned-zoom-in",
        ] {
            tap(selector, cx);
            assert_eq!(cx.update(|window, _| window.rem_size()), before);
        }
        view.read_with(cx, |state, _| {
            assert_eq!(state.calls, [ZoomStep::Out, ZoomStep::Reset, ZoomStep::In]);
            assert_eq!(
                state.percent, percent,
                "owner retains control of its zoom state"
            );
        });
    }
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    let reset = cx.debug_bounds("shell-owned-zoom-reset").unwrap();
    let expected = cx.update(|window, cx| {
        cx.theme()
            .control_height(crate::theme::ControlSize::Sm)
            .to_pixels(window.rem_size())
    });
    assert_eq!(reset.size.height, expected);
}

impl Render for Zoom {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().flex().child(ZoomControl::new("zoom"))
    }
}

#[gpui::test]
fn zoom_reset_keeps_touch_minimum_across_scale_and_desktop_size(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.touch_mode = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Zoom);
    cx.simulate_resize(size(px(280.), px(200.)));
    for percent in [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200] {
        let scale = percent as f32 / 100.;
        cx.update(|window, cx| window.set_rem_size(cx.theme().base_rem() * scale));
        settle(cx);
        let reset = cx.debug_bounds("shell-zoom-reset").unwrap();
        let minus = cx.debug_bounds("shell-zoom-out").unwrap();
        let plus = cx.debug_bounds("shell-zoom-in").unwrap();
        assert!(minus.right() <= reset.left() && reset.right() <= plus.left());
        for hit in [minus, reset, plus] {
            assert!(
                hit.size.width >= px(48.) && hit.size.height >= px(48.),
                "{hit:?}"
            );
            assert!(hit.left() >= px(0.) && hit.right() <= px(280.));
            assert!(hit.top() >= px(0.) && hit.bottom() <= px(200.));
        }
        tap("shell-zoom-reset", cx);
        assert_eq!(
            cx.update(|window, cx| window.rem_size()),
            cx.update(|_, cx| cx.theme().base_rem())
        );
    }
    cx.update(|_, cx| Theme::update(cx, |theme| theme.touch_mode = false));
    settle(cx);
    let reset = cx.debug_bounds("shell-zoom-reset").unwrap();
    let expected = cx.update(|window, cx| {
        cx.theme()
            .control_height(crate::theme::ControlSize::Sm)
            .to_pixels(window.rem_size())
    });
    assert_eq!(reset.size.height, expected);
}
