use ely_gpui_component::shell::{Connectivity, UpdateState};
use gpui::{
    Context, Entity, IntoElement, KeyUpEvent, Keystroke, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, px, size,
};

use super::mobile::{self, Fixture};

struct MobileStory {
    title: SharedString,
    state_id: &'static str,
    state: Option<Entity<Fixture>>,
    missing: Option<String>,
}
impl Render for MobileStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        cx.set_global(crate::ui::Story::new(self.title.clone()));
        self.state = Some(mobile::fixture(window, cx, self.state_id));
        let bounds = gpui::Bounds::new(gpui::point(px(0.), px(0.)), window.viewport_size());
        let body = mobile::render(window, cx, Some(bounds));
        self.missing = cx.remove_global::<crate::ui::Story>().missing("shell");
        div().w_full().child(body)
    }
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
        assert!(cx.try_global::<crate::ui::Story>().is_none());
    });
}
fn setup<'a>(
    cx: &'a mut TestAppContext,
    title: &'static str,
    state_id: &'static str,
) -> (Entity<MobileStory>, &'a mut VisualTestContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        use ely_gpui_component::theme::{Mode, Theme};
        Theme::update(cx, |theme| {
            theme.touch_mode = true;
            theme.reduced_motion = true;
        });
        Theme::set_mode_now(Mode::Light, cx);
        cx.on_action(|_: &crate::menu_model::LightMode, cx| Theme::set_mode_now(Mode::Light, cx));
        cx.on_action(|_: &crate::menu_model::DarkMode, cx| Theme::set_mode_now(Mode::Dark, cx));
    });
    let (view, cx) = cx.add_window_view(|_, _| MobileStory {
        title: title.into(),
        state_id,
        state: None,
        missing: None,
    });
    cx.simulate_resize(size(px(390.), px(844.)));
    draw(cx);
    (view, cx)
}
fn snapshot(view: &Entity<MobileStory>, cx: &VisualTestContext) -> Fixture {
    view.read_with(cx, |view, cx| view.state.as_ref().unwrap().read(cx).clone())
}
fn tap(selector: &'static str, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds(selector).expect(selector);
    assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
    cx.simulate_click(bounds.center(), gpui::Modifiers::none());
    draw(cx);
}
fn activate_first(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.blur(cx);
        window.focus_next(cx);
    });
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    draw(cx);
}

#[gpui::test]
fn mobile_shell_resolves_all_24_catalog_stories_without_claiming_os_support(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, "SplashScreen", "mobile-splash");
    let catalog: serde_json::Value =
        serde_json::from_str(include_str!("../../stories.json")).unwrap();
    let all = catalog["pages"].as_array().unwrap();
    assert_eq!(
        all.iter()
            .map(|page| page["stories"].as_array().unwrap().len())
            .sum::<usize>(),
        624
    );
    let stories = all.iter().find(|page| page["slug"] == "shell").unwrap()["stories"]
        .as_array()
        .unwrap();
    assert_eq!(stories.len(), 24);
    let mut unavailable = 0;
    for story in stories {
        view.update(cx, |view, cx| {
            view.title = story["title"].as_str().unwrap().to_owned().into();
            cx.notify();
        });
        draw(cx);
        view.read_with(cx, |view, _| {
            assert!(view.missing.is_none(), "{:?}", view.missing)
        });
        let marker = match story["slug"].as_str().unwrap() {
            "titlebar" | "columnshell-shellcolumn" => Some("shell-touch-header-specimen"),
            "splashscreen" => Some("shell-splash"),
            "aboutdialog" => Some("shell-about"),
            "updatebanner" => Some("shell-update-banner"),
            "updatedialog" => Some("shell-update-dialog"),
            "crashreporter" => Some("shell-crash"),
            "offlineindicator" => Some("shell-connectivity"),
            "zoomcontrol" => Some("shell-zoom"),
            "nativemenu-menubar-jumplist" => Some("shell-menubar"),
            _ => None,
        };
        if let Some(marker) = marker {
            let bounds = cx.debug_bounds(marker).expect(marker);
            // Online intentionally hides the OfflineIndicator until its state changes.
            if marker != "shell-connectivity" {
                assert!(
                    bounds.size.width > px(0.) && bounds.size.height > px(0.),
                    "{marker}"
                );
            }
            assert!(
                bounds.top() >= px(0.) && bounds.top() < px(844.),
                "{marker}"
            );
        }
        unavailable += usize::from(cx.debug_bounds("shell-capability-unavailable").is_some());
    }
    assert_eq!(
        unavailable, 10,
        "Capability notices are not actual-component passes"
    );
}

