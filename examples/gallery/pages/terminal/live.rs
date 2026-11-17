#[cfg(not(any(target_os = "ios", target_os = "android")))]
use {ely_gpui_component::terminal::Launch, std::path::PathBuf};

use ely_gpui_component::{
    editor::{FindOptions, FindWidget},
    forms::TextInput,
    layout::{PaneGroup, PaneId},
    navigation::{EditorTab, EditorTabs},
    primitives::IconName,
    terminal::{ShellChoice, ShellSelector, Terminal, TerminalToolbar},
    theme::{ActiveTheme, Radius},
};
use gpui::{
    App, AppContext as _, Axis, Entity, IntoElement, ParentElement, SharedString, Styled, Window,
    div, prelude::*, px,
};

use super::mobile;
use crate::{
    probe::probe,
    ui::{keep, set},
};

/// A build's output as a terminal printed it, colors and all.
const BUILD: &str = "\x1b[1;32m   Compiling\x1b[0m ely-gpui-component v0.1.0\r\n\x1b[1;33mwarning\x1b[0m\x1b[1m: unused variable: `spare`\x1b[0m\r\n  \x1b[1;34m-->\x1b[0m src/main.rs:47:9\r\n\x1b[1;33mwarning\x1b[0m\x1b[1m: field `hint` is never read\x1b[0m\r\n  \x1b[1;34m-->\x1b[0m src/palette.rs:12:5\r\n\x1b[1;32m    Finished\x1b[0m `dev` profile [unoptimized + debuginfo] target(s) in 4.21s\r\n\x1b[1;32m     Running\x1b[0m `target/debug/palette`\r\n\x1b[38;5;110m◆\x1b[0m 5 tints of accent, the last \x1b[38;2;232;243;252m██\x1b[0m #e8f3fc\r\n\x1b[2mdocs: https://github.com/leidianyayi/Ely-GPUI-Components\x1b[0m\r\n";

const SHELLS: [(&str, &str); 3] = [
    ("bash", "/bin/bash"),
    ("zsh", "/bin/zsh"),
    ("sh", "/bin/sh"),
];

/// A shell with no startup files, its prompt the folder's name, in this repository.
#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn launch(shell: usize) -> Launch {
    let (name, program) = SHELLS[shell];
    let args = match name {
        "bash" => vec!["--noprofile".into(), "--norc".into(), "-i".into()],
        "zsh" => vec!["-f".into(), "-i".into()],
        _ => vec!["-i".into()],
    };
    Launch {
        program: Some((program.into(), args)),
        cwd: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        env: vec![
            ("PS1".into(), "\\W › ".into()),
            ("PROMPT".into(), "%1~ › ".into()),
            ("BASH_SILENCE_DEPRECATION_WARNING".into(), "1".into()),
        ],
    }
}

/// A pane group of shells, and the shell each pane holds.
type Sessions = Entity<Vec<(PaneId, Entity<Terminal>)>>;
type Panes = (Entity<PaneGroup>, Sessions);

/// Panes of `shell`, fresh for each `start`: a pick starts new shells even of the same kind.
fn panes(shell: usize, start: usize, replay: bool, window: &mut Window, cx: &mut App) -> Panes {
    let sessions = window.use_keyed_state(("terminal-sessions", start), cx, |_, _| Vec::new());
    let held = sessions.clone();
    let group = window.use_keyed_state(("terminal-panes", start), cx, move |_, _| {
        PaneGroup::new(move |pane, _, cx| {
            session(&held, pane, shell, replay, cx).into_any_element()
        })
    });
    (group, sessions)
}

/// The shell pane `pane` holds, started the first time the pane draws.
fn session(
    sessions: &Sessions,
    pane: PaneId,
    shell: usize,
    replay: bool,
    cx: &mut App,
) -> Entity<Terminal> {
    if let Some((_, terminal)) = sessions.read(cx).iter().find(|(id, _)| *id == pane) {
        return terminal.clone();
    }
    let terminal = cx.new(|cx| {
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        if !replay {
            return Terminal::spawn(launch(shell), cx).expect("the shell starts");
        }
        let _ = replay;
        mobile::replay(shell, cx)
    });
    sessions.update(cx, |all, _| all.push((pane, terminal.clone())));
    terminal
}

