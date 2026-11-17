use super::*;
use crate::forms::options::opens_up;

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
        let scroll = window
            .use_keyed_state((id.clone(), "popup-scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
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
                let visible = window.fully_visible_bounds();
                let compact =
                    cx.theme().touch_mode || visible.size.width < cx.theme().touch_compact_width();
                let theme = cx.theme();
                let rem = window.rem_size();
                let row = theme.touch_minimum(theme.control_height(ControlSize::Md).to_pixels(rem));
                let estimated =
                    (row * (CALENDAR_ROWS + 1) as f32).min(theme.list_max_height().to_pixels(rem));
                let room = if opens_up(anchor, estimated, visible) {
                    anchor.top()
                } else {
                    window.viewport_size().height - anchor.bottom()
                };
                let max_height = (room - theme.touch_spacing() * 2.0).max(Pixels::ZERO);
                let out = close.clone();
                float(
                    id.clone(),
                    anchor,
                    CALENDAR_ROWS,
                    surface((id.clone(), "popup"), cx)
                        .debug_selector(|| "date-time-popup".into())
                        .flex()
                        .when(compact, |popup| {
                            popup
                                .flex_col()
                                .max_h(max_height)
                                .overflow_y_scroll()
                                .track_scroll(&scroll)
                                .on_scroll(|_, _, cx| cx.stop_propagation())
                        })
                        .on_mouse_down_out(move |_, window, cx| out(window, cx))
                        .child(calendar.on_escape(close).on_pick(move |date, window, cx| {
                            on_day(date.to_datetime(time), window, cx)
                        }))
                        .child(
                            div()
                                .when(compact, |divider| divider.border_t_1())
                                .when(!compact, |divider| divider.border_l_1())
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
