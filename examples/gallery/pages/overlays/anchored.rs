use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::Switch,
    overlays::{HoverCard, Popover},
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
};
use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, App, FontWeight, IntoElement, ParentElement, Styled, Window, div};

use crate::{
    probe::probe,
    ui::{keep, row, section, set, specimen},
};

pub fn popovers(
    window: &mut Window,
    cx: &mut App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> impl IntoElement + use<> {
    let public = keep("share-public", || true, window, cx);
    let on = *public.read(cx);
    let theme = cx.theme();
    let muted = theme.colors.fg_muted;
    let panel = move |_: &mut Window, cx: &mut App| {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .w_72()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Share this page"),
            )
            .child(
                div()
                    .text_color(theme.colors.fg_muted)
                    .child("Anyone with the link can view it."),
            )
            .child(
                Switch::new("share-switch", on)
                    .label("Public link")
                    .on_change(move |next, _, cx| set(&public, next, cx)),
            )
            .child(
                Button::new("share-copy", "Copy link")
                    .icon(IconName::Link)
                    .full_width(),
            )
    };
    section(
        "Popover / DropdownPanel / InlinePopup",
        "A panel of any content under its button. A press outside or Escape closes it, and Tab stays inside. The Link variant sets the trigger inline.",
        cx,
    )
    .child(
        row()
            .child(probe(
                "share",
                Popover::new("share", "Share", panel).when_some(bounds, |overlay, area| overlay.usable_bounds(area)).icon(IconName::Share2),
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_color(muted)
                    .child("Read the")
                    .child(
                        Popover::new("terms", "terms", |_, cx| {
                            div()
                                .w_72()
                                .text_color(cx.theme().colors.fg_muted)
                                .child("Plain words: you own your work, and we keep it private.")
                        }).when_some(bounds, |overlay, area| overlay.usable_bounds(area))
                        .variant(ButtonVariant::Link),
                    )
                    .child("before you share."),
            ),
    )
}

fn profile(cx: &App) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    div()
        .flex()
        .flex_col()
        .gap_3()
        .w_64()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_10()
                        .rounded_full()
                        .bg(colors.hover)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("AL"),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Ada Lovelace"),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle)
                                .child("Design · London"),
                        ),
                ),
        )
        .child(
            div()
                .text_color(colors.fg_muted)
                .child("Writes the first programs for machines not yet built."),
        )
        .child(Button::new("ada-follow", "Follow").primary())
        .into_any_element()
}

pub fn hover_card(
    cx: &App,
    bounds: Option<gpui::Bounds<gpui::Pixels>>,
) -> impl IntoElement + use<> {
    let theme = cx.theme();
    section(
        "HoverCard",
        "On desktop, rest the pointer on a name to open a card without taking focus. On touch, press the separate disclosure to open it; the name keeps its primary action.",
        cx,
    )
    .child(specimen(
        "desktop: hover the name; touch: press its disclosure",
        probe(
            "ada",
            HoverCard::new(
                "ada",
                div()
                    .text_color(theme.colors.link)
                    .font_weight(FontWeight::MEDIUM)
                    .child("@ada"),
                |_, cx| profile(cx),
            ).when_some(bounds, |overlay, area| overlay.usable_bounds(area)),
        ),
        cx,
    ))
}