fn field(key: &'static str, text: &str, window: &mut Window, cx: &mut App) -> Entity<TextInput> {
    let seed = text.to_string();
    window.use_keyed_state(key, cx, move |window, cx| {
        let mut field = TextInput::new(window, cx);
        field.set_text(seed, cx);
        field
    })
}

/// The shell and the build's output, in tabs.
pub fn shell(window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
    let state = Controls::new(
        cfg!(any(target_os = "ios", target_os = "android")),
        window,
        cx,
    );
    draw(&state, cx)
}

pub(super) struct Controls {
    pub replay: bool,
    pub tab: Entity<SharedString>,
    pub shell: Entity<(usize, usize)>,
    pub group: Entity<PaneGroup>,
    pub sessions: Sessions,
    pub build: Entity<Terminal>,
    pub query: Entity<TextInput>,
    pub options: Entity<FindOptions>,
    searched: Entity<Option<(String, FindOptions)>>,
    pub error: Entity<Option<SharedString>>,
    pub notice: Entity<SharedString>,
}

impl Controls {
    pub fn new(replay: bool, window: &mut Window, cx: &mut App) -> Self {
        let shell = keep("terminal-shell-choice", || (0usize, 0usize), window, cx);
        let (choice, start) = *shell.read(cx);
        let (group, sessions) = panes(choice, start, replay, window, cx);
        Self {
            replay,
            tab: keep("terminal-tab", || "shell".into(), window, cx),
            shell,
            group,
            sessions,
            build: window.use_keyed_state("terminal-build", cx, |_, cx| {
                Terminal::replay(BUILD.as_bytes(), 92, 12, cx)
            }),
            query: field("terminal-find", "warning", window, cx),
            options: keep("terminal-options", FindOptions::default, window, cx),
            searched: keep("terminal-searched", || None, window, cx),
            error: keep("terminal-find-error", || None, window, cx),
            notice: keep(
                "terminal-replay-notice",
                || "Recorded output — no local shell".into(),
                window,
                cx,
            ),
        }
    }
}

