use crate::{
    buttons::IconButton,
    primitives::IconName,
    shell::{ColumnShell, ShellColumn, TabBar, TitleBar, WindowTab, ZoomControl},
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
            cx.update(|window, _cx| window.rem_size()),
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

/// Tabs, and the ones closed.
struct Tabs {
    closed: Vec<String>,
}

impl Render for Tabs {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w_full().child(
            TabBar::new("tabs")
                .tab(WindowTab::new("one", "One"))
                .tab(WindowTab::new("two", "Two"))
                .selected("one")
                .on_close(move |id, _, cx| {
                    owner.update(cx, |state, _| state.closed.push(id.to_string()))
                }),
        )
    }
}

#[gpui::test]
fn an_unselected_tab_keeps_its_close_button_in_touch_mode(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
    });
    let (view, cx) = cx.add_window_view(|_, _| Tabs { closed: Vec::new() });
    settle(cx);
    let close = cx
        .debug_bounds("window-tab-close-touch two")
        .expect("the unselected tab keeps its close button");
    assert!(
        close.size.width >= px(48.) && close.size.height >= px(48.),
        "the close button is a touch target: {close:?}"
    );
    // The button is real: a tap on it closes that tab.
    let at = close.center();
    cx.simulate_mouse_down(at, gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, gpui::MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |state, _| state.closed.clone()), ["two"]);
    assert!(
        cx.debug_bounds("window-tab-close-touch two").is_some(),
        "the close button stays put"
    );
}

#[gpui::test]
fn an_unselected_tab_hides_its_close_button_on_the_desktop(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let (_, cx) = cx.add_window_view(|_, _| Tabs { closed: Vec::new() });
    settle(cx);
    assert!(
        cx.debug_bounds("window-tab-close-touch two").is_none(),
        "on the desktop the close button waits for a hover"
    );
}
