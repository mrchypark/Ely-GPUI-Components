use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, Window};

use super::{Commit, OnSpan, Track};

/// Two thumbs that bound a span. Neither passes the other.
#[derive(IntoElement)]
pub struct RangeSlider {
    id: ElementId,
    value: (f64, f64),
    range: (f64, f64, f64),
    vertical: bool,
    disabled: bool,
    on_change: Option<OnSpan>,
}

impl RangeSlider {
    pub fn new(id: impl Into<ElementId>, low: f64, high: f64) -> Self {
        assert!(low <= high, "range {low}..{high} runs backwards");
        Self {
            id: id.into(),
            value: (low, high),
            range: (0.0, 100.0, 1.0),
            vertical: false,
            disabled: false,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(min < max, "slider range {min}..{max} is empty");
        self.range.0 = min;
        self.range.1 = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        assert!(step > 0.0, "slider step {step} must be positive");
        self.range.2 = step;
        self
    }

    /// Runs bottom to top; the parent gives it a height.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn((f64, f64), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RangeSlider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, (low, high), on_change) = (self.id.clone(), self.value, self.on_change);
        let commit: Commit = Rc::new(move |index, next, force, window, cx| {
            let span = if index == 0 {
                (next.min(high), high)
            } else {
                (low, next.max(low))
            };
            if !force && span == (low, high) {
                return;
            }
            log::debug!("range slider {id:?}: {span:?}");
            if let Some(on_change) = &on_change {
                on_change(span, window, cx);
            }
        });
        Track {
            id: self.id,
            values: vec![low, high],
            range: self.range,
            vertical: self.vertical,
            disabled: self.disabled,
            commit,
        }
        .render(window, cx)
    }
}