pub(super) fn draw(state: &Controls, cx: &mut App) -> impl IntoElement + use<> {
    let Controls {
        replay,
        tab,
        shell,
        group,
        sessions,
        build,
        query,
        options,
        searched,
        error,
        notice,
    } = state;
    let replay = *replay;
    let (now_tab, (now_shell, start)) = (tab.read(cx).clone(), *shell.read(cx));
    let wanted = (query.read(cx).text().to_string(), *options.read(cx));
    if searched.read(cx).as_ref() != Some(&wanted) {
        let found = build.update(cx, |build, cx| build.find(&wanted.0, wanted.1, cx));
        set(error, found.err().map(SharedString::from), cx);
        set(searched, Some(wanted.clone()), cx);
    }
    let (total, current) = build.read(cx).matches();
    let focused = group.read(cx).focused();
    let active = session(sessions, focused, now_shell, replay, cx);
    let (pick, choose, splitter, clearer, stopper) = (
        tab.clone(),
        shell.clone(),
        group.clone(),
        active.clone(),
        active.clone(),
    );
    let shells = ShellSelector::new(
        "terminal-shells",
        SHELLS.map(|(name, program)| ShellChoice {
            name: if replay {
                format!("{name} replay").into()
            } else {
                name.into()
            },
            program: program.into(),
        }),
    )
    .default_shell(now_shell)
    .on_pick({
        let notice = notice.clone();
        move |ix, _, cx| {
            set(&choose, (ix, start + 1), cx);
            if replay {
                set(
                    &notice,
                    "Recorded output reloaded — no local shell".into(),
                    cx,
                );
            }
        }
    });
    let on_shell = now_tab.as_ref() == "shell";
    let exit = active
        .read(cx)
        .exit_status()
        .and_then(|status| status.code());
    let toolbar = TerminalToolbar::new(
        "terminal-toolbar",
        if replay {
            "Replay".to_string()
        } else {
            format!("{} — Ely-GPUI-Components", SHELLS[now_shell].0)
        },
        exit,
    )
    .on_split(move |_, cx| {
        splitter.update(cx, |group, cx| {
            let pane = group.focused();
            group.split(
                pane,
                if replay {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                },
                cx,
            );
        })
    })
    .on_clear(move |_, cx| clearer.update(cx, |terminal, cx| terminal.clear(cx)))
    .on_stop({
        let notice = notice.clone();
        move |_, cx| {
            if replay {
                set(&notice, "Recorded output: no process to stop.".into(), cx);
            } else {
                stopper.update(cx, |terminal, _| terminal.write(&b"\x03"[..]));
            }
        }
    });
    let toolbar = if replay {
        div()
            .w_full()
            .child(
                div().flex().child(
                    div()
                        .flex_none()
                        .debug_selector(|| "terminal-replay-selector".into())
                        .child(shells),
                ),
            )
            .child(
                div()
                    .debug_selector(|| "terminal-replay-toolbar".into())
                    .child(toolbar),
            )
            .into_any_element()
    } else {
        toolbar.shells(shells).into_any_element()
    };
    let (step, flip) = (build.clone(), options.clone());
    let find = FindWidget::new("terminal-find-widget", query, current, total)
        .options(*options.read(cx))
        .on_options(move |next, _, cx| set(&flip, next, cx))
        .on_step(move |forward, _, cx| step.update(cx, |build, cx| build.step(forward, cx)));
    let find = match error.read(cx).clone() {
        Some(message) => find.error(message),
        None => find,
    };
    let tabs = EditorTabs::new(
        "terminal-tabs",
        [
            EditorTab::new(
                "shell",
                if replay {
                    "Replay"
                } else {
                    SHELLS[now_shell].0
                },
            )
            .icon(IconName::Terminal),
            EditorTab::new("build", if replay { "Build replay" } else { "cargo run" })
                .icon(IconName::FileText),
        ],
    )
    .selected(now_tab.clone())
    .on_select(move |id, _, cx| set(&pick, id.clone(), cx));
    let theme = cx.theme();
    let frame = div()
        .debug_selector(|| "terminal-composition".into())
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(theme.colors.border)
        .overflow_hidden();
    if replay {
        return frame
            .w_full()
            .min_w_0()
            .child(
                div()
                    .w_full()
                    .debug_selector(|| "terminal-replay-tabs".into())
                    .child(tabs),
            )
            .children(on_shell.then_some(toolbar))
            .child(
                div()
                    .debug_selector(|| "terminal-replay-notice".into())
                    .child(notice.read(cx).clone()),
            )
            .when(!on_shell, |el| {
                el.child("Swipe search controls horizontally if needed.")
                    .child(mobile::scroll(
                        "terminal-replay-find",
                        find.into_any_element(),
                    ))
            })
            .child(
                div()
                    .w_full()
                    .h(px(300.))
                    .debug_selector(|| "terminal-replay-grid".into())
                    .map(|body| {
                        if on_shell {
                            body.child(group.clone())
                        } else {
                            body.child(build.clone())
                        }
                    }),
            )
            .child(mobile::targets(notice));
    }
    frame
        .w(px(840.))
        .child(probe("terminal-tabs", div().w(px(838.)).child(tabs)))
        .children(on_shell.then_some(toolbar))
        .child(probe(
            "terminal",
            div().w(px(838.)).h(px(300.)).relative().map(|body| {
                if on_shell {
                    body.child(group.clone())
                } else {
                    body.child(build.clone())
                        .child(div().absolute().top_1().right_4().child(find))
                }
            }),
        ))
}