#[gpui::test]
fn mobile_update_banner_uses_real_download_restart_and_dismiss_actions(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "UpdateBanner", "mobile-update-banner");
    activate_first(cx);
    assert_eq!(snapshot(&view, cx).update, UpdateState::Downloading(0.42));
    tap("shell-banner-complete", cx);
    assert_eq!(snapshot(&view, cx).update, UpdateState::Ready);
    activate_first(cx);
    let state = snapshot(&view, cx);
    assert_eq!(state.update, UpdateState::Available);
    assert!(state.last.contains("not restarted"));
    tap("shell-update-banner-dismiss", cx);
    assert!(!snapshot(&view, cx).update_visible);
    assert!(cx.debug_bounds("shell-update-banner").is_none());
    tap("shell-banner-reset", cx);
    assert!(cx.debug_bounds("shell-update-banner").is_some());
}

#[gpui::test]
fn mobile_update_dialog_later_dismisses_inline_without_closing_window(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "UpdateDialog", "mobile-update-dialog");
    activate_first(cx);
    assert!(!snapshot(&view, cx).update_visible);
    assert!(cx.debug_bounds("shell-update-dialog").is_none());
    tap("shell-dialog-reset", cx);
    assert!(cx.debug_bounds("shell-update-dialog").is_some());
}

#[gpui::test]
fn mobile_splash_about_and_connectivity_controls_change_real_inline_content(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, "SplashScreen", "mobile-splash");
    tap("shell-splash-progress", cx);
    assert_eq!(snapshot(&view, cx).progress, None);
    tap("shell-splash-progress", cx);
    assert_eq!(snapshot(&view, cx).progress, Some(0.62));
    view.update(cx, |view, cx| {
        view.title = "AboutDialog".into();
        view.state_id = "mobile-about";
        cx.notify();
    });
    draw(cx);
    tap("shell-about-toggle", cx);
    assert!(!snapshot(&view, cx).about_visible);
    assert!(cx.debug_bounds("shell-about").is_none());
    tap("shell-about-toggle", cx);
    assert!(cx.debug_bounds("shell-about").is_some());
    view.update(cx, |view, cx| {
        view.title = "OfflineIndicator".into();
        view.state_id = "mobile-connectivity";
        cx.notify();
    });
    draw(cx);
    tap("shell-offline", cx);
    assert_eq!(snapshot(&view, cx).connectivity, Connectivity::Offline);
    tap("shell-reconnecting", cx);
    assert_eq!(snapshot(&view, cx).connectivity, Connectivity::Reconnecting);
    tap("shell-online", cx);
    assert_eq!(snapshot(&view, cx).connectivity, Connectivity::Online);
}

#[gpui::test]
fn mobile_crash_report_actions_record_local_results_and_reopen(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "CrashReporter", "mobile-crash");
    let activate = |count, cx: &mut VisualTestContext| {
        cx.update(|window, cx| {
            window.blur(cx);
            for _ in 0..count {
                window.focus_next(cx);
            }
        });
        cx.simulate_keystrokes("enter");
        cx.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").unwrap(),
        });
        draw(cx);
    };
    // Actual component order: details, copy, dismiss, send; reset follows it.
    activate(4, cx);
    let state = snapshot(&view, cx);
    assert!(state.report_sent && !state.report_visible);
    assert!(state.last.contains("nothing transmitted"));
    assert!(cx.debug_bounds("shell-crash").is_none());
    tap("shell-crash-reset", cx);
    assert!(cx.debug_bounds("shell-crash").is_some());
    activate(3, cx);
    let state = snapshot(&view, cx);
    assert!(!state.report_sent && !state.report_visible);
    assert!(state.last.contains("dismissed"));
}

