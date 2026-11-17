use ely_gpui_component::{
    buttons::{
        ActionBar, ActionSheet, BulkActionBar, Button, ButtonVariant, ConfirmButton, ConfirmMode,
        CopyButton, FloatingActionButton, QuickAction, QuickActions, ShareButton, back_button,
        close_button, more_button,
    },
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius},
    typography::Caption,
};
use gpui::{App, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px};

use crate::{
    probe::probe,
    ui::{code, row, section, specimen, specimens},
};

pub fn later(cx: &App) -> impl IntoElement + use<> {
    section(
        "SplitButton / DropdownButton / MenuButton / LoadingButton",
        "Built from parts in later chapters.",
        cx,
    )
    .child(Caption::new(
        "Split and dropdown buttons open a Menu: they land with Menus, chapter 8.",
    ))
    .child(Caption::new(
        "The loading state spins a Spinner: it lands with Loading, chapter 11.",
    ))
}

pub fn fab(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let width = (window.viewport_size().width - px(16.))
        .max(px(0.))
        .min(px(420.));
    section(
        "FloatingActionButton",
        "Round and raised for the screen's one main action. It rises in.",
        cx,
    )
    .child(
        div()
            .relative()
            .w(width)
            .h(px(160.0))
            .debug_selector(|| "gallery-fab-frame".into())
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.sunken)
            .child(
                div()
                    .absolute()
                    .right_4()
                    .bottom_4()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().debug_selector(|| "gallery-fab-compose".into()).child(
                        FloatingActionButton::new("fab-compose", IconName::Pencil).label("Compose"),
                    ))
                    .child(div().debug_selector(|| "gallery-fab-add".into()).child(
                        FloatingActionButton::new("fab-add", IconName::Plus).tooltip("New item"),
                    )),
            ),
    )
}

pub fn confirm(cx: &App) -> impl IntoElement + use<> {
    section(
        "ConfirmButton / HoldToConfirm",
        "Hold until the fill completes, or click twice within three seconds.",
        cx,
    )
    .child(
        specimens()
            .child(specimen(
                "hold",
                probe(
                    "confirm-hold",
                    ConfirmButton::new("confirm-hold", "Hold to delete", ConfirmMode::Hold)
                        .on_confirm(|_, _| log::info!("gallery: held to delete")),
                ),
                cx,
            ))
            .child(specimen(
                "twice",
                probe(
                    "confirm-twice",
                    ConfirmButton::new("confirm-twice", "Discard draft", ConfirmMode::Twice)
                        .on_confirm(|_, _| log::info!("gallery: draft discarded")),
                ),
                cx,
            )),
    )
}

pub fn small_buttons(cx: &App) -> impl IntoElement + use<> {
    section(
        "CopyButton / ShareButton / CloseButton / BackButton / MoreButton",
        "Small, familiar verbs. Each names itself on hover.",
        cx,
    )
    .child(
        row()
            .child(specimen(
                "copy",
                probe(
                    "copy-button",
                    CopyButton::new("copy-invite", "https://ely.dev/invite/7Q2"),
                ),
                cx,
            ))
            .child(specimen(
                "share",
                probe(
                    "share-button",
                    ShareButton::new("share-page")
                        .text("Ely, components for GPUI")
                        .url("https://github.com/ZacharyZhang-NY/Ely-GPUI-Components"),
                ),
                cx,
            ))
            .child(specimen("close", close_button("preset-close"), cx))
            .child(specimen("back", back_button("preset-back"), cx))
            .child(specimen("more", more_button("preset-more"), cx)),
    )
    .child(code(
        "close_button(id), back_button(id), more_button(id)",
        cx,
    ))
}

pub fn action_bar(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let width = (window.viewport_size().width - px(16.))
        .max(px(0.))
        .min(px(560.));
    section(
        "ActionBar",
        "Docked along a panel's foot, or floating as a pill.",
        cx,
    )
    .child(
        div()
            .w(width)
            .min_h(px(200.0))
            .debug_selector(|| "gallery-action-bar-frame".into())
            .flex()
            .flex_col()
            .justify_end()
            .items_center()
            .overflow_hidden()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.sunken)
            .child(
                div()
                    .pb_4()
                    .debug_selector(|| "gallery-action-bar-floating".into())
                    .child(
                        ActionBar::new()
                            .floating()
                            .child(
                                div()
                                    .debug_selector(|| "gallery-action-reply".into())
                                    .child(
                                        Button::new("bar-reply", "Reply")
                                            .variant(ButtonVariant::Ghost)
                                            .icon(IconName::Reply),
                                    ),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "gallery-action-forward".into())
                                    .child(
                                        Button::new("bar-forward", "Forward")
                                            .variant(ButtonVariant::Ghost)
                                            .icon(IconName::Forward),
                                    ),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "gallery-action-archive".into())
                                    .child(
                                        Button::new("bar-archive", "Archive")
                                            .primary()
                                            .icon(IconName::Archive),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .debug_selector(|| "gallery-action-bar-docked".into())
                    .child(
                        ActionBar::new()
                            .w_full()
                            .child(
                                div()
                                    .debug_selector(|| "gallery-action-cancel".into())
                                    .child(
                                        Button::new("bar-cancel", "Cancel")
                                            .variant(ButtonVariant::Ghost),
                                    ),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "gallery-action-save".into())
                                    .child(Button::new("bar-save", "Save changes").primary()),
                            ),
                    ),
            ),
    )
}

