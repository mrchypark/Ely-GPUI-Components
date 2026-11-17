use ely_gpui_component::{
    forms::{
        ExpressionInput, HotkeyInput, KeyValueInput, ListInput, MentionInput, PathInput,
        RegexInput, TagInput, expression_highlights, mention_highlights, regex_highlights,
    },
    theme::ActiveTheme,
    typography::Caption,
};
use gpui::prelude::FluentBuilder as _;
use gpui::{App, IntoElement, Keystroke, ParentElement, SharedString, Styled, Window, div, px};

use super::text::field;
use crate::{
    probe::{probe, probe_sized},
    ui::{code, section},
};

pub fn mention(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let message = field("input-mention", window, cx, |input| {
        input
            .multi_line(2, 5)
            .placeholder("Write a note. @ for people, # for tags.")
            .highlighter(mention_highlights)
    });
    section(
        "MentionInput / HashtagInput",
        "@ and # open suggestions at the caret. Arrows choose, Enter picks.",
        cx,
    )
    .child(probe(
        "mention",
        div().w(px(420.0)).child(
            MentionInput::new("mentions", &message)
                .trigger('@', ["ada", "alan", "grace", "linus"])
                .trigger('#', ["release", "research", "design", "docs"]),
        ),
    ))
}

pub fn tags(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let tags = window.use_keyed_state("tags", cx, |_, _| {
        vec![SharedString::from("rust"), SharedString::from("gpui")]
    });
    let now = tags.read(cx).clone();
    section(
        "TagInput / TokenInput",
        "Enter or a comma adds a chip; Backspace on an empty field drops the last.",
        cx,
    )
    .child(probe_sized(
        "tags",
        div()
            .w(px(420.0))
            .when(cx.theme().touch_mode, |view| view.w_full().min_w_0())
            .child(
                TagInput::new("topic-tags", now).on_change(move |next, _, cx| {
                    tags.update(cx, |tags, cx| {
                        *tags = next;
                        cx.notify();
                    })
                }),
            ),
        cx.theme().touch_mode,
    ))
}

pub fn rows(cx: &App) -> impl IntoElement + use<> {
    let touch = cx.theme().touch_mode;
    section(
        "KeyValueInput / ListInput",
        "Rows you add and remove. Each row reports as you type.",
        cx,
    )
    .child(
        div()
            .flex()
            .when(touch, |row| row.flex_col())
            .gap_8()
            .child(
                div()
                    .w(px(380.0))
                    .when(touch, |column| column.w_full().min_w_0())
                    .max_w_full()
                    .min_w_0()
                    .child(
                        KeyValueInput::new(
                            "env",
                            [
                                ("RUST_LOG".to_string(), "info".to_string()),
                                ("PORT".to_string(), "8080".to_string()),
                            ],
                        )
                        .on_change(|pairs, _, _| log::info!("gallery: {} pairs", pairs.len())),
                    ),
            )
            .child(
                div()
                    .w(px(240.0))
                    .when(touch, |column| column.w_full().min_w_0())
                    .max_w_full()
                    .min_w_0()
                    .child(
                        ListInput::new("hosts", ["ely.dev".to_string()])
                            .placeholder("Host")
                            .on_change(|items, _, _| log::info!("gallery: {} hosts", items.len())),
                    ),
            ),
    )
}

pub fn path(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let mobile = cfg!(any(target_os = "ios", target_os = "android"));
    let path_value = field("input-path", window, cx, |input| {
        input.placeholder(if mobile {
            "/path/to/file"
        } else {
            "~/Projects"
        })
    });
    section(
        "PathInput",
        if mobile {
            "Browse imports a file and fills the field with its path."
        } else {
            "Type a path, or choose a folder with Browse."
        },
        cx,
    )
    .child(probe_sized(
        "path",
        {
            let picker = PathInput::new("path-browse", &path_value);
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            let picker = picker.directories();
            div()
                .w(px(420.0))
                .when(cx.theme().touch_mode, |view| view.w_full().min_w_0())
                .child(picker)
        },
        cx.theme().touch_mode,
    ))
}

pub fn regex(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let pattern = field("input-regex", window, cx, |input| {
        input.placeholder("^[a-z]+$").highlighter(regex_highlights)
    });
    section(
        "RegexInput",
        "Colored by part as you type; a broken pattern says why.",
        cx,
    )
    .child(probe_sized(
        "regex",
        div()
            .w(px(420.0))
            .when(cx.theme().touch_mode, |view| view.w_full().min_w_0())
            .child(RegexInput::new(&pattern)),
        cx.theme().touch_mode,
    ))
}

pub fn expression(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let formula = field("input-formula", window, cx, |input| {
        input
            .placeholder("price * qty")
            .highlighter(expression_highlights)
    });
    section(
        "ExpressionInput",
        "A formula, colored and evaluated as you type.",
        cx,
    )
    .child(probe_sized(
        "expression",
        div()
            .w(px(420.0))
            .when(cx.theme().touch_mode, |view| view.w_full().min_w_0())
            .child(
                ExpressionInput::new(&formula)
                    .variable("price", 12.5)
                    .variable("qty", 3.0),
            ),
        cx.theme().touch_mode,
    ))
    .child(code(
        "price = 12.5, qty = 3; abs sqrt round floor ceil min max",
        cx,
    ))
}

pub fn hotkey(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let recorded = window.use_keyed_state("hotkey", cx, |_, _| Keystroke::parse("cmd-k").ok());
    let now = recorded.read(cx).clone();
    section(
        "HotkeyInput / ShortcutRecorder",
        "Focus it and press a shortcut; it keeps that one and lets go.",
        cx,
    )
    .child(probe(
        "hotkey",
        div()
            .w(px(260.0))
            .child(
                HotkeyInput::new("palette-key", now).on_change(move |next, _, cx| {
                    recorded.update(cx, |recorded, cx| {
                        *recorded = next;
                        cx.notify();
                    })
                }),
            ),
    ))
    .child(Caption::new(
        "Records any key with its modifiers; Tab still moves focus.",
    ))
}