#[gpui::test]
fn mobile_menu_dispatches_shared_actions_and_zoom_changes_only_in_app_scale(
    cx: &mut TestAppContext,
) {
    use ely_gpui_component::theme::ActiveTheme;
    let (view, cx) = setup(cx, "NativeMenu / MenuBar / JumpList", "mobile-menu-test");
    tap("menu-bar-title-0", cx);
    tap("menu-row-Dark", cx);
    assert!(cx.update(|_, cx| cx.theme().is_dark()));
    assert!(cx.debug_bounds("menu-panel-0").is_none());
    view.update(cx, |view, cx| {
        view.title = "ZoomControl".into();
        cx.notify();
    });
    draw(cx);
    let before = cx.update(|window, _| window.rem_size());
    activate_first(cx);
    assert!(cx.update(|window, _| window.rem_size()) < before);
    // The number is pointer-only; the next Tab stop is the real increase button.
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    draw(cx);
    assert_eq!(cx.update(|window, _| window.rem_size()), before);
}

#[gpui::test]
fn mobile_visible_update_crash_and_zoom_controls_accept_pointer_taps(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "UpdateBanner", "mobile-update-banner");
    let click = |selector: &'static str, host: &'static str, cx: &mut VisualTestContext| {
        let hit = cx.debug_bounds(selector).expect(selector);
        let component = cx.debug_bounds(host).expect(host);
        let viewport = cx.update(|window, _| {
            gpui::Bounds::new(gpui::point(px(0.), px(0.)), window.viewport_size())
        });
        assert!(
            hit.size.width > px(0.) && hit.size.height > px(0.),
            "{selector}: {hit:?}"
        );
        for area in [component, viewport] {
            assert!(
                hit.left() >= area.left()
                    && hit.right() <= area.right()
                    && hit.top() >= area.top()
                    && hit.bottom() <= area.bottom(),
                "{selector} is not fully reachable: {hit:?} outside {area:?}"
            );
        }
        eprintln!(
            "pointer target {selector}: {hit:?}, component {component:?}, viewport {viewport:?}"
        );
        if selector.starts_with("shell-zoom-") {
            let targets = ["shell-zoom-out", "shell-zoom-reset", "shell-zoom-in"]
                .map(|name| cx.debug_bounds(name).expect(name));
            for bounds in targets {
                assert!(
                    bounds.size.width >= px(48.) && bounds.size.height >= px(48.),
                    "{bounds:?}"
                );
            }
            assert!(targets[0].right() <= targets[1].left());
            assert!(targets[1].right() <= targets[2].left());
        }
        cx.simulate_click(hit.center(), gpui::Modifiers::none());
        draw(cx);
    };
    click("shell-update-banner-download", "shell-update-banner", cx);
    assert_eq!(snapshot(&view, cx).update, UpdateState::Downloading(0.42));
    tap("shell-banner-complete", cx);
    click("shell-update-banner-restart", "shell-update-banner", cx);
    assert_eq!(snapshot(&view, cx).update, UpdateState::Available);
    assert!(snapshot(&view, cx).last.contains("not restarted"));
    click("shell-update-banner-dismiss", "shell-update-banner", cx);
    assert!(!snapshot(&view, cx).update_visible);
    let select = |title: &'static str, id, cx: &mut VisualTestContext| {
        view.update(cx, |view, cx| {
            view.title = title.into();
            view.state_id = id;
            cx.notify();
        });
        draw(cx);
    };
    select("UpdateDialog", "mobile-update-dialog", cx);
    click("shell-update-dialog-download", "shell-update-dialog", cx);
    assert_eq!(snapshot(&view, cx).update, UpdateState::Downloading(0.42));
    tap("shell-dialog-complete", cx);
    click("shell-update-dialog-restart", "shell-update-dialog", cx);
    assert_eq!(snapshot(&view, cx).update, UpdateState::Available);
    assert!(snapshot(&view, cx).last.contains("not restarted"));
    click("shell-update-dialog-later", "shell-update-dialog", cx);
    assert!(!snapshot(&view, cx).update_visible);
    select("CrashReporter", "mobile-crash", cx);
    click("shell-crash-send", "shell-crash", cx);
    let state = snapshot(&view, cx);
    assert!(state.report_sent && !state.report_visible);
    assert!(state.last.contains("nothing transmitted"));
    assert!(cx.debug_bounds("shell-crash").is_none());
    tap("shell-crash-reset", cx);
    click("shell-crash-dismiss", "shell-crash", cx);
    let state = snapshot(&view, cx);
    assert!(!state.report_sent && !state.report_visible);
    assert!(state.last.contains("dismissed"));
    select("ZoomControl", "mobile-zoom-pointer", cx);
    let before = cx.update(|window, _| window.rem_size());
    click("shell-zoom-out", "shell-zoom", cx);
    assert!(cx.update(|window, _| window.rem_size()) < before);
    click("shell-zoom-reset", "shell-zoom", cx);
    assert_eq!(cx.update(|window, _| window.rem_size()), before);
    click("shell-zoom-in", "shell-zoom", cx);
    assert!(cx.update(|window, _| window.rem_size()) > before);
    click("shell-zoom-reset", "shell-zoom", cx);
    assert_eq!(cx.update(|window, _| window.rem_size()), before);
}

