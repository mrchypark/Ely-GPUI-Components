mod bars;
mod chrome;
mod columns;
#[cfg(any(test, target_os = "ios", target_os = "android"))]
mod mobile;
#[cfg(any(test, target_os = "ios", target_os = "android"))]
mod mobile_headers;
#[cfg(all(test, feature = "test-support", not(target_family = "wasm")))]
mod mobile_tests;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod status;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod system;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod windows;

use gpui::{AnyElement, App, IntoElement, ParentElement, Window, div};

use super::Page;
use crate::step::Step;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub use windows::{open_about, open_managed};

pub const PAGE: Page = Page {
    number: 4,
    slug: "shell",
    title: "Window & Shell",
    summary: "Title bars, rails, tabs, and the windows around an app.",
    render,
    script: SCRIPT,
};

const SCRIPT: &[Step] = &[
    Step::Rest,
    Step::ClickEnd("tab-bar"),
    Step::Wait(400),
    Step::Shot("tab-added"),
    Step::DownAt("activity", 24.0, 76.0),
    Step::UpAt("activity", 24.0, 76.0),
    Step::Wait(400),
    Step::Shot("activity-search"),
    Step::DownAt("activity", 24.0, 28.0),
    Step::UpAt("activity", 24.0, 28.0),
    Step::Click("offline-offline"),
    Step::Wait(400),
    Step::Shot("offline"),
    Step::Click("offline-reconnecting"),
    Step::Wait(400),
    Step::Shot("reconnecting"),
    Step::Click("offline-online"),
    Step::Wait(400),
    Step::Shot("back-online"),
    Step::ClickEnd("zoom"),
    Step::Wait(400),
    Step::Shot("zoomed"),
    Step::Click("zoom"),
    Step::Wait(300),
    Step::Click("about-open"),
    Step::Wait(700),
    Step::KeyWindow("about", "tab"),
    Step::Wait(200),
    Step::ShotWindow("about", "about-window"),
    Step::CloseWindow("about"),
    Step::Click("splash-open"),
    Step::Wait(700),
    Step::ShotWindow("splash", "splash-window"),
    Step::CloseWindow("splash"),
    Step::Click("update-open"),
    Step::Wait(700),
    Step::ShotWindow("update", "update-window"),
    Step::CloseWindow("update"),
    Step::Click("crash-open"),
    Step::Wait(700),
    Step::ShotWindow("crash", "crash-window"),
    Step::CloseWindow("crash"),
    Step::Click("managed-open"),
    Step::Wait(700),
    Step::ShotWindow("managed", "managed-window"),
    Step::CloseNative("managed"),
    Step::Wait(300),
    Step::ShotWindow("managed", "managed-asked"),
    Step::Click("switcher-open"),
    Step::Wait(400),
    Step::Key("tab"),
    Step::Wait(200),
    Step::Shot("switcher"),
    Step::Key("escape"),
    Step::Wait(300),
    Step::CloseNative("managed"),
    Step::Wait(400),
    Step::ExpectClosed("managed"),
    Step::Click("mini-open"),
    Step::Wait(700),
    Step::ShotWindow("mini", "mini-window"),
    Step::CloseWindow("mini"),
    Step::Click("pip-open"),
    Step::Wait(700),
    Step::ShotWindow("pip", "pip-window"),
    Step::CloseWindow("pip"),
    Step::Click("tray-show"),
    Step::Wait(300),
    Step::Click("tray-pick"),
    Step::Wait(400),
    Step::Shot("tray-picked"),
    Step::Click("tray-show"),
    Step::Click("badge-set"),
    Step::Wait(300),
    Step::Shot("badge"),
    Step::Click("badge-clear"),
    Step::Rest,
];

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn render(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> AnyElement {
    div()
        .child(chrome::title_bar(cx))
        .child(columns::column_shell(cx))
        .child(chrome::window_controls(cx))
        .child(chrome::drag_area(window, cx))
        .child(chrome::resize_border(cx))
        .child(chrome::menus(cx))
        .child(bars::toolbar(cx))
        .child(bars::status_bar(cx))
        .child(bars::activity_bar(window, cx))
        .child(bars::tab_bar(window, cx))
        .child(windows::splash_screen(cx))
        .child(windows::about_dialog(cx))
        .child(system::tray(window, cx))
        .child(system::dock(cx))
        .child(system::notification(cx))
        .child(windows::window_manager(cx))
        .child(windows::window_switcher(window, cx, bounds))
        .child(windows::quick_launcher(cx))
        .child(windows::mini_windows(cx))
        .child(status::update_banner(cx))
        .child(status::update_window(cx))
        .child(status::crash_reporter(cx))
        .child(status::offline(window, cx))
        .child(status::zoom(cx))
        .into_any_element()
}

#[cfg(any(target_os = "ios", target_os = "android"))]
fn render(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> AnyElement {
    mobile::render(window, cx, bounds)
}