pub fn bulk(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let picked = window.use_keyed_state("bulk-picked", cx, |_, _| 0usize);
    let count = *picked.read(cx);
    let (add, clear) = (picked.clone(), picked);
    section(
        "BulkActionBar",
        "Rises once rows are selected. The count rolls as it changes.",
        cx,
    )
    .child(
        row()
            .child(probe(
                "bulk-select",
                Button::new("bulk-select", "Select one more")
                    .variant(ButtonVariant::Outline)
                    .on_click(move |_, _, cx| {
                        add.update(cx, |count, cx| {
                            *count += 1;
                            cx.notify();
                        })
                    }),
            ))
            .child(Caption::new(format!("{count} selected"))),
    )
    .child(
        div()
            .h(cx.theme().control_height(ControlSize::Lg) * 1.5)
            .child(
                BulkActionBar::new("bulk-bar", count)
                    .action(
                        Button::new("bulk-move", "Move")
                            .variant(ButtonVariant::Ghost)
                            .icon(IconName::Folder),
                    )
                    .action(
                        Button::new("bulk-delete", "Delete")
                            .variant(ButtonVariant::Danger)
                            .icon(IconName::Trash2),
                    )
                    .on_clear(move |_, cx| {
                        clear.update(cx, |count, cx| {
                            *count = 0;
                            cx.notify();
                        })
                    }),
            ),
    )
}