#[gpui::test]
fn mobile_real_headers_route_folders_details_and_message_actions_at_phone_width(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx, "TitleBar", "mobile-header-test");
    let hit = |selector: &'static str, cx: &mut VisualTestContext| {
        let target = cx.debug_bounds(selector).expect(selector);
        let frame = cx.debug_bounds("shell-touch-header-specimen").unwrap();
        let viewport = cx.update(|window, _| {
            gpui::Bounds::new(gpui::point(px(0.), px(0.)), window.viewport_size())
        });
        assert!(
            target.size.width >= px(48.) && target.size.height >= px(48.),
            "{selector}: {target:?}"
        );
        for area in [frame, viewport] {
            assert!(
                target.left() >= area.left()
                    && target.right() <= area.right()
                    && target.top() >= area.top()
                    && target.bottom() <= area.bottom(),
                "{selector}: {target:?} outside {area:?}"
            );
        }
        target
    };
    for title in ["TitleBar", "ColumnShell / ShellColumn"] {
        view.update(cx, |view, cx| {
            view.title = title.into();
            cx.notify();
        });
        for width in [390., 320.] {
            cx.simulate_resize(size(px(width), px(844.)));
            draw(cx);
            assert!(cx.debug_bounds("titlebar-controls").is_none());
            assert!(cx.debug_bounds("column-shell-controls").is_none());
            assert!(cx.debug_bounds("shell-header-message").is_some());
            let leading = hit("shell-header-leading", cx);
            let action = hit("shell-header-action", cx);
            assert!(
                leading.right() <= action.left()
                    || action.right() <= leading.left()
                    || leading.bottom() <= action.top()
                    || action.bottom() <= leading.top()
            );
            cx.simulate_click(leading.center(), gpui::Modifiers::none());
            draw(cx);
            assert!(cx.debug_bounds("shell-header-empty").is_some());
            assert!(cx.debug_bounds("shell-header-message").is_none());
            tap("shell-header-leading", cx);
            assert!(cx.debug_bounds("shell-header-message").is_some());
            tap("shell-header-action", cx);
            assert!(cx.debug_bounds("shell-header-details").is_some());
            let body = hit("shell-header-body-action", cx);
            cx.simulate_click(body.center(), gpui::Modifiers::none());
            draw(cx);
            assert!(cx.debug_bounds("shell-header-empty").is_some());
            tap("shell-header-leading", cx);
            assert!(cx.debug_bounds("shell-header-message").is_some());
            let body = hit("shell-header-body-action", cx);
            cx.simulate_click(body.center(), gpui::Modifiers::none());
            draw(cx);
            assert!(cx.debug_bounds("shell-header-empty").is_some());
            tap("shell-header-leading", cx);
            tap("shell-header-action", cx);
            assert!(cx.debug_bounds("shell-header-message").is_some());
            assert!(cx.debug_bounds("shell-header-details").is_none());
            assert!(cx.debug_bounds("shell-header-close-requested").is_none());
        }
    }
}
