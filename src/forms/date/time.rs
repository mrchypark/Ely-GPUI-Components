use std::{rc::Rc, time::Duration};

use gpui::{
    App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, Role, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div,
    point, prelude::*,
};
use jiff::{
    ToSpan,
    civil::{Date, DateTime, Time},
};

use super::{
    super::{
        NumberInput,
        options::{Run, float, surface},
    },
    picker::{CALENDAR_ROWS, Dropdown, Face, dropdown, picker_field, popped},
    show_date, show_time, zoned_now,
};
use crate::{
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

type OnTime = Rc<dyn Fn(Time, &mut Window, &mut App)>;
type OnDateTime = Rc<dyn Fn(DateTime, &mut Window, &mut App)>;
type OnDuration = Rc<dyn Fn(Duration, &mut Window, &mut App)>;

/// Scroll handles of the two columns, and the time they last scrolled to.
#[derive(Default)]
struct Columns {
    hours: ScrollHandle,
    minutes: ScrollHandle,
    shown: Option<Time>,
}

/// `time` moved by `minutes`, wrapping around midnight.
pub(crate) fn nudged(time: Time, minutes: i64) -> Time {
    time.wrapping_add(minutes.minutes())
}

#[derive(Clone, Copy)]
struct TimeRules {
    min: Option<Time>,
    max: Option<Time>,
    base: Time,
    step: u8,
}

impl TimeRules {
    fn contains(self, time: Time) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) if min > max => time >= min || time <= max,
            (Some(min), Some(max)) => time >= min && time <= max,
            (Some(min), None) => time >= min,
            (None, Some(max)) => time <= max,
            (None, None) => true,
        }
    }

    fn choices(self) -> Vec<Time> {
        let mut times: Vec<_> = (0..(1440 / usize::from(self.step)))
            .map(|ix| nudged(self.base, (ix * usize::from(self.step)) as i64))
            .collect();
        times.sort_unstable();
        times
    }
}

