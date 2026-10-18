use ely_gpui_component::{
    buttons::Button,
    menus::MenuBar,
    shell::{
        AboutDialog, Connectivity, CrashReporter, OfflineIndicator, SplashScreen, UpdateBanner,
        UpdateDialog, UpdateState, ZoomControl,
    },
};
use gpui::{
    AnyElement, App, Bounds, Entity, IntoElement, ParentElement, Pixels, Styled, Window, div,
    prelude::*, px,
};

use crate::{probe::probe, ui::section};

#[derive(Clone)]
pub(super) struct Fixture {
    pub update: UpdateState,
    pub update_visible: bool,
    pub report_visible: bool,
    pub report_sent: bool,
    pub progress: Option<f32>,
    pub about_visible: bool,
    pub connectivity: Connectivity,
    pub last: &'static str,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            update: UpdateState::Available,
            update_visible: true,
            report_visible: true,
            report_sent: false,
            progress: Some(0.62),
            about_visible: true,
            connectivity: Connectivity::Online,
            last: "Local fixture; no system action performed.",
        }
    }
}

pub(super) fn fixture(window: &mut Window, cx: &mut App, id: &'static str) -> Entity<Fixture> {
    window.use_keyed_state(id, cx, |_, _| Fixture::default())
}

fn change(
    state: &Entity<Fixture>,
    edit: fn(&mut Fixture),
) -> impl Fn(&mut Window, &mut App) + use<> {
    let state = state.clone();
    move |_, cx| {
        state.update(cx, |state, cx| {
            edit(state);
            cx.notify();
        })
    }
}

fn control(
    id: &'static str,
    label: &'static str,
    state: &Entity<Fixture>,
    edit: fn(&mut Fixture),
) -> impl IntoElement + use<> {
    let action = change(state, edit);
    probe(
        id,
        div()
            .debug_selector(move || id.into())
            .child(Button::new(id, label).on_click(move |_, window, cx| action(window, cx))),
    )
}

fn reset(state: &Entity<Fixture>, id: &'static str) -> impl IntoElement + use<> {
    control(id, "Reset fixture", state, |state| {
        *state = Fixture::default()
    })
}

fn download(state: &mut Fixture) {
    state.update = UpdateState::Downloading(0.42);
    state.last = "Simulated download: 42%; no network request.";
}
fn restart(state: &mut Fixture) {
    state.update = UpdateState::Available;
    state.last = "Restart requested in fixture; app was not restarted.";
}
fn later(state: &mut Fixture) {
    state.update_visible = false;
    state.last = "Update dismissed locally.";
}

fn updates(window: &mut Window, cx: &mut App, dialog: bool) -> impl IntoElement + use<> {
    let (title, id, marker, complete, reset_id) = if dialog {
        (
            "UpdateDialog",
            "mobile-update-dialog",
            "shell-update-dialog",
            "shell-dialog-complete",
            "shell-dialog-reset",
        )
    } else {
        (
            "UpdateBanner",
            "mobile-update-banner",
            "shell-update-banner",
            "shell-banner-complete",
            "shell-banner-reset",
        )
    };
    let state = fixture(window, cx, id);
    let value = state.read(cx).clone();
    let content = if dialog {
        div()
            .h(px(340.))
            .flex()
            .flex_col()
            .child(
                UpdateDialog::new("Ely", "0.1.0", "0.2.0", value.update)
                    .note("A local fixture for download, progress, restart and dismissal.")
                    .on_download(change(&state, download))
                    .on_restart(change(&state, restart))
                    .on_later(change(&state, later)),
            )
            .into_any_element()
    } else {
        UpdateBanner::new(id, "0.2.0", value.update)
            .on_download(change(&state, download))
            .on_restart(change(&state, restart))
            .on_later(change(&state, later))
            .into_any_element()
    };
    section(title, "Real component, inline. All updater actions are local fixtures; no install, download or OS restart.", cx)
        .children(value.update_visible.then(|| div().debug_selector(move || marker.into()).child(content)))
        .children(matches!(value.update, UpdateState::Downloading(_)).then(|| control(complete, "Complete simulated download", &state, |state| {
            state.update = UpdateState::Ready;
            state.last = "Fixture ready; no update was installed.";
        })))
        .child(div().debug_selector(|| "shell-update-result".into()).child(value.last))
        .child(reset(&state, reset_id))
}

fn splash(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let state = fixture(window, cx, "mobile-splash");
    let progress = state.read(cx).progress;
    section(
        "SplashScreen",
        "Real inline splash; no additional native window is opened.",
        cx,
    )
    .child(
        div()
            .h(px(260.))
            .debug_selector(|| "shell-splash".into())
            .child(
                SplashScreen::new("Ely")
                    .tagline("Mobile gallery fixture")
                    .status("Loading workspace…")
                    .progress(progress),
            ),
    )
    .child(control(
        "shell-splash-progress",
        "Toggle determinate progress",
        &state,
        |state| {
            state.progress = if state.progress.is_some() {
                None
            } else {
                Some(0.62)
            };
        },
    ))
}

fn about(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let state = fixture(window, cx, "mobile-about");
    let shown = state.read(cx).about_visible;
    section(
        "AboutDialog",
        "Real inline About content with copyable version and external links; no desktop window.",
        cx,
    )
    .children(shown.then(|| {
        div().debug_selector(|| "shell-about".into()).child(
            AboutDialog::new("Ely", "0.1.0 (gallery fixture)")
                .description("A component library for GPUI.")
                .link(
                    "Website",
                    "https://github.com/ZacharyZhang-NY/Ely-GPUI-Components",
                )
                .link(
                    "License",
                    "https://github.com/ZacharyZhang-NY/Ely-GPUI-Components#license",
                )
                .copyright("© 2026 Ely contributors"),
        )
    }))
    .child(control(
        "shell-about-toggle",
        if shown {
            "Dismiss inline About"
        } else {
            "Show inline About"
        },
        &state,
        |state| state.about_visible = !state.about_visible,
    ))
}

