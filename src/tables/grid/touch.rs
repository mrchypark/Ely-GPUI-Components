use gpui::prelude::*;
use gpui::{App, Div, Entity, FocusHandle, ParentElement, Styled, Window, div};

use super::{Face, OnCells, Sheet, Text};
use crate::{
    buttons::Button,
    forms::Editing,
    tables::gridkeys::{begin, clear, fill, fit},
    theme::ActiveTheme,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Select,
    Extend,
    Fill,
}

pub(super) fn reset(sheet: &Entity<Sheet>, cx: &mut App) {
    sheet.update(cx, |sheet, cx| {
        sheet.touch = Mode::Select;
        cx.notify();
    });
}

pub(super) fn prepare(
    face: &Face,
    sheet: &Entity<Sheet>,
    editor: &Entity<Editing>,
    focus: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    let touch = cx.theme().touch_mode;
    let shrunk = sheet.update(cx, |sheet, _| {
        let changed = sheet.dimensions != (face.rows, face.cols);
        let cancel_edit = touch && sheet.editing && (changed || face.on_change.is_none());
        if changed || !touch || face.on_change.is_none() {
            sheet.touch = Mode::Select;
        }
        sheet.dimensions = (face.rows, face.cols);
        sheet.rows = face.rows;
        fit(sheet, (face.rows, face.cols)) || cancel_edit
    });
    if shrunk {
        editor.update(cx, |editing, cx| editing.finish(true, window, cx));
    }
    if sheet.read(cx).editing && editor.read(cx).field().is_none() {
        sheet.update(cx, |sheet, _| sheet.editing = false);
        window.focus(focus, cx);
    }
}

/// Consume a destination before the ordinary tap can replace the source range.
pub(super) fn destination(
    sheet: &Entity<Sheet>,
    cell: (usize, usize),
    raw: &Text,
    heard: &Option<OnCells>,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    if sheet.read(cx).touch != Mode::Fill {
        return false;
    }
    sheet.update(cx, |sheet, cx| {
        sheet.touch = Mode::Select;
        sheet.fill = Some(cell);
        cx.notify();
    });
    fill(sheet, raw, heard, window, cx);
    true
}

pub(super) fn toolbar(
    face: &Face,
    sheet: &Entity<Sheet>,
    editor: &Entity<Editing>,
    focus: &FocusHandle,
    cx: &App,
) -> Div {
    let mut bar = div()
        .flex()
        .flex_wrap()
        .flex_shrink_0()
        .gap_1()
        .debug_selector(|| "grid-touch-toolbar".into());
    let editing = sheet.read(cx).editing;
    let mode = sheet.read(cx).touch;
    let enabled = face.rows > 0 && face.cols > 0 && face.on_change.is_some();
    let actions: &[(&str, &str)] = if editing {
        &[("done", "Done"), ("cancel", "Cancel")]
    } else if mode == Mode::Fill {
        &[("cancel", "Cancel fill")]
    } else if face.fills {
        &[
            ("edit", "Edit"),
            ("extend", "Extend"),
            ("clear", "Clear"),
            ("fill", "Fill"),
        ]
    } else {
        &[("edit", "Edit"), ("extend", "Extend"), ("clear", "Clear")]
    };
    for &(action, label) in actions {
        let sheet = sheet.clone();
        let editor = editor.clone();
        let focus = focus.clone();
        let raw = face.raw.clone();
        let heard = face.on_change.clone();
        let label = if action == "extend" && mode == Mode::Extend {
            "End range"
        } else {
            label
        };
        bar = bar.child(
            div()
                .debug_selector(move || format!("grid-touch-{action}"))
                .child(
                    Button::new((face.id.clone(), action), label)
                        .disabled(!enabled && action != "cancel")
                        .on_click(move |_, window, cx| match action {
                            "edit" => {
                                let cell = sheet.read(cx).cursor;
                                sheet.update(cx, |sheet, _| sheet.touch = Mode::Select);
                                begin(
                                    &sheet,
                                    &editor,
                                    heard.clone(),
                                    raw(cell.0, cell.1).to_string(),
                                    false,
                                    window,
                                    cx,
                                );
                            }
                            "done" | "cancel" if sheet.read(cx).editing => {
                                editor.update(cx, |editor, cx| {
                                    editor.finish(action == "cancel", window, cx)
                                });
                                sheet.update(cx, |sheet, cx| {
                                    sheet.editing = false;
                                    cx.notify();
                                });
                                window.focus(&focus, cx);
                            }
                            "clear" => clear(&sheet, &heard, window, cx),
                            _ => {
                                sheet.update(cx, |sheet, cx| {
                                    sheet.touch = match action {
                                        "extend" if sheet.touch != Mode::Extend => Mode::Extend,
                                        "fill" => Mode::Fill,
                                        _ => Mode::Select,
                                    };
                                    sheet.fill = None;
                                    cx.notify();
                                });
                            }
                        }),
                ),
        );
    }
    if mode == Mode::Fill {
        bar = bar.child("Tap a destination below or to the right");
    }
    bar
}
