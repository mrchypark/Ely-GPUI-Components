use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Window, div, prelude::*,
};

use super::{Input, InputEvent, TextInput};
use crate::{
    buttons::Button,
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius},
    typography::Ellipsis,
};

pub(crate) type OnCommit = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A text being edited in place: its field while open, who hears the kept text, and where focus goes back after Enter or Escape. An edit that hands focus back keeps its text on Enter's release, so the release does not open it again.
#[derive(Default)]
pub(crate) struct Editing {
    field: Option<(Entity<TextInput>, Subscription)>,
    pub on_commit: Option<OnCommit>,
    back: Option<FocusHandle>,
}

fn finish_inline(state: &Entity<Editing>, cancel: bool, window: &mut Window, cx: &mut App) {
    state.update(cx, |editing, cx| {
        if editing.field.is_none() {
            return;
        }
        let before = window.focused(cx);
        editing.finish(cancel, window, cx);
        if window.focused(cx).is_none() || window.focused(cx) == before {
            editing.give_back(window, cx);
        } else {
            editing.back = None;
        }
    });
}

impl Editing {
    /// Opens a field on `value`, all of it selected and focused; Enter or leaving keeps it.
    pub(crate) fn begin(state: &Entity<Editing>, value: String, window: &mut Window, cx: &mut App) {
        let field = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(value, cx);
            input
        });
        let all = field.read(cx).text().len();
        field.update(cx, |input, cx| input.select(0..all, cx));
        window.focus(&field.read(cx).focus().clone(), cx);
        let owner = state.clone();
        let events = window.subscribe(&field, cx, move |_, event, window, cx| match event {
            InputEvent::Submit => owner.update(cx, |editing, cx| {
                if editing.back.is_none() {
                    editing.finish(false, window, cx);
                }
            }),
            InputEvent::Blur => owner.update(cx, |editing, cx| {
                editing.back = None;
                editing.finish(false, window, cx);
            }),
            _ => {}
        });
        state.update(cx, |editing, cx| {
            editing.field = Some((field, events));
            cx.notify();
        });
    }

    /// Focus goes back here after Enter or Escape.
    pub(crate) fn returning(&mut self, back: FocusHandle) {
        self.back = Some(back);
    }

    pub(crate) fn give_back(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(back) = self.back.take() {
            log::info!("inline edit: focus handed back");
            window.focus(&back, cx);
        }
    }

    pub(crate) fn field(&self) -> Option<Entity<TextInput>> {
        self.field.as_ref().map(|(field, _)| field.clone())
    }

    /// Ends editing; keeps the text unless `cancel`.
    pub(crate) fn finish(&mut self, cancel: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((field, _)) = self.field.take() else {
            return;
        };
        if cancel {
            log::info!("inline edit: cancelled");
        } else {
            let text = SharedString::from(field.read(cx).text().to_string());
            log::info!("inline edit: kept");
            if let Some(commit) = self.on_commit.clone() {
                commit(&text, window, cx);
            }
        }
        cx.notify();
    }
}

/// Text that becomes a field when clicked. Enter or leaving keeps; Escape reverts. Touch adds Done and Cancel.
#[derive(IntoElement)]
pub struct InlineEdit {
    id: ElementId,
    value: SharedString,
    placeholder: SharedString,
    on_commit: Option<OnCommit>,
}

impl InlineEdit {
    pub fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: "Empty".into(),
            on_commit: None,
        }
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn on_commit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_commit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for InlineEdit {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Editing::default());
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        state.update(cx, |editing, _| editing.on_commit = self.on_commit.clone());
        let touch = cx.theme().touch_mode;
        if let Some(field) = state.read(cx).field() {
            let (escape, enter) = (state.clone(), state.clone());
            return div()
                .id(self.id.clone())
                .when(touch, |editor| editor.flex().flex_col().min_w_0().gap_1())
                .on_key_down(move |event, window, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        finish_inline(&escape, true, window, cx);
                    }
                })
                .on_key_up(move |event, window, cx| {
                    if event.keystroke.key == "enter" {
                        cx.stop_propagation();
                        finish_inline(&enter, false, window, cx);
                    }
                })
                .child(Input::new(&field).size(ControlSize::Sm))
                .when(touch, |editor| {
                    editor.child(
                        div().flex().flex_wrap().gap_1().children(
                            [("done", "Done", false), ("cancel", "Cancel", true)]
                                .into_iter()
                                .map(|(action, label, cancel)| {
                                    let state = state.clone();
                                    Button::new((self.id.clone(), action), label)
                                        .debug_selector(move || format!("inline-edit-{action}"))
                                        .on_click(move |_, window, cx| {
                                            cx.stop_propagation();
                                            finish_inline(&state, cancel, window, cx);
                                        })
                                }),
                        ),
                    )
                })
                .into_any_element();
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let empty = self.value.is_empty();
        let (value, start, back) = (self.value.clone(), state, focus.clone());
        let group = SharedString::from(format!("inline-edit-{:?}", self.id));
        let label = div()
            .debug_selector(|| format!("inline-edit {}", self.id))
            .flex_1()
            .min_w_0()
            .child(Ellipsis::new(if empty {
                self.placeholder
            } else {
                self.value
            }));
        div()
            .id(self.id)
            .debug_selector(|| "inline-edit-opener".into())
            .group(group.clone())
            .flex()
            .items_center()
            .gap_2()
            .h(theme.control_height(ControlSize::Sm))
            .map(|opener| theme.touch_target(opener))
            .px(theme.control_padding(ControlSize::Sm))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(gpui::transparent_black())
            .track_focus(&focus)
            .focus_ring(cx)
            .cursor_text()
            .hover(|style| style.bg(colors.hover))
            .text_color(if empty { colors.fg_subtle } else { colors.fg })
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, window, cx| {
                log::info!("inline edit: editing");
                start.update(cx, |editing, _| editing.returning(back.clone()));
                Editing::begin(&start, value.to_string(), window, cx);
            })
            .child(label)
            .child(
                div()
                    .debug_selector(|| "inline-edit-pencil".into())
                    .flex_none()
                    .when(!touch, |pencil| {
                        pencil
                            .invisible()
                            .group_hover(group, |style| style.visible())
                    })
                    // A phone never hovers, so the pencil shows outright and keeps
                    // a thumb-sized target.
                    .when(touch, |pencil| {
                        pencil
                            .map(|pencil| theme.touch_target(pencil))
                            .debug_selector(|| "inline-edit-pencil-touch".into())
                    })
                    .child(
                        Icon::new(IconName::Pencil)
                            .size(IconSize::Xs)
                            .color(colors.fg_subtle),
                    ),
            )
            .into_any_element()
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