pub fn action_sheet(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> impl IntoElement + use<> {
    let open = window.use_keyed_state("sheet-shown", cx, |_, _| false);
    let shown = *open.read(cx);
    let (show, hide) = (open.clone(), open);
    section(
        "ActionSheet",
        "A few choices from the bottom, and a separate way out.",
        cx,
    )
    .child(probe(
        "action-sheet-open",
        Button::new("action-sheet-open", "Photo options")
            .variant(ButtonVariant::Outline)
            .on_click(move |_, _, cx| {
                show.update(cx, |shown, cx| {
                    *shown = true;
                    cx.notify();
                })
            }),
    ))
    .when(shown, |block| {
        block.child(
            ActionSheet::new("photo-sheet", move |_, cx| {
                hide.update(cx, |shown, cx| {
                    *shown = false;
                    cx.notify();
                })
            })
            .when_some(bounds, |overlay, area| overlay.usable_bounds(area))
            .title("dunes.jpg")
            .message("Taken at dusk. 4.2 MB.")
            .action("Set as wallpaper", |_, _| log::info!("gallery: wallpaper"))
            .action("Duplicate", |_, _| log::info!("gallery: duplicate"))
            .destructive("Delete photo", |_, _| log::info!("gallery: delete photo")),
        )
    })
}

pub fn quick_actions(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let note = |label: &'static str| {
        move |_: &mut Window, _: &mut App| log::info!("gallery: quick {label}")
    };
    let width = (window.viewport_size().width - px(16.))
        .max(px(0.))
        .min(px(420.));
    section(
        "QuickActions",
        "Common moves with their keys. Arrows choose, Enter runs.",
        cx,
    )
    .child(probe(
        "quick",
        div()
            .w(width)
            .debug_selector(|| "gallery-quick-actions-frame".into())
            .child(
                div()
                    .w_full()
                    .debug_selector(|| "gallery-quick-actions-panel".into())
                    .child(
                        QuickActions::new("quick-actions")
                            .action(
                                QuickAction::new(IconName::FilePlus, "New document", note("new"))
                                    .note("Blank, or from a template")
                                    .shortcut("cmd-n"),
                            )
                            .action(
                                QuickAction::new(IconName::FolderOpen, "Open…", note("open"))
                                    .shortcut("cmd-o"),
                            )
                            .action(
                                QuickAction::new(
                                    IconName::Search,
                                    "Search everywhere",
                                    note("search"),
                                )
                                .note("Files, symbols, commands")
                                .shortcut("cmd-shift-p"),
                            )
                            .action(
                                QuickAction::new(IconName::Settings, "Settings", note("settings"))
                                    .shortcut("cmd-,"),
                            ),
                    ),
            ),
    ))
    .child(Caption::new(SharedString::from(
        "Tab in, then use the arrows.",
    )))
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use ely_gpui_component::theme::Theme;
    use gpui::{Bounds, Context, Pixels, Render, TestAppContext, Window, px, size};

    use super::*;

    struct FabDemo;
    struct ActionBarDemo;
    struct QuickActionsDemo;

    impl Render for FabDemo {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            fab(window, cx)
        }
    }

    impl Render for ActionBarDemo {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            action_bar(window, cx)
        }
    }

    impl Render for QuickActionsDemo {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            quick_actions(window, cx)
        }
    }

    #[gpui::test]
    fn fab_actions_fit_phone_and_tablet_widths(cx: &mut TestAppContext) {
        cx.update(ely_gpui_component::init_for_tests);
        let (_, cx) = cx.add_window_view(|_, _| FabDemo);

        for (viewport, expected_width) in [(320., 304.), (768., 420.)] {
            cx.simulate_resize(size(px(viewport), px(844.)));
            for _ in 0..3 {
                cx.update(|window, cx| {
                    window.refresh();
                    window.draw(cx).clear(cx);
                });
            }

            let frame = cx.debug_bounds("gallery-fab-frame").expect("FAB frame");
            let compose = cx.debug_bounds("gallery-fab-compose").expect("compose FAB");
            let add = cx.debug_bounds("gallery-fab-add").expect("add FAB");

            assert_eq!(frame.size.width, px(expected_width));
            assert!(frame.left() >= px(0.) && frame.right() <= px(viewport));
            assert!(compose.left() >= frame.left() && compose.right() <= frame.right());
            assert!(add.left() >= frame.left() && add.right() <= frame.right());
            assert!(compose.right() <= add.left(), "FAB actions overlap");
        }
    }

    #[gpui::test]
    fn action_bars_fit_phone_and_tablet_widths(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (_, cx) = cx.add_window_view(|_, _| ActionBarDemo);
        let inside = |outer: Bounds<Pixels>, inner: Bounds<Pixels>| {
            inner.left() >= outer.left()
                && inner.top() >= outer.top()
                && inner.right() <= outer.right()
                && inner.bottom() <= outer.bottom()
        };
        let overlaps = |a: Bounds<Pixels>, b: Bounds<Pixels>| {
            a.left() < b.right()
                && a.right() > b.left()
                && a.top() < b.bottom()
                && a.bottom() > b.top()
        };

        for (viewport, expected_width) in [(320., 304.), (768., 560.)] {
            cx.simulate_resize(size(px(viewport), px(844.)));
            for _ in 0..3 {
                cx.update(|window, cx| {
                    window.refresh();
                    window.draw(cx).clear(cx);
                });
            }

            let frame = cx
                .debug_bounds("gallery-action-bar-frame")
                .expect("ActionBar frame");
            let floating = cx
                .debug_bounds("gallery-action-bar-floating")
                .expect("floating ActionBar");
            let docked = cx
                .debug_bounds("gallery-action-bar-docked")
                .expect("docked ActionBar");
            let floating_actions = [
                cx.debug_bounds("gallery-action-reply").expect("reply"),
                cx.debug_bounds("gallery-action-forward").expect("forward"),
                cx.debug_bounds("gallery-action-archive").expect("archive"),
            ];
            let docked_actions = [
                cx.debug_bounds("gallery-action-cancel").expect("cancel"),
                cx.debug_bounds("gallery-action-save").expect("save"),
            ];

            assert_eq!(frame.size.width, px(expected_width));
            for bounds in [floating, docked]
                .into_iter()
                .chain(floating_actions)
                .chain(docked_actions)
            {
                assert!(
                    inside(frame, bounds),
                    "{viewport}: {bounds:?} outside {frame:?}"
                );
            }
            for actions in [floating_actions.as_slice(), docked_actions.as_slice()] {
                for (index, left) in actions.iter().enumerate() {
                    for right in &actions[index + 1..] {
                        assert!(
                            !overlaps(*left, *right),
                            "actions overlap: {left:?}, {right:?}"
                        );
                    }
                }
            }
            for action in floating_actions {
                assert!(inside(floating, action));
            }
            for action in docked_actions {
                assert!(inside(docked, action));
            }
        }
    }

    #[gpui::test]
    fn quick_actions_fit_phone_and_tablet_widths(cx: &mut TestAppContext) {
        cx.update(|cx| {
            ely_gpui_component::init_for_tests(cx);
            Theme::update(cx, |theme| theme.touch_mode = true);
        });
        let (_, cx) = cx.add_window_view(|_, _| QuickActionsDemo);

        for (viewport, expected_width) in [(320., 304.), (768., 420.)] {
            cx.simulate_resize(size(px(viewport), px(844.)));
            for _ in 0..3 {
                cx.update(|window, cx| {
                    window.refresh();
                    window.draw(cx).clear(cx);
                });
            }

            let frame = cx
                .debug_bounds("gallery-quick-actions-frame")
                .expect("QuickActions frame");
            let panel = cx
                .debug_bounds("gallery-quick-actions-panel")
                .expect("QuickActions panel");

            assert_eq!(frame.size.width, px(expected_width));
            assert!(frame.left() >= px(0.) && frame.right() <= px(viewport));
            assert!(panel.left() >= frame.left() && panel.right() <= frame.right());
        }
    }
}
