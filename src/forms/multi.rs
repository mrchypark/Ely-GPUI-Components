use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{
    Choice,
    listbox::chosen,
    options::{OnValues, Pick, Popup, step},
    select::{Picker, measure_anchor, moved},
};
use crate::{
    data_display::Tag,
    i18n,
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// A select for several values. Chosen ones sit in the field as chips.
#[derive(IntoElement)]
pub struct MultiSelect {
    id: ElementId,
    choices: Vec<Choice>,
    selected: Vec<SharedString>,
    placeholder: SharedString,
    size: ControlSize,
    disabled: bool,
    on_change: Option<OnValues>,
}

impl MultiSelect {
    pub fn new(id: impl Into<ElementId>, choices: impl IntoIterator<Item = Choice>) -> Self {
        Self {
            id: id.into(),
            choices: choices.into_iter().collect(),
            selected: Vec::new(),
            placeholder: SharedString::from("Choose…"),
            size: ControlSize::default(),
            disabled: false,
            on_change: None,
        }
    }

    pub fn selected(mut self, values: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MultiSelect {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            self.disabled || !self.choices.is_empty(),
            "multi select {:?} has no choices",
            self.id
        );
        let disabled = self.disabled;
        let focus = tab_stop((self.id.clone(), "focus").into(), !disabled, window, cx);
        let focused = !disabled && focus.is_focused(window);
        let picker =
            window.use_keyed_state((self.id.clone(), "picker"), cx, |_, _| Picker::default());
        if picker.read(cx).open && (disabled || !focused) {
            picker.update(cx, |picker, _| picker.open = false);
        }
        let (choices, selected) = (Rc::new(self.choices), Rc::new(self.selected));
        let start = choices
            .iter()
            .position(|choice| !choice.disabled && selected.contains(&choice.value))
            .unwrap_or_else(|| {
                if choices.is_empty() {
                    0
                } else {
                    step(&choices, choices.len() - 1, 1)
                }
            });
        if picker.read(cx).highlighted >= choices.len() {
            picker.update(cx, |picker, _| picker.highlighted = start);
        }
        let (open, highlighted, anchor, scroll) = {
            let picker = picker.read(cx);
            (
                picker.open,
                picker.highlighted,
                picker.anchor,
                picker.scroll.clone(),
            )
        };
        let reveal = Picker::reveal(&picker, open, highlighted, cx);
        let commit: OnValues = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next, window, cx| {
                log::info!("multi select {id:?}: {next:?}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let toggle: Pick = {
            let (choices, selected, picker, commit) = (
                choices.clone(),
                selected.clone(),
                picker.clone(),
                commit.clone(),
            );
            Rc::new(move |ix, window, cx| {
                if choices[ix].disabled {
                    return;
                }
                Picker::show(&picker, true, ix, cx);
                commit(&chosen(&choices, &selected, ix, true), window, cx);
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let chips: Vec<_> = choices
            .iter()
            .enumerate()
            .filter(|(_, choice)| selected.contains(&choice.value))
            .map(|(ix, choice)| {
                let (choices, selected, commit) =
                    (choices.clone(), selected.clone(), commit.clone());
                let tag = Tag::new(
                    (self.id.clone(), format!("chip-{ix}")),
                    choice.label.clone(),
                );
                let tag = if disabled || choice.disabled {
                    tag
                } else {
                    tag.on_remove(move |window, cx| {
                        commit(&chosen(&choices, &selected, ix, true), window, cx)
                    })
                };
                div()
                    .debug_selector(move || format!("multi-chip-{ix}"))
                    .when(disabled, |chip| chip.opacity(0.5))
                    .child(tag)
            })
            .collect();
        let (click, keys, rows, close) = (
            picker.clone(),
            picker.clone(),
            choices.clone(),
            picker.clone(),
        );
        let (enter, last, erase) = (toggle.clone(), selected.clone(), commit);
        div()
            .debug_selector(|| "multi-select-root".into())
            .id(self.id.clone())
            .track_focus(&focus)
            .role(Role::ComboBox)
            .when(disabled, |field| {
                field.aria_description(i18n::text(cx, "state.unavailable", &[]))
            })
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .min_h(theme.control_height(self.size))
            .pl_1()
            .pr(theme.control_padding(self.size))
            .py_0p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if focused {
                colors.focus
            } else {
                colors.border_strong
            })
            .bg(if disabled {
                colors.sunken
            } else {
                colors.surface
            })
            .when(disabled, |field| field.text_color(colors.fg_disabled))
            .text_size(theme.text_size(TextSize::Base))
            .when(!disabled, |field| field.cursor_pointer())
            .on_click(move |event, _, cx| {
                if !disabled && matches!(event, ClickEvent::Mouse(_)) {
                    Picker::show(&click, !open, start, cx)
                }
            })
            .on_key_down(move |event, window, cx| {
                if disabled {
                    return;
                }
                let at = keys.read(cx).highlighted;
                let key = event.keystroke.key.as_str();
                if key == "backspace" && !last.is_empty() {
                    cx.stop_propagation();
                    if let Some(index) = last.iter().rposition(|value| {
                        rows.iter()
                            .any(|choice| choice.value == *value && !choice.disabled)
                    }) {
                        let mut next = last.as_ref().clone();
                        next.remove(index);
                        erase(&next, window, cx);
                    }
                } else if !open {
                    if matches!(key, "down" | "up" | "enter" | "space") {
                        cx.stop_propagation();
                        Picker::show(&keys, true, start, cx);
                    }
                } else if let Some(to) = moved(event, &rows, at) {
                    cx.stop_propagation();
                    Picker::show(&keys, true, to, cx);
                } else if matches!(key, "enter" | "space") {
                    cx.stop_propagation();
                    enter(at, window, cx);
                } else if key == "escape" {
                    cx.stop_propagation();
                    Picker::show(&keys, false, at, cx);
                }
            })
            .children(chips)
            .when(selected.is_empty(), |field| {
                field.child(
                    div()
                        .pl_2()
                        .text_color(if disabled {
                            colors.fg_disabled
                        } else {
                            colors.fg_subtle
                        })
                        .child(self.placeholder),
                )
            })
            .child(
                div().ml_auto().child(
                    Icon::new(IconName::ChevronsUpDown)
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                ),
            )
            .child(measure_anchor(picker))
            .when(open, |field| {
                field.child(
                    Popup {
                        id: (self.id, "list").into(),
                        anchor,
                        rows: &choices,
                        highlighted: Some(highlighted),
                        checked: Some(&selected),
                        pick: toggle,
                        dismiss: Some(Rc::new(move |_, cx| {
                            let at = close.read(cx).highlighted;
                            Picker::show(&close, false, at, cx)
                        })),
                        scroll: Some(&scroll),
                        reveal,
                    }
                    .render(window, cx),
                )
            })
    }
}
