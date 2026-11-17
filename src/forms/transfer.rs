use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::{Choice, ListBox, options::OnValues};
use crate::{
    buttons::{ButtonVariant, IconButton},
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
};

/// Marked rows on each side, waiting to move.
#[derive(Default)]
struct Marks {
    left: Vec<SharedString>,
    right: Vec<SharedString>,
}

/// `chosen` after moving `marked` across, in choice order.
fn moved(
    choices: &[Choice],
    chosen: &[SharedString],
    marked: &[SharedString],
    add: bool,
) -> Vec<SharedString> {
    choices
        .iter()
        .map(|choice| &choice.value)
        .filter(|value| {
            if marked.contains(value) {
                add
            } else {
                chosen.contains(value)
            }
        })
        .cloned()
        .collect()
}

/// Two lists and the buttons between them. Mark rows, then move them across.
#[derive(IntoElement)]
pub struct TransferList {
    id: ElementId,
    choices: Vec<Choice>,
    chosen: Vec<SharedString>,
    titles: (SharedString, SharedString),
    on_change: Option<OnValues>,
}

impl TransferList {
    pub fn new(id: impl Into<ElementId>, choices: impl IntoIterator<Item = Choice>) -> Self {
        Self {
            id: id.into(),
            choices: choices.into_iter().collect(),
            chosen: Vec::new(),
            titles: ("Available".into(), "Chosen".into()),
            on_change: None,
        }
    }

    pub fn chosen(mut self, values: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.chosen = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn titles(mut self, left: impl Into<SharedString>, right: impl Into<SharedString>) -> Self {
        self.titles = (left.into(), right.into());
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

impl RenderOnce for TransferList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let stacked = cx.theme().touch_mode
            && window.viewport_size().width < cx.theme().touch_compact_width();
        let marks = window.use_keyed_state((self.id.clone(), "marks"), cx, |_, _| Marks::default());
        let (left, right) = {
            let marks = marks.read(cx);
            (marks.left.clone(), marks.right.clone())
        };
        let (available, picked): (Vec<Choice>, Vec<Choice>) = self
            .choices
            .iter()
            .cloned()
            .partition(|choice| !self.chosen.contains(&choice.value));
        let (choices, chosen) = (Rc::new(self.choices), Rc::new(self.chosen));
        let shift = |add: bool| {
            let (id, choices, chosen, marks, on_change) = (
                self.id.clone(),
                choices.clone(),
                chosen.clone(),
                marks.clone(),
                self.on_change.clone(),
            );
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                let marked = if add {
                    marks.read(cx).left.clone()
                } else {
                    marks.read(cx).right.clone()
                };
                let next = moved(&choices, &chosen, &marked, add);
                log::info!("transfer list {id:?}: {} chosen", next.len());
                marks.update(cx, |marks, cx| {
                    *marks = Marks::default();
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            }
        };
        let side = |key: &'static str,
                    title: SharedString,
                    rows: Vec<Choice>,
                    marked: Vec<SharedString>,
                    cx: &App| {
            let theme = cx.theme();
            let store = marks.clone();
            div()
                .when(stacked, |side| side.w_full().flex_none())
                .when(!stacked, |side| side.flex_1())
                .min_w_0()
                .flex()
                .flex_col()
                .gap_2()
                .debug_selector(move || format!("transfer-{key}"))
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(theme.colors.fg_muted)
                        .child(format!("{title} · {}", rows.len())),
                )
                .child(div().h(theme.list_max_height()).map(|frame| {
                    if rows.is_empty() {
                        return frame;
                    }
                    frame.child(
                        ListBox::new((self.id.clone(), key), rows)
                            .multiple()
                            .selected(marked)
                            .on_change(move |next, _, cx| {
                                let next = next.to_vec();
                                store.update(cx, |marks, cx| {
                                    if key == "left" {
                                        marks.left = next;
                                    } else {
                                        marks.right = next;
                                    }
                                    cx.notify();
                                })
                            }),
                    )
                }))
        };
        let buttons = div()
            .flex()
            .when(stacked, |buttons| {
                buttons.flex_row().w_full().justify_center()
            })
            .when(!stacked, |buttons| buttons.flex_col().justify_center())
            .gap_2()
            .debug_selector(|| "transfer-actions".into())
            .child(
                IconButton::new((self.id.clone(), "add"), IconName::ChevronRight)
                    .variant(ButtonVariant::Outline)
                    .tooltip("Move to chosen")
                    .disabled(left.is_empty())
                    .on_click(shift(true)),
            )
            .child(
                IconButton::new((self.id.clone(), "remove"), IconName::ChevronLeft)
                    .variant(ButtonVariant::Outline)
                    .tooltip("Move back")
                    .disabled(right.is_empty())
                    .on_click(shift(false)),
            );
        let (available_title, chosen_title) = self.titles.clone();
        div()
            .w_full()
            .flex()
            .when(stacked, |list| list.flex_col().w_full())
            .gap_3()
            .min_w_0()
            .debug_selector(|| "transfer-layout".into())
            .child(side("left", available_title, available, left, cx))
            .child(buttons)
            .child(side("right", chosen_title, picked, right, cx))
    }
}

#[cfg(test)]
mod tests {
    use gpui::{
        Context, InteractiveElement, IntoElement, ParentElement, Render, SharedString, Styled,
        TestAppContext, VisualTestContext, Window, div, px, size,
    };

