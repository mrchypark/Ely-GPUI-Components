use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, Axis, Bounds, ElementId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Pixels, Point, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, div, point, prelude::*,
};

use crate::{
    layout::reveal_when_focused_in_bounds,
    motion,
    primitives::{Backdrop, FocusScope, Place, give_back, tab_stop, take_focus},
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
};

type Run = Rc<dyn Fn(&mut Window, &mut App)>;

#[cfg(all(test, feature = "test-support"))]
mod tests;

/// A short list of choices rising from the bottom, with its own cancel.
#[derive(IntoElement)]
pub struct ActionSheet {
    id: ElementId,
    usable_bounds: Option<Bounds<Pixels>>,
    title: Option<SharedString>,
    message: Option<SharedString>,
    actions: Vec<(SharedString, bool, Run)>,
    on_close: Run,
}

impl ActionSheet {
    pub fn new(
        id: impl Into<ElementId>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            usable_bounds: None,
            title: None,
            message: None,
            actions: Vec::new(),
            on_close: Rc::new(on_close),
        }
    }

    /// Constrains the sheet to logical window bounds supplied by the host.
    pub fn usable_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.usable_bounds = Some(bounds);
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Runs `handler`, then closes.
    pub fn action(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.push((label.into(), false, Rc::new(handler)));
        self
    }

    /// An action that destroys something, set in the danger tone.
    pub fn destructive(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions.push((label.into(), true, Rc::new(handler)));
        self
    }
}

impl RenderOnce for ActionSheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let viewport = window.viewport_size();
        let bounds = self
            .usable_bounds
            .unwrap_or_else(|| Bounds::new(Point::default(), viewport));
        let left = bounds.left().max(Pixels::ZERO).min(viewport.width);
        let top = bounds.top().max(Pixels::ZERO).min(viewport.height);
        let bounds = Bounds::from_corners(
            point(left, top),
            point(
                bounds.right().max(left).min(viewport.width),
                bounds.bottom().max(top).min(viewport.height),
            ),
        );
        let takeover = take_focus(self.id.clone(), window, cx);
        let close: Run = {
            let (takeover, on_close) = (takeover.clone(), self.on_close);
            Rc::new(move |window, cx| {
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        let scroll = window
            .use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let mut stops = (0..=self.actions.len())
            .map(|ix| {
                let name = if ix == self.actions.len() {
                    "cancel".to_owned()
                } else {
                    format!("action-{ix}")
                };
                let focus = tab_stop(
                    (self.id.clone(), format!("focus-{name}")).into(),
                    true,
                    window,
                    cx,
                );
                let reveal = reveal_when_focused_in_bounds(
                    (self.id.clone(), format!("reveal-{name}")),
                    &scroll,
                    &focus,
                    Axis::Vertical,
                    bounds,
                    window,
                    cx,
                );
                (focus, reveal)
            })
            .collect::<Vec<_>>()
            .into_iter();
        let theme = cx.theme();
        let colors = &theme.colors;
        let row_height = theme.touch_minimum(
            (theme.control_height(ControlSize::Lg) + theme.control_height(ControlSize::Sm) / 2.0)
                .to_pixels(window.rem_size()),
        );
        let card = || {
            div()
                .flex()
                .flex_col()
                .flex_none()
                .overflow_hidden()
                .rounded(theme.radius(Radius::Xl))
                .bg(colors.overlay)
                .border_1()
                .border_color(colors.border)
                .shadow(theme.elevation(Elevation::Modal))
        };
        let mut row = |id: ElementId, label: SharedString, fg, strong: bool| {
            let (focus, reveal) = stops.next().expect("one focus stop per action and cancel");
            div()
                .id(id)
                .debug_selector({
                    let label = label.clone();
                    move || format!("action-sheet-row-{label}")
                })
                .flex()
                .relative()
                .flex_none()
                .items_center()
                .justify_center()
                .min_h(row_height)
                .px_3()
                .text_center()
                .text_size(theme.text_size(TextSize::Md))
                .font_weight(if strong {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(fg)
                .cursor_pointer()
                .track_focus(&focus)
                .focus(|style| style.bg(colors.active))
                .hover(|style| style.bg(colors.hover))
                // Keep the focus reveal canvas aligned with the whole row, including its separator.
                .when(!strong, |row| {
                    row.child(
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .w_full()
                            .border_t_1()
                            .border_color(colors.border),
                    )
                })
                .child(label)
                .child(reveal)
        };
        let heading = (self.title.is_some() || self.message.is_some()).then(|| {
            div()
                .flex()
                .flex_col()
                .flex_none()
                .items_center()
                .gap_1()
                .px_6()
                .py_4()
                .text_center()
                .when_some(self.title, |head, title| {
                    head.child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg_muted)
                            .child(title),
                    )
                })
                .when_some(self.message, |head, message| {
                    head.child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_subtle)
                            .child(message),
                    )
                })
        });
        let rows: Vec<_> = self
            .actions
            .into_iter()
            .enumerate()
            .map(|(ix, (label, danger, run))| {
                let close = close.clone();
                let fg = if danger { colors.danger } else { colors.fg };
                row(("sheet-action", ix).into(), label.clone(), fg, false).on_click(
                    move |_, window, cx| {
                        log::info!("action sheet: {label}");
                        run(window, cx);
                        close(window, cx);
                    },
                )
            })
            .collect();
        let (cancel, escape, dismiss) = (close.clone(), close.clone(), close);
        let focus = takeover.read(cx).focus.clone();
        let panel = div()
            .id("action-sheet")
            .debug_selector(|| "action-sheet-scroll".into())
            .w(theme
                .sheet_size()
                .to_pixels(window.rem_size())
                .min(bounds.size.width))
            .max_h(bounds.size.height)
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .pb_4()
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    escape(window, cx);
                }
            })
            .child(
                FocusScope::new(&focus)
                    .trap()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(card().map(|card| match heading {
                        Some(heading) => card.child(heading).children(rows),
                        None => card.children(rows),
                    }))
                    .child(
                        card().child(
                            row("sheet-cancel".into(), "Cancel".into(), colors.fg, true)
                                .on_click(move |_, window, cx| cancel(window, cx)),
                        ),
                    ),
            )
            .with_animation(
                "action-sheet-in",
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                |panel, t| panel.opacity(t),
            );
        Backdrop::new(self.id)
            .place(Place::Bottom)
            .content_bounds(bounds)
            .on_dismiss(move |window, cx| dismiss(window, cx))
            .child(div().w_full().flex().justify_center().child(panel))
    }
}
