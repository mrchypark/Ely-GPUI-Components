use std::{collections::HashSet, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};

use super::OnIndex;
use crate::{
    buttons::{ButtonVariant, IconButton, shortcut_text},
    forms::Checkbox,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

/// Whether the program runs or waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugState {
    Running,
    Paused,
}

/// What the debugger is asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugCommand {
    Continue,
    Pause,
    StepOver,
    StepInto,
    StepOut,
    Restart,
    Stop,
}

type OnCommand = Rc<dyn Fn(DebugCommand, &mut Window, &mut App)>;
type OnSwitch = Rc<dyn Fn(usize, bool, &mut Window, &mut App)>;

/// The floating row that drives a debug session: continue or pause, the three steps while paused, restart and stop. Each tooltip names the key debuggers use for it; the app binds the keys.
#[derive(IntoElement)]
pub struct DebugToolbar {
    id: ElementId,
    state: DebugState,
    on_command: Option<OnCommand>,
}

impl DebugToolbar {
    pub fn new(id: impl Into<ElementId>, state: DebugState) -> Self {
        Self {
            id: id.into(),
            state,
            on_command: None,
        }
    }

    pub fn on_command(
        mut self,
        handler: impl Fn(DebugCommand, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_command = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DebugToolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let paused = self.state == DebugState::Paused;
        let platform = theme.platform;
        let button =
            |command: DebugCommand, icon: IconName, words: &str, key: &str, enabled: bool| {
                let run = self.on_command.clone();
                IconButton::new((self.id.clone(), format!("{command:?}")), icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip(format!("{words}  {}", shortcut_text(key, platform)))
                    .disabled(!enabled)
                    .when_some(run, |button, run| {
                        button.on_click(move |_, window, cx| {
                            log::info!("debug toolbar: {command:?}");
                            run(command, window, cx)
                        })
                    })
            };
        let rule = || div().w_0().h_4().border_l_1().border_color(colors.border);
        div()
            .flex()
            .items_center()
            .gap_0p5()
            .p_1()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .child(if paused {
                button(
                    DebugCommand::Continue,
                    IconName::Play,
                    "Continue",
                    "f5",
                    true,
                )
            } else {
                button(DebugCommand::Pause, IconName::Pause, "Pause", "f6", true)
            })
            .child(button(
                DebugCommand::StepOver,
                IconName::RedoDot,
                "Step over",
                "f10",
                paused,
            ))
            .child(button(
                DebugCommand::StepInto,
                IconName::ArrowDownToDot,
                "Step into",
                "f11",
                paused,
            ))
            .child(button(
                DebugCommand::StepOut,
                IconName::ArrowUpFromDot,
                "Step out",
                "shift-f11",
                paused,
            ))
            .child(rule())
            .child(button(
                DebugCommand::Restart,
                IconName::RotateCw,
                "Restart",
                "secondary-shift-f5",
                true,
            ))
            .child(button(
                DebugCommand::Stop,
                IconName::Square,
                "Stop",
                "shift-f5",
                true,
            ))
    }
}

/// A breakpoint: where, whether it is on, the condition that guards it, and how often it has hit.
#[derive(Clone, Debug, PartialEq)]
pub struct Breakpoint {
    pub path: SharedString,
    /// From one.
    pub line: usize,
    pub enabled: bool,
    pub condition: Option<SharedString>,
    pub hits: usize,
}

/// A breakpoint's own key: where it sits and what guards it.
///
/// Reveal 상태는 index가 아니라 이 키로 묶는다. index로 묶으면 항목 하나가 지워질 때
/// 뒤의 항목들이 한 칸씩 밀리며, reveal이 남은 채 다른 항목의 제거 버튼으로 옮겨 붙는다.
fn breakpoint_key(point: &Breakpoint) -> SharedString {
    format!(
        "{}:{}:{}",
        point.path,
        point.line,
        point.condition.as_deref().unwrap_or_default()
    )
    .into()
}

/// Every breakpoint: turn each on or off, see its condition and hits, go to it, or remove it.
#[derive(IntoElement)]
pub struct BreakpointList {
    id: ElementId,
    breakpoints: Vec<Breakpoint>,
    on_toggle: Option<OnSwitch>,
    on_open: Option<OnIndex>,
    on_remove: Option<OnIndex>,
}

impl BreakpointList {
    pub fn new(
        id: impl Into<ElementId>,
        breakpoints: impl IntoIterator<Item = Breakpoint>,
    ) -> Self {
        Self {
            id: id.into(),
            breakpoints: breakpoints.into_iter().collect(),
            on_toggle: None,
            on_open: None,
            on_remove: None,
        }
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(usize, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub fn on_open(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn on_remove(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BreakpointList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A phone never hovers, so a press on a row reveals its remove action.
        // Reveal은 행마다 따로 든다: 몇 행을 눌러도 그 행들만 함께 열리고, 각자 닫힌다.
        let revealed = window.use_keyed_state((self.id.clone(), "remove"), cx, |_, _| {
            HashSet::<SharedString>::new()
        });
        let shown: HashSet<SharedString> = revealed.read(cx).clone();
        // 목록이 바뀌어도 눌린 행의 index가 밀리지 않도록, 지금 보이는 순서를 따로 든다.
        let latest = window.use_keyed_state((self.id.clone(), "latest"), cx, |_, _| {
            Vec::<SharedString>::new()
        });
        let keys: Vec<SharedString> = self.breakpoints.iter().map(breakpoint_key).collect();
        if *latest.read(cx) != keys {
            latest.update(cx, |latest, _| *latest = keys.clone());
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        // 터치에서는 보이지 않은 제거 버튼이 눌리지 않게, reveal된 행에만 동작을 건다.
        let touch = theme.touch_mode;
        let rows: Vec<AnyElement> = self
            .breakpoints
            .iter()
            .enumerate()
            .map(|(ix, point)| {
                let (folder, name) = point.path.rsplit_once('/').unwrap_or(("", &point.path));
                let group = SharedString::from(format!("breakpoint-{ix}"));
                let toggle = self.on_toggle.clone();
                let (open, remove) = (self.on_open.clone(), self.on_remove.clone());
                let press = revealed.clone();
                let press_on_remove = revealed.clone();
                let key = breakpoint_key(point);
                let shown_row = shown.contains(&key);
                let check = Checkbox::new((self.id.clone(), format!("on-{ix}")), point.enabled);
                let check = match toggle {
                    Some(toggle) => check.on_change(move |on, window, cx| {
                        cx.stop_propagation();
                        toggle(ix, on, window, cx)
                    }),
                    None => check,
                };
                let row = div()
                    .id((self.id.clone(), format!("row-{ix}")))
                    .group(group.clone())
                    .debug_selector(move || format!("breakpoint-row-{ix}"))
                    // A phone never hovers, so a press on a row reveals its remove action.
                    // A pointer keeps its hover, so a press there must not change the reveal.
                    .when(touch, |row| {
                        let press = press.clone();
                        let key = key.clone();
                        row.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            press.update(cx, |revealed, cx| {
                                // 같은 행을 다시 누르면 그 행만 닫히고, 다른 행은 그대로 열려 있다.
                                if !revealed.remove(&key) {
                                    revealed.insert(key.clone());
                                }
                                cx.notify();
                            });
                        })
                    })
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(theme.radius(Radius::Sm))
                    .hover(|row| row.bg(colors.hover))
                    .when_some(open, |row, open| {
                        row.cursor_pointer()
                            .on_click(move |_, window, cx| open(ix, window, cx))
                    })
                    .child(check)
                    .child(
                        div()
                            .flex_none()
                            .size_2()
                            .rounded_full()
                            .border_1()
                            .border_color(colors.danger)
                            .when(point.enabled, |dot| dot.bg(colors.danger)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .debug_selector(move || format!("breakpoint-label-{ix}"))
                            .child(format!("{name}:{}", point.line)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg_subtle)
                            .child(Ellipsis::new(folder.to_string())),
                    )
                    .children(point.condition.clone().map(|condition| {
                        div()
                            .flex_none()
                            .font_family(theme.mono_family.clone())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(format!("when {condition}"))
                    }))
                    .when(point.hits > 0, |row| {
                        row.child(
                            tabular(div().flex_none().text_color(colors.fg_subtle))
                                .child(format!("×{}", point.hits)),
                        )
                    })
                    .child(
                        div()
                            .opacity(if shown_row { 1. } else { 0. })
                            .group_hover(group, |actions| actions.opacity(1.))
                            .debug_selector(move || {
                                if shown_row {
                                    "breakpoint-remove-revealed".to_string()
                                } else {
                                    "breakpoint-remove".to_string()
                                }
                            })
                            // 제거 버튼을 누른 press가 행까지 올라가 reveal을 다시 켜지 않게 막는다.
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                div()
                                    .debug_selector(move || {
                                        if shown_row {
                                            format!("breakpoint-remove-revealed-{ix}")
                                        } else {
                                            format!("breakpoint-remove-{ix}")
                                        }
                                    })
                                    .child(
                                        IconButton::new(
                                            (self.id.clone(), format!("remove-{ix}")),
                                            IconName::X,
                                        )
                                        .debug_selector(|| "breakpoint-remove-button".to_string())
                                        .variant(ButtonVariant::Ghost)
                                        .size(ControlSize::Sm)
                                        .tooltip("Remove breakpoint")
                                        .when_some(
                                            remove.filter(|_| !touch || shown_row),
                                            |button, remove| {
                                                let latest = latest.clone();
                                                let key = key.clone();
                                                button.on_click(move |_, window, cx| {
                                                    cx.stop_propagation();
                                                    // 제거와 함께 그 행의 reveal만 닫는다.
                                                    press_on_remove.update(cx, |revealed, cx| {
                                                        revealed.remove(&key);
                                                        cx.notify();
                                                    });
                                                    // 지금 보이는 목록에서의 자리로 알린다.
                                                    let at = latest
                                                        .read(cx)
                                                        .iter()
                                                        .position(|known| *known == key)
                                                        .unwrap_or(ix);
                                                    remove(at, window, cx)
                                                })
                                            },
                                        ),
                                    ),
                            ),
                    )
                    .into_any_element();
                // 행 전체를 가리키는 옛 이름도 남겨 둔다: 행별 이름이 더 정확하지만, 이 이름이 행의 자리다.
                div()
                    .flex()
                    .flex_col()
                    .debug_selector(|| "breakpoint-row".to_string())
                    .child(row)
                    .into_any_element()
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}
