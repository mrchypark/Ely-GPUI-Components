use super::View;
use crate::{
    buttons::Button,
    forms::Editing,
    primitives::tab_stop,
    tables::body::{Body, Line, OnEdit},
    theme::ActiveTheme,
};
use gpui::{App, Div, Entity, ParentElement, SharedString, Styled, Window, div, prelude::*};
use std::rc::Rc;

pub(crate) fn begin(
    view: &Entity<View>,
    editor: &Entity<Editing>,
    target: (SharedString, SharedString, SharedString),
    heard: OnEdit,
    window: &mut Window,
    cx: &mut App,
) {
    let (key, at, words) = target;
    view.update(cx, |view, _| view.editing = Some((key.clone(), at.clone())));
    editor.update(cx, |editing, _| {
        editing.on_commit = Some(Rc::new(move |text, window, cx| {
            heard(&key, &at, text, window, cx)
        }));
    });
    Editing::begin(editor, words.to_string(), window, cx);
}

pub(super) fn controls(
    body: &mut Body,
    shown: &[usize],
    window: &mut Window,
    cx: &mut App,
) -> Option<Div> {
    if !cx.theme().touch_mode {
        return None;
    }
    let valid = |target: &(SharedString, SharedString)| {
        body.on_edit.is_some()
            && body
                .lines
                .iter()
                .any(|line| matches!(line, Line::Row { ix, .. } if body.rows[*ix].key == target.0))
            && shown
                .iter()
                .any(|col| body.columns[*col].key == target.1 && body.columns[*col].editable)
    };
    let target = body.view.read(cx).touch_target.clone().filter(&valid);
    let stale = body.editing.as_ref().is_some_and(|target| !valid(target));
    if stale {
        body.editor
            .update(cx, |editing, cx| editing.finish(true, window, cx));
        body.view.update(cx, |view, _| view.editing = None);
        body.editing = None;
    }
    body.view
        .update(cx, |view, _| view.touch_target = target.clone());
    if body.on_edit.is_none() || !shown.iter().any(|col| body.columns[*col].editable) {
        return None;
    }
    let focus = tab_stop(
        (body.id.clone(), "touch-edit-focus").into(),
        true,
        window,
        cx,
    );
    let mut strip = div()
        .flex()
        .flex_wrap()
        .flex_shrink_0()
        .gap_1()
        .debug_selector(|| "table-touch-tools".into());
    if body.editing.is_some() {
        for (name, cancel) in [("Done", false), ("Cancel", true)] {
            let (view, editor, focus) = (body.view.clone(), body.editor.clone(), focus.clone());
            strip = strip.child(
                div()
                    .debug_selector(move || format!("table-touch-{name}"))
                    .child(Button::new((body.id.clone(), name), name).on_click(
                        move |_, window, cx| {
                            cx.stop_propagation();
                            let before = window.focused(cx);
                            editor.update(cx, |editing, cx| editing.finish(cancel, window, cx));
                            view.update(cx, |view, cx| {
                                view.editing = None;
                                cx.notify();
                            });
                            if window.focused(cx).is_none() || window.focused(cx) == before {
                                window.focus(&focus, cx);
                            }
                        },
                    )),
            );
        }
    } else {
        let selected = target.and_then(|(key, at)| {
            let row = body.rows.iter().find(|row| row.key == key)?;
            let col = body.columns.iter().position(|col| col.key == at)?;
            Some((key, at, row.cells[col].words()))
        });
        let (view, editor, heard) = (
            body.view.clone(),
            body.editor.clone(),
            body.on_edit.clone().unwrap(),
        );
        strip = strip.child(
            div().debug_selector(|| "table-touch-Edit".into()).child(
                Button::new((body.id.clone(), "touch-edit"), "Edit")
                    .focus_handle(&focus)
                    .disabled(selected.is_none())
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        if let Some(target) = &selected {
                            begin(&view, &editor, target.clone(), heard.clone(), window, cx);
                        }
                    }),
            ),
        );
    }
    Some(strip)
}