/// Hours and minutes as two scrolling columns. A click sets that part.
fn time_columns(
    id: &ElementId,
    time: Time,
    step: u8,
    rules: Option<TimeRules>,
    on_pick: OnTime,
    window: &mut Window,
    cx: &mut App,
) -> Div {
    let columns = window.use_keyed_state((id.clone(), "columns"), cx, |_, _| Columns::default());
    let row = cx.theme().touch_minimum(
        cx.theme()
            .control_height(ControlSize::Sm)
            .to_pixels(window.rem_size()),
    );
    if columns.read(cx).shown != Some(time) {
        let above = |ix: usize| point(Pixels::ZERO, -(row * ix.saturating_sub(2) as f32));
        columns.update(cx, |columns, _| {
            columns.hours.set_offset(above(time.hour() as usize));
            columns
                .minutes
                .set_offset(above(time.minute() as usize / step as usize));
            columns.shown = Some(time);
        });
    }
    let (hours, minutes) = {
        let columns = columns.read(cx);
        (columns.hours.clone(), columns.minutes.clone())
    };
    let theme = cx.theme();
    let colors = &theme.colors;
    let cell = |key: &'static str, ix: usize, label: String, on: bool, enabled: bool, pick: Run| {
        div()
            .id((key, ix))
            .debug_selector(move || format!("time-{key}-{ix}"))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .h(row)
            .map(|cell| theme.touch_target(cell))
            .role(Role::Button)
            .aria_label(format!(
                "{} {label}{}",
                key,
                if enabled { "" } else { ", unavailable" }
            ))
            .px_3()
            .rounded(theme.radius(Radius::Md))
            .when(enabled, |cell| cell.cursor_pointer())
            .when(!enabled, |cell| cell.opacity(0.4))
            .map(|cell| {
                if on {
                    cell.bg(colors.accent).text_color(colors.on_accent)
                } else {
                    cell.text_color(colors.fg)
                        .hover(|style| style.bg(colors.hover))
                }
            })
            .when(enabled, |cell| {
                cell.on_click(move |_, window, cx| pick(window, cx))
            })
            .child(label)
    };
    let column = |key: &'static str, handle: &ScrollHandle| {
        div()
            .id((id.clone(), key))
            .debug_selector(move || format!("time-column-{key}"))
            .track_scroll(handle)
            .overflow_y_scroll()
            .max_h(theme.list_max_height())
            .p_1()
            .flex()
            .flex_col()
    };
    let choices = rules.map(TimeRules::choices);
    let hour_cells = (0..24).map(|hour| {
        let pick = on_pick.clone();
        let target = choices.as_ref().map(|choices| {
            choices
                .iter()
                .copied()
                .filter(|candidate| candidate.hour() == hour && rules.unwrap().contains(*candidate))
                .min_by_key(|candidate| {
                    (i32::from(candidate.minute()) - i32::from(time.minute())).abs()
                })
        });
        let enabled = target.is_none() || target.flatten().is_some();
        let target = target
            .flatten()
            .unwrap_or_else(|| time.with().hour(hour).build().expect("hours run 0 to 23"));
        cell(
            "hour",
            hour as usize,
            format!("{hour:02}"),
            hour == time.hour(),
            enabled,
            Rc::new(move |window, cx| pick(target, window, cx)),
        )
    });
    let minute_start = rules.map_or(0, |rules| rules.base.minute() as usize % step as usize);
    let minute_cells = (minute_start..60).step_by(step as usize).map(|minute| {
        let pick = on_pick.clone();
        let candidate = choices.as_ref().and_then(|choices| {
            choices.iter().copied().find(|candidate| {
                candidate.hour() == time.hour() && candidate.minute() as usize == minute
            })
        });
        let enabled = rules.is_none()
            || candidate.is_some_and(|candidate| rules.unwrap().contains(candidate));
        let target = candidate.unwrap_or_else(|| {
            time.with()
                .minute(minute as i8)
                .build()
                .expect("minutes run 0 to 59")
        });
        cell(
            "minute",
            minute as usize,
            format!("{minute:02}"),
            minute == time.minute() as usize,
            enabled,
            Rc::new(move |window, cx| pick(target, window, cx)),
        )
    });
    div()
        .flex()
        .text_size(theme.text_size(TextSize::Sm))
        .child(column("hours", &hours).children(hour_cells))
        .child(
            column("minutes", &minutes)
                .border_l_1()
                .border_color(colors.border)
                .children(minute_cells),
        )
}

/// Arrow keys on a time popup move by `step`; Escape and Enter close it.
fn time_keys(
    state: &Entity<Dropdown>,
    current: Time,
    step: u8,
    rules: TimeRules,
    commit: OnTime,
    close: Run,
    cx: &App,
) -> Div {
    div()
        .track_focus(&state.read(cx).inner)
        .on_key_down(move |event, window, cx| {
            let by = match event.keystroke.key.as_str() {
                "up" => -i64::from(step),
                "down" => i64::from(step),
                "escape" | "enter" => {
                    cx.stop_propagation();
                    return close(window, cx);
                }
                _ => return,
            };
            cx.stop_propagation();
            let next = nudged(current, by);
            let choices = rules.choices();
            let wraps = (by > 0 && next < current) || (by < 0 && next > current);
            let reversed = matches!((rules.min, rules.max), (Some(min), Some(max)) if min > max);
            let crosses_limit = wraps
                && !reversed
                && ((by > 0 && rules.max.is_some()) || (by < 0 && rules.min.is_some()));
            if !crosses_limit && rules.contains(next) && choices.contains(&next) {
                commit(next, window, cx);
            } else if !rules.contains(current) || !choices.contains(&current) {
                let candidate = if by > 0 {
                    choices
                        .iter()
                        .copied()
                        .find(|time| *time > current && rules.contains(*time))
                } else {
                    choices
                        .iter()
                        .rev()
                        .copied()
                        .find(|time| *time < current && rules.contains(*time))
                };
                if let Some(candidate) = candidate {
                    commit(candidate, window, cx);
                }
            }
        })
}

