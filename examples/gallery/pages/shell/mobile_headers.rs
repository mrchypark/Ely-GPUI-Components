use ely_gpui_component::{
    buttons::{Button, IconButton},
    primitives::IconName,
    shell::{ColumnShell, ShellColumn, TitleBar},
};
use gpui::{App, Entity, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, px};

use crate::ui::section;

#[derive(Default)]
struct Mail {
    archive_folder: bool,
    archived: bool,
    details: bool,
    close_requests: usize,
}

fn edit(state: &Entity<Mail>, apply: fn(&mut Mail)) -> impl Fn(&mut Window, &mut App) + use<> {
    let state = state.clone();
    move |_, cx| {
        state.update(cx, |state, cx| {
            apply(state);
            cx.notify();
        })
    }
}

pub(super) fn render(window: &mut Window, cx: &mut App, columns: bool) -> impl IntoElement + use<> {
    let id = if columns {
        "mobile-columns"
    } else {
        "mobile-titlebar"
    };
    let state = window.use_keyed_state(id, cx, |_, _| Mail::default());
    let mail = state.read(cx);
    let (archive, archived, details, close_requests) = (
        mail.archive_folder,
        mail.archived,
        mail.details,
        mail.close_requests,
    );
    let folder = if archive { "Archive" } else { "Inbox" };
    let leading = edit(&state, |mail| mail.archive_folder = !mail.archive_folder);
    let action = edit(&state, |mail| mail.details = !mail.details);
    let body_action = edit(&state, |mail| mail.archived = !mail.archived);
    let close = edit(&state, |mail| mail.close_requests += 1);
    let leading = IconButton::new("mail-folder", IconName::Inbox)
        .tooltip("Switch Inbox / Archive")
        .debug_selector(|| "shell-header-leading".into())
        .on_click(move |_, window, cx| leading(window, cx));
    let action = IconButton::new("mail-details", IconName::Info)
        .tooltip("Toggle message details")
        .debug_selector(|| "shell-header-action".into())
        .on_click(move |_, window, cx| action(window, cx));
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            div()
                .debug_selector(|| "shell-header-folder".into())
                .child(folder),
        )
        .children((archive == archived).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .debug_selector(|| "shell-header-message".into())
                .child("Quarterly review — numbers, notes and next steps.")
                .child(
                    Button::new(
                        "mail-archive",
                        if archived {
                            "Restore to Inbox"
                        } else {
                            "Archive message"
                        },
                    )
                    .debug_selector(|| "shell-header-body-action".into())
                    .on_click(move |_, window, cx| body_action(window, cx)),
                )
        }))
        .children((archive != archived).then(|| {
            div()
                .debug_selector(|| "shell-header-empty".into())
                .child("No messages in this folder.")
        }))
        .children(details.then(|| {
            div()
                .debug_selector(|| "shell-header-details".into())
                .child("From Ada · Local gallery message; no mail is sent.")
        }))
        .children((close_requests > 0).then(|| {
            div()
                .debug_selector(|| "shell-header-close-requested".into())
                .child("Host close requested; gallery retained for inspection.")
        }));
    let content = if columns {
        div()
            .h(px(320.))
            .child(
                ColumnShell::new(id).on_close(close).column(
                    ShellColumn::new("mobile-mail-column")
                        .title(folder)
                        .leading(leading)
                        .action(action)
                        .child(body),
                ),
            )
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .child(
                TitleBar::new(id)
                    .title(folder)
                    .on_close(close)
                    .child(leading)
                    .action(action),
            )
            .child(body)
            .into_any_element()
    };
    section(if columns { "ColumnShell / ShellColumn" } else { "TitleBar" },
        "Real touch header with local Inbox/Archive, message details and archive/restore actions. No desktop window controls; compact multi-column navigation remains separate.", cx)
        .child(div().w_full().debug_selector(|| "shell-touch-header-specimen".into()).child(content))
}
