mod blocks;
#[cfg(not(target_family = "wasm"))]
mod live;
#[cfg(not(target_family = "wasm"))]
mod mobile;
mod panels;
#[cfg(all(test, feature = "test-support", not(target_family = "wasm")))]
mod tests;

use gpui::{AnyElement, App, IntoElement, ParentElement, Window, div};

use super::Page;
use crate::{step::Step, ui::section};

pub const PAGE: Page = Page {
    number: 18,
    slug: "terminal",
    title: "Terminal",
    summary: "A desktop shell or mobile output replay drawn cell by cell; command blocks and history; colored output, logs and processes.",
    render,
    script: SCRIPT,
};

#[cfg(any(target_os = "ios", target_os = "android"))]
const SCRIPT: &[Step] = &[];

#[cfg(not(any(target_os = "ios", target_os = "android")))]
const SCRIPT: &[Step] = &[
    Step::Rest,
    Step::DownAt("terminal-tabs", 46.0, 16.0),
    Step::UpAt("terminal-tabs", 46.0, 16.0),
    Step::Wait(300),
    Step::DownAt("terminal", 200.0, 150.0),
    Step::UpAt("terminal", 200.0, 150.0),
    Step::Wait(400),
    Step::Type("ls -G src"),
    Step::Key("enter"),
    Step::Wait(800),
    Step::Shot("shell"),
    Step::Key("ctrl-l"),
    Step::DownAt("terminal-tabs", 155.0, 16.0),
    Step::UpAt("terminal-tabs", 155.0, 16.0),
    Step::Wait(400),
    Step::Shot("find"),
    Step::Rest,
];

/// A desktop shell, a mobile replay, or the header alone on the web.
fn live(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    #[cfg(any(target_os = "ios", target_os = "android"))]
    let description = "Recorded ANSI output with tabs, splits, toolbar and search. No local shell runs. Link targets have separate preview buttons; in-grid touch activation is not supported.";
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    let description = "A real shell on a pseudo-terminal, drawn cell by cell in the theme's sixteen colors. Tabs hold shells and output; split for another shell beside it. Find lights every match through the scrollback. Cmd-press a link or a path:line.";
    let section = section(
        "Terminal / TerminalTabs / TerminalSplit / TerminalToolbar / ShellSelector / TerminalSearch / TerminalLink",
        description,
        cx,
    );
    #[cfg(not(target_family = "wasm"))]
    return if crate::ui::live(
        "Terminal / TerminalTabs / TerminalSplit / TerminalToolbar / ShellSelector / TerminalSearch / TerminalLink",
        cx,
    ) {
        section.child(live::shell(window, cx))
    } else {
        section
    };
    #[cfg(target_family = "wasm")]
    {
        let _ = window;
        section
    }
}

fn render(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> AnyElement {
    div()
        .child(live(window, cx))
        .child(blocks::blocks(window, cx, bounds))
        .child(blocks::ansi(cx))
        .child(panels::logs(window, cx))
        .child(panels::process_list(window, cx))
        .into_any_element()
}
