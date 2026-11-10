use ely_gpui_component::{
    buttons::Button,
    terminal::{Target, Terminal, links},
};
use gpui::{
    AnyElement, Context, Entity, IntoElement, ParentElement, SharedString, Styled, div, prelude::*,
    px,
};

use crate::ui::set;

const TARGETS: [&str; 2] = ["https://example.com/replay", "src/main.rs:47:9"];

pub(super) fn replay(shell: usize, cx: &mut Context<Terminal>) -> Terminal {
    let name = ["bash", "zsh", "sh"][shell];
    let output = format!(
        "\x1b[1;36mRecorded {name} output\x1b[0m\r\n\x1b[32mOK\x1b[0m ANSI rendering fixture\r\nNo local shell is running.\r\n{}\r\n{}\r\n",
        TARGETS[0], TARGETS[1]
    );
    Terminal::replay(output.as_bytes(), 40, 8, cx)
}

pub(super) fn scroll(id: &'static str, child: AnyElement) -> impl IntoElement {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .w_full()
        .min_w_0()
        .overflow_x_scroll()
        .child(
            div()
                .debug_selector(|| "terminal-replay-find-content".into())
                .w(px(440.))
                .child(child),
        )
}

pub(super) fn targets(notice: &Entity<SharedString>) -> impl IntoElement + use<> {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_2()
        .child("Recorded link targets — preview only. In-grid touch activation is not supported.")
        .children(TARGETS.into_iter().enumerate().map(|(ix, line)| {
            let (_, target) = links(line).into_iter().next().expect("fixture has a link");
            let label = match &target {
                Target::Url(_) => "Preview URL target",
                Target::Path { .. } => "Preview path target",
            };
            let notice = notice.clone();
            div()
                .debug_selector(move || format!("terminal-replay-target-{ix}"))
                .child(
                    Button::new(("terminal-replay-target", ix), label)
                        .full_width()
                        .on_click(move |_, _, cx| {
                            let message = match &target {
                                Target::Url(url) => format!("Recorded URL target: {url}"),
                                Target::Path { path, line, column } => {
                                    let mut message = format!("Recorded path target: {path}");
                                    if let Some(line) = line {
                                        message.push_str(&format!(", line {line}"));
                                    }
                                    if let Some(column) = column {
                                        message.push_str(&format!(", column {column}"));
                                    }
                                    message
                                }
                            };
                            set(&notice, message.into(), cx);
                        }),
                )
        }))
}