/// A field that opens hour and minute columns. Up and Down move by the step.
#[derive(IntoElement)]
pub struct TimePicker {
    id: ElementId,
    value: Option<Time>,
    step: u8,
    bounds: (Option<Time>, Option<Time>),
    step_base: Option<Time>,
    placeholder: SharedString,
    size: ControlSize,
    disabled: bool,
    on_change: Option<OnTime>,
}

impl TimePicker {
    pub fn new(id: impl Into<ElementId>, value: Option<Time>) -> Self {
        Self {
            id: id.into(),
            value,
            step: 5,
            bounds: (None, None),
            step_base: value,
            placeholder: SharedString::from("Pick a time"),
            size: ControlSize::default(),
            disabled: false,
            on_change: None,
        }
    }

    /// Minutes between choices; divides the hour.
    pub fn step(mut self, minutes: u8) -> Self {
        assert!(
            minutes > 0 && 60 % minutes == 0,
            "a {minutes}-minute step does not divide the hour"
        );
        self.step = minutes;
        self
    }

    /// Inclusive bounds; a minimum after the maximum wraps across midnight.
    pub fn range(mut self, min: Option<Time>, max: Option<Time>) -> Self {
        self.bounds = (min, max);
        self
    }

    /// Base for step alignment when no minimum is supplied.
    pub fn step_base(mut self, base: Option<Time>) -> Self {
        self.step_base = base;
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

    pub fn on_change(mut self, handler: impl Fn(Time, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TimePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, step, value) = (self.id.clone(), self.step, self.value);
        let rules = TimeRules {
            min: self.bounds.0,
            max: self.bounds.1,
            base: self.bounds.0.or(self.step_base).unwrap_or(Time::midnight()),
            step,
        };
        let state = dropdown(&id, window, cx);
        let commit: OnTime = {
            let (id, on_change) = (id.clone(), self.on_change);
            Rc::new(move |time, window, cx| {
                log::info!("time picker {id:?}: {time}");
                if let Some(on_change) = &on_change {
                    on_change(time, window, cx);
                }
            })
        };
        picker_field(
            self.id,
            Face {
                icon: IconName::Clock,
                shown: value.map(|time| show_time(time).into()),
                placeholder: self.placeholder,
                size: self.size,
                disabled: self.disabled,
            },
            move |close, anchor, window, cx| {
                let time = value.unwrap_or(Time::midnight());
                let columns =
                    time_columns(&id, time, step, Some(rules), commit.clone(), window, cx);
                let out = close.clone();
                float(
                    id.clone(),
                    anchor,
                    CALENDAR_ROWS,
                    surface((id.clone(), "popup"), cx)
                        .on_mouse_down_out(move |_, window, cx| out(window, cx))
                        .child(
                            time_keys(&state, time, step, rules, commit, close, cx).child(columns),
                        ),
                    window,
                    cx,
                )
            },
            window,
            cx,
        )
    }
}

/// A day and a time: a calendar beside hour and minute columns.
#[derive(IntoElement)]
pub struct DateTimePicker {
    id: ElementId,
    value: Option<DateTime>,
    step: u8,
    placeholder: SharedString,
    today: Option<Date>,
    size: ControlSize,
    on_change: Option<OnDateTime>,
}

impl DateTimePicker {
    pub fn new(id: impl Into<ElementId>, value: Option<DateTime>) -> Self {
        Self {
            id: id.into(),
            value,
            step: 5,
            placeholder: SharedString::from("Pick a date and time"),
            today: None,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    pub fn step(mut self, minutes: u8) -> Self {
        assert!(
            minutes > 0 && 60 % minutes == 0,
            "a {minutes}-minute step does not divide the hour"
        );
        self.step = minutes;
        self
    }

    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(DateTime, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DateTimePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, step, value, today) = (self.id.clone(), self.step, self.value, self.today);
        let state = dropdown(&id, window, cx);
        let commit: OnDateTime = {
            let (id, on_change) = (id.clone(), self.on_change);
            Rc::new(move |at, window, cx| {
                log::info!("date time picker {id:?}: {at}");
                if let Some(on_change) = &on_change {
                    on_change(at, window, cx);
                }
            })
        };
        picker_field(
            self.id,
            Face {
                icon: IconName::CalendarDays,
                shown: value
                    .map(|at| format!("{} {}", show_date(at.date()), show_time(at.time())).into()),
                placeholder: self.placeholder,
                size: self.size,
                disabled: false,
            },
            move |close, anchor, window, cx| {
                let day = value.map_or_else(
                    || today.unwrap_or_else(|| zoned_now().date()),
                    |at| at.date(),
                );
                let time = value.map_or(Time::midnight(), |at| at.time());
                let mut calendar = popped(&id, &state, (None, None, today), cx);
                if let Some(value) = value {
                    calendar = calendar.selected(value.date());
                }
                let (on_day, on_time) = (commit.clone(), commit);
                let columns = time_columns(
                    &id,
                    time,
                    step,
                    None,
                    Rc::new(move |time, window, cx| on_time(day.to_datetime(time), window, cx)),
                    window,
                    cx,
                );
                let out = close.clone();
                float(
                    id.clone(),
                    anchor,
                    CALENDAR_ROWS,
                    surface((id.clone(), "popup"), cx)
                        .flex()
                        .on_mouse_down_out(move |_, window, cx| out(window, cx))
                        .child(calendar.on_escape(close).on_pick(move |date, window, cx| {
                            on_day(date.to_datetime(time), window, cx)
                        }))
                        .child(
                            div()
                                .border_l_1()
                                .border_color(cx.theme().colors.border)
                                .child(columns),
                        ),
                    window,
                    cx,
                )
            },
            window,
            cx,
        )
    }
}

/// Hours, minutes, and optionally seconds, each typed or stepped.
#[derive(IntoElement)]
pub struct DurationPicker {
    id: ElementId,
    value: Duration,
    seconds: bool,
    on_change: Option<OnDuration>,
}

impl DurationPicker {
    pub fn new(id: impl Into<ElementId>, value: Duration) -> Self {
        Self {
            id: id.into(),
            value,
            seconds: false,
            on_change: None,
        }
    }