fn crash(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let state = fixture(window, cx, "mobile-crash");
    let value = state.read(cx).clone();
    section(
        "CrashReporter",
        "Real report UI. Send records a local result only; no report leaves this app.",
        cx,
    )
    .children(value.report_visible.then(|| {
        div()
            .h(px(390.))
            .flex()
            .flex_col()
            .debug_selector(|| "shell-crash".into())
            .child(
                CrashReporter::new(
                    "Ely",
                    "Example diagnostic report; no crash occurred.",
                    "Fixture report\nThread: main\nExample: gallery rendering",
                )
                .on_send(change(&state, |state| {
                    state.report_sent = true;
                    state.report_visible = false;
                    state.last = "Report accepted by local fixture; nothing transmitted.";
                }))
                .on_dismiss(change(&state, |state| {
                    state.report_visible = false;
                    state.last = "Report dismissed; nothing transmitted.";
                })),
            )
    }))
    .child(
        div()
            .debug_selector(|| "shell-crash-result".into())
            .child(value.last),
    )
    .child(reset(&state, "shell-crash-reset"))
}

fn offline(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let state = fixture(window, cx, "mobile-connectivity");
    let current = state.read(cx).connectivity;
    section(
        "OfflineIndicator",
        "Real connectivity UI driven by fixture choices, not a network monitor.",
        cx,
    )
    .child(
        div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(control("shell-online", "Online", &state, |state| {
                state.connectivity = Connectivity::Online
            }))
            .child(control("shell-offline", "Offline", &state, |state| {
                state.connectivity = Connectivity::Offline
            }))
            .child(control(
                "shell-reconnecting",
                "Reconnecting",
                &state,
                |state| state.connectivity = Connectivity::Reconnecting,
            )),
    )
    .child(
        div()
            .debug_selector(|| "shell-connectivity".into())
            .child(OfflineIndicator::new("mobile-connectivity-pill", current)),
    )
}

const UNAVAILABLE: &[(&str, &str)] = &[
    (
        "WindowControls / TrafficLights",
        "Desktop close/minimize/maximize controls act on a host window. Only close is overridable; no fake component pass is claimed.",
    ),
    (
        "WindowDragRegion",
        "Native desktop window movement is unavailable here. In-app scrolling and touch delivery are separate tests.",
    ),
    (
        "WindowResizeBorder",
        "A phone surface is sized by its host, not desktop resize borders. WindowResizeBorder has no supported mobile resize interaction.",
    ),
    (
        "TrayIcon / TrayMenu",
        "The current TrayIcon API requires macOS. Mobile has no desktop tray; no tray icon or menu is installed.",
    ),
    (
        "DockBadge",
        "The current Dock badge API requires macOS. This is not an implementation of mobile app-icon badging.",
    ),
    (
        "SystemNotification",
        "System notification delivery requires a mobile host integration and permission flow. This gallery does not request permission or deliver notifications.",
    ),
    (
        "MultiWindow Manager",
        "WindowManager opens native desktop windows. This single-surface gallery does not claim multiwindow support.",
    ),
    (
        "WindowSwitcher",
        "WindowSwitcher selects real windows registered with WindowManager. This single-surface gallery has no supported registered-window workflow; fake entries would not verify that contract.",
    ),
    (
        "QuickLauncher",
        "QuickLauncher opens an OS popup window. The real inline SearchPalette is a separate component, not proof of QuickLauncher support on this surface.",
    ),
    (
        "MiniWindow / CompactMode / PictureInPicture",
        "MiniWindow uses native popup windows. OS picture-in-picture and always-on-top behavior are not implemented by an inline picture or player.",
    ),
];

pub(super) fn render(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<Bounds<Pixels>>,
) -> AnyElement {
    let menus = vec![crate::menu_model::view_menu().owned()];
    div()
        .child(super::mobile_headers::render(window, cx, false))
        .child(super::mobile_headers::render(window, cx, true))
        .child(super::bars::toolbar(cx))
        .child(super::bars::status_bar(cx))
        .child(super::bars::activity_bar(window, cx))
        .child(super::bars::tab_bar(window, cx))
        .child(splash(window, cx))
        .child(about(window, cx))
        .child(updates(window, cx, false))
        .child(updates(window, cx, true))
        .child(crash(window, cx))
        .child(offline(window, cx))
        .child(section("ZoomControl", "Real in-app scale controls: decrease, increase and reset. No OS magnification is changed.", cx)
            .child(probe("zoom", div().debug_selector(|| "shell-zoom".into()).child(ZoomControl::new("mobile-zoom")))))
        .child(section("NativeMenu / MenuBar / JumpList", "Real in-app MenuBar using the shared View menu. Light/Dark act on the gallery; no OS menu bar or JumpList is installed.", cx)
            .child(probe("shell-menubar", div().debug_selector(|| "shell-menubar".into()).child(
                MenuBar::new("mobile-shell-menu", menus).when_some(bounds, |bar, area| bar.usable_bounds(area)),
            ))))
        .children(UNAVAILABLE.iter().map(|&(title, reason)| section(title, reason, cx)
            .child(div().debug_selector(|| "shell-capability-unavailable".into()).child("Unsupported host capability — not a component verification pass."))))
        .into_any_element()
}