    use super::{Choice, TransferList, moved};
    use crate::theme::Theme;

    struct Demo;

    impl Render for Demo {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w_full()
                .max_w(px(560.))
                .debug_selector(|| "transfer-demo".into())
                .child(TransferList::new(
                    "transfer-test",
                    [Choice::new("a", "Amsterdam"), Choice::new("b", "Berlin")],
                ))
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        for _ in 0..3 {
            cx.run_until_parked();
            cx.update(|window, cx| {
                window.refresh();
                window.draw(cx).clear(cx);
            });
        }
    }

    #[test]
    fn moving_keeps_choice_order() {
        let rows = [
            Choice::new("a", "A"),
            Choice::new("b", "B"),
            Choice::new("c", "C"),
        ];
        let values = |list: &[&str]| {
            list.iter()
                .map(|v| SharedString::from(v.to_string()))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            moved(&rows, &values(&["c"]), &values(&["a"]), true),
            values(&["a", "c"])
        );
        assert_eq!(
            moved(&rows, &values(&["a", "c"]), &values(&["c"]), false),
            values(&["a"])
        );
    }

    #[gpui::test]
    fn phone_stacks_without_horizontal_clipping_and_wide_touch_keeps_columns(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init_for_tests);
        cx.update(|cx| {
            Theme::update(cx, |theme| {
                theme.touch_mode = true;
                theme.reduced_motion = true;
            })
        });
        let (_, cx) = cx.add_window_view(|_, _| Demo);

        cx.simulate_resize(size(px(390.), px(844.)));
        draw(cx);
        let root = cx.debug_bounds("transfer-demo").unwrap();
        let layout = cx.debug_bounds("transfer-layout").unwrap();
        let left = cx.debug_bounds("transfer-left").unwrap();
        let actions = cx.debug_bounds("transfer-actions").unwrap();
        let right = cx.debug_bounds("transfer-right").unwrap();
        assert!(layout.size.width >= root.size.width - px(1.));
        for pane in [left, right] {
            assert!(pane.left() >= root.left() && pane.right() <= root.right());
        }
        assert!(left.bottom() <= actions.top() && actions.bottom() <= right.top());

        cx.simulate_resize(size(px(800.), px(844.)));
        draw(cx);
        let root = cx.debug_bounds("transfer-demo").unwrap();
        let layout = cx.debug_bounds("transfer-layout").unwrap();
        let left = cx.debug_bounds("transfer-left").unwrap();
        let actions = cx.debug_bounds("transfer-actions").unwrap();
        let right = cx.debug_bounds("transfer-right").unwrap();
        assert!(layout.size.width >= root.size.width - px(1.));
        assert!(left.right() <= actions.left() && actions.right() <= right.left());
        assert_eq!(left.top(), right.top());
    }
}