    /// Adds a seconds field.
    pub fn seconds(mut self) -> Self {
        self.seconds = true;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(Duration, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

/// Whole hours, minutes and seconds in `total`.
pub(crate) fn parts(total: Duration) -> [u64; 3] {
    let secs = total.as_secs();
    [secs / 3600, secs / 60 % 60, secs % 60]
}

impl RenderOnce for DurationPicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let now = parts(self.value);
        let theme = cx.theme();
        let width = theme.control_height(ControlSize::Md) * 4.0;
        let units: &[(&str, &str, f64)] = if self.seconds {
            &[
                ("hours", "h", 9999.0),
                ("minutes", "m", 59.0),
                ("seconds", "s", 59.0),
            ]
        } else {
            &[("hours", "h", 9999.0), ("minutes", "m", 59.0)]
        };
        let fields: Vec<_> = units
            .iter()
            .enumerate()
            .map(|(ix, (key, unit, max))| {
                let (id, on_change) = (self.id.clone(), self.on_change.clone());
                div().w(width).child(
                    NumberInput::new((self.id.clone(), *key), now[ix] as f64)
                        .range(0.0, *max)
                        .suffix(div().child(*unit))
                        .on_change(move |value, window, cx| {
                            let mut next = now;
                            next[ix] = value as u64;
                            let total =
                                Duration::from_secs(next[0] * 3600 + next[1] * 60 + next[2]);
                            log::info!("duration picker {id:?}: {total:?}");
                            if let Some(on_change) = &on_change {
                                on_change(total, window, cx);
                            }
                        }),
                )
            })
            .collect();
        div().flex().items_center().gap_2().children(fields)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jiff::civil::time;

    use super::{nudged, parts};

    #[test]
    fn times_wrap_and_durations_split() {
        assert_eq!(nudged(time(23, 55, 0, 0), 10), time(0, 5, 0, 0));
        assert_eq!(nudged(time(0, 0, 0, 0), -5), time(23, 55, 0, 0));
        assert_eq!(
            parts(Duration::from_secs(3 * 3600 + 25 * 60 + 9)),
            [3, 25, 9]
        );
    }
}
