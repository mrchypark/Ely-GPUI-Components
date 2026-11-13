/// Shares past this many ranges lie off any screen; the bound keeps their pixels in `f32`.
const FAR: f64 = 1e4;
const LOG_FLOOR: f64 = 1e-12;

/// Continuous axis mapping mode for cartesian and scatter charts.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum ScaleMode {
    #[default]
    Linear,
    Log(f64),
    Symlog(f64),
}

/// A linear map from a domain of values onto a range of pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Linear {
    pub domain: (f64, f64),
    pub range: (f32, f32),
}

impl Linear {
    pub(crate) fn new(domain: (f64, f64), range: (f32, f32)) -> Self {
        assert!(
            domain.0.is_finite() && domain.1.is_finite(),
            "a scale needs a finite domain"
        );
        Self { domain, range }
    }

    /// The value at pixel `at`; a flat range reads the middle of the domain.
    pub(crate) fn value(&self, at: f32) -> f64 {
        let span = self.range.1 - self.range.0;
        let share = if span == 0.0 {
            0.5
        } else {
            (f64::from(at) - f64::from(self.range.0)) / f64::from(span)
        };
        let (low, high) = (self.domain.0 / 2.0, self.domain.1 / 2.0);
        (low + (high - low) * share) * 2.0
    }

    /// Where `value` lands; a flat domain lands in the middle. Halves keep wide domains finite.
    pub(crate) fn at(&self, value: f64) -> f32 {
        let (low, high) = (self.domain.0 / 2.0, self.domain.1 / 2.0);
        let share = if high == low {
            0.5
        } else {
            ((value / 2.0 - low) / (high - low)).clamp(-FAR, FAR)
        };
        self.range.0 + (self.range.1 - self.range.0) * share as f32
    }
}

/// A logarithmic map from a strictly positive domain onto pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Log {
    pub domain: (f64, f64),
    pub range: (f32, f32),
    pub base: f64,
}

impl Log {
    pub(crate) fn new(domain: (f64, f64), range: (f32, f32), base: f64) -> Self {
        assert!(
            domain.0.is_finite() && domain.1.is_finite() && domain.0 > 0.0 && domain.1 > 0.0,
            "a log scale needs a finite positive domain"
        );
        assert!(
            base.is_finite() && base > 0.0 && (base - 1.0).abs() > 1e-12,
            "a log scale base must be positive and not 1"
        );
        Self {
            domain,
            range,
            base,
        }
    }

    pub(crate) fn at(&self, value: f64) -> f32 {
        let l0 = self.domain.0.log(self.base);
        let l1 = self.domain.1.log(self.base);
        let share = if (l1 - l0).abs() < 1e-15 {
            0.5
        } else {
            let safe = value.max(LOG_FLOOR);
            ((safe.log(self.base) - l0) / (l1 - l0)).clamp(-FAR, FAR)
        };
        self.range.0 + (self.range.1 - self.range.0) * share as f32
    }

    pub(crate) fn value(&self, at: f32) -> f64 {
        let span = self.range.1 - self.range.0;
        let share = if span == 0.0 {
            0.5
        } else {
            (f64::from(at) - f64::from(self.range.0)) / f64::from(span)
        };
        let l0 = self.domain.0.log(self.base);
        let l1 = self.domain.1.log(self.base);
        self.base.powf(l0 + (l1 - l0) * share)
    }
}

/// A symmetric logarithmic map supporting negative values and zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Symlog {
    pub domain: (f64, f64),
    pub range: (f32, f32),
    pub constant: f64,
}

impl Symlog {
    pub(crate) fn new(domain: (f64, f64), range: (f32, f32), constant: f64) -> Self {
        assert!(
            domain.0.is_finite() && domain.1.is_finite(),
            "a symlog scale needs a finite domain"
        );
        assert!(
            constant.is_finite() && constant > 0.0,
            "a symlog constant must be finite and positive"
        );
        Self {
            domain,
            range,
            constant,
        }
    }

    fn forward(&self, x: f64) -> f64 {
        x.signum() * (1.0 + x.abs() / self.constant).ln()
    }

    fn inverse(&self, u: f64) -> f64 {
        u.signum() * self.constant * (u.abs().exp() - 1.0)
    }

    pub(crate) fn at(&self, value: f64) -> f32 {
        let u0 = self.forward(self.domain.0);
        let u1 = self.forward(self.domain.1);
        let share = if (u1 - u0).abs() < 1e-15 {
            0.5
        } else {
            ((self.forward(value) - u0) / (u1 - u0)).clamp(-FAR, FAR)
        };
        self.range.0 + (self.range.1 - self.range.0) * share as f32
    }

    pub(crate) fn value(&self, at: f32) -> f64 {
        let span = self.range.1 - self.range.0;
        let share = if span == 0.0 {
            0.5
        } else {
            (f64::from(at) - f64::from(self.range.0)) / f64::from(span)
        };
        let u0 = self.forward(self.domain.0);
        let u1 = self.forward(self.domain.1);
        self.inverse(u0 + (u1 - u0) * share)
    }
}

/// Unified continuous scale dispatching Linear, Log, or Symlog.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ValueScale {
    Linear(Linear),
    Log(Log),
    Symlog(Symlog),
}

impl ValueScale {
    pub(crate) fn new(mode: ScaleMode, domain: (f64, f64), range: (f32, f32)) -> Self {
        match mode {
            ScaleMode::Linear => Self::Linear(Linear::new(domain, range)),
            ScaleMode::Log(base) => Self::Log(Log::new(domain, range, base)),
            ScaleMode::Symlog(constant) => Self::Symlog(Symlog::new(domain, range, constant)),
        }
    }

    pub(crate) fn at(&self, value: f64) -> f32 {
        match self {
            Self::Linear(s) => s.at(value),
            Self::Log(s) => s.at(value),
            Self::Symlog(s) => s.at(value),
        }
    }

    pub(crate) fn value(&self, at: f32) -> f64 {
        match self {
            Self::Linear(s) => s.value(at),
            Self::Log(s) => s.value(at),
            Self::Symlog(s) => s.value(at),
        }
    }
}

pub(crate) fn nice_log(low: f64, high: f64, base: f64, count: usize) -> ((f64, f64), Vec<f64>) {
    let (low, high) = (low.min(high).max(LOG_FLOOR), low.max(high).max(LOG_FLOOR));
    let (low, high) = if (high / low) <= 1.0 + 1e-12 {
        ((low / base).max(LOG_FLOOR), high * base)
    } else {
        (low, high)
    };
    let e_min = low.log(base).floor() as i32;
    let e_max = high.log(base).ceil() as i32;
    let start = base.powi(e_min).max(LOG_FLOOR);
    let end = base.powi(e_max);
    let span_decades = (e_max - e_min).max(1) as usize;
    let step = (span_decades / count.max(1)).max(1);
    let mut ticks = Vec::new();
    let mut e = e_min;
    while e <= e_max {
        let v = base.powi(e);
        if v.is_finite() {
            ticks.push(v);
        }
        e += step as i32;
    }
    if ticks.len() < 3 && (base - 10.0).abs() < 1e-9 {
        let mut detailed = Vec::new();
        for p in e_min..e_max {
            let decade = base.powi(p);
            for mult in [1.0, 2.0, 5.0] {
                let v = decade * mult;
                if v >= start * 0.999 && v <= end * 1.001 {
                    detailed.push(v);
                }
            }
        }
        detailed.push(end);
        if detailed.len() >= 2 {
            ticks = detailed;
        }
    }
    ((start, end), ticks)
}

pub(crate) fn nice_symlog(
    low: f64,
    high: f64,
    constant: f64,
    count: usize,
) -> ((f64, f64), Vec<f64>) {
    let (domain, mut ticks) = nice(low, high, count);
    if domain.0 < 0.0 && domain.1 > 0.0 && !ticks.iter().any(|t| t.abs() < 1e-9) {
        ticks.push(0.0);
    }
    let max_abs = domain.0.abs().max(domain.1.abs());
    if max_abs > constant * 20.0 {
        let mut log_ticks = Vec::new();
        if domain.0 <= 0.0 && domain.1 >= 0.0 {
            log_ticks.push(0.0);
        }
        let max_exp = (max_abs / constant).log10().ceil() as i32;
        for exp in 0..=max_exp {
            let mag = constant * 10f64.powi(exp);
            if mag >= domain.0 && mag <= domain.1 {
                log_ticks.push(mag);
            }
            if -mag >= domain.0 && -mag <= domain.1 {
                log_ticks.push(-mag);
            }
        }
        log_ticks.sort_by(f64::total_cmp);
        log_ticks.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        if log_ticks.len() >= 2 {
            ticks = log_ticks;
        }
    }
    ticks.sort_by(f64::total_cmp);
    (domain, ticks)
}

pub(crate) fn nice_for(
    mode: ScaleMode,
    low: f64,
    high: f64,
    count: usize,
) -> ((f64, f64), Vec<f64>) {
    match mode {
        ScaleMode::Linear => nice(low, high, count),
        ScaleMode::Log(base) => nice_log(low, high, base, count),
        ScaleMode::Symlog(constant) => nice_symlog(low, high, constant, count),
    }
}

/// A round step near `span / count`: 1, 2 or 5 times a power of ten.
pub(crate) fn nice_step(span: f64, count: usize) -> f64 {
    let raw = span / count.max(1) as f64;
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = nice * power;
    if step > 0.0 && step.is_finite() {
        step
    } else {
        raw
    }
}

/// A domain widened to round steps that hold `low..=high`, and the ticks along it.
pub(crate) fn nice(low: f64, high: f64, count: usize) -> ((f64, f64), Vec<f64>) {
    let (low, high) = (low.min(high), low.max(high));
    let (low, high) = if high - low <= high.abs().max(low.abs()) * 1e-12 {
        let pad = (high.abs().max(low.abs()) * 0.1).max(1.0);
        ((low - pad).max(f64::MIN), (high + pad).min(f64::MAX))
    } else {
        (low, high)
    };
    let step = nice_step(high - low, count);
    let start = ((low / step).floor() * step).max(f64::MIN);
    let end = ((high / step).ceil() * step).min(f64::MAX);
    let ticks = (0..count.max(1) * 4 + 4)
        .map(|ix| start + step * ix as f64)
        .take_while(|tick| *tick <= end + step * 1e-9)
        .map(|tick| if tick.abs() < step * 1e-9 { 0.0 } else { tick })
        .collect();
    ((start, end), ticks)
}

/// Numbers as a chart reads them: thousands as k, millions as M, billions as B, to two places at most.
pub(crate) fn compact(value: f64) -> String {
    let (scaled, unit) = match value.abs() {
        size if size >= 1e9 => (value / 1e9, "B"),
        size if size >= 1e6 => (value / 1e6, "M"),
        size if size >= 1e3 => (value / 1e3, "k"),
        _ => (value, ""),
    };
    let text = format!("{scaled:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{}{unit}", if text == "-0" { "0" } else { text })
}

/// Slots for categories along a range, each with its start and width, a share of each left as padding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Band {
    pub count: usize,
    pub range: (f32, f32),
    pub padding: f32,
}

impl Band {
    /// Slot `ix`'s start and its width, less padding.
    pub(crate) fn slot(&self, ix: usize) -> (f32, f32) {
        let step = (self.range.1 - self.range.0) / self.count.max(1) as f32;
        let width = step * (1.0 - self.padding);
        (
            self.range.0 + step * ix as f32 + (step - width) / 2.0,
            width,
        )
    }

    /// The middle of slot `ix`.
    pub(crate) fn middle(&self, ix: usize) -> f32 {
        let (start, width) = self.slot(ix);
        start + width / 2.0
    }

    /// The slot under pixel `at`, if any.
    pub(crate) fn index(&self, at: f32) -> Option<usize> {
        let step = (self.range.1 - self.range.0) / self.count.max(1) as f32;
        let ix = ((at - self.range.0) / step).floor();
        (ix >= 0.0 && (ix as usize) < self.count).then_some(ix as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scale_maps_values_to_pixels() {
        let scale = Linear::new((0.0, 50.0), (100.0, 0.0));
        assert_eq!(scale.at(25.0), 50.0);
        assert_eq!(scale.at(50.0), 0.0);
        assert_eq!(scale.value(20.0), 40.0, "and back");
        assert_eq!(
            Linear::new((3.0, 3.0), (0.0, 10.0)).at(3.0),
            5.0,
            "a flat domain sits in the middle"
        );
    }

    #[test]
    fn ticks_step_round_and_cover_the_data() {
        assert_eq!(nice_step(100.0, 5), 20.0);
        assert_eq!(nice_step(7.0, 5), 2.0);
        assert_eq!(nice_step(0.3, 3), 0.1);
        let (domain, ticks) = nice(3.0, 97.0, 5);
        assert_eq!(domain, (0.0, 100.0));
        assert_eq!(ticks, [0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
        let (domain, ticks) = nice(-12.0, 8.0, 4);
        assert_eq!(domain, (-15.0, 10.0));
        assert_eq!(ticks.first(), Some(&-15.0));
        assert_eq!(nice(5.0, 5.0, 4).0, (4.0, 6.0), "a single value widens");
        for (low, high) in [(1e30, 1e30), (1e30, 1e30 + 1e14)] {
            let ((from, to), ticks) = nice(low, high, 4);
            assert!(
                from < to && (2..=20).contains(&ticks.len()),
                "a huge flat domain widens by its size: {ticks:?}"
            );
        }
    }

    #[test]
    fn numbers_read_compact() {
        assert_eq!(compact(1250.0), "1.25k");
        assert_eq!(compact(2_000_000.0), "2M");
        assert_eq!(compact(0.5), "0.5");
        assert_eq!(compact(-3400.0), "-3.4k");
        assert_eq!((compact(40.0), compact(-0.001)), ("40".into(), "0".into()));
    }

    #[test]
    fn bands_slot_categories_with_padding() {
        let band = Band {
            count: 4,
            range: (0.0, 400.0),
            padding: 0.2,
        };
        assert_eq!(band.slot(0), (10.0, 80.0));
        assert_eq!(band.middle(3), 350.0);
        assert_eq!(band.index(399.0), Some(3));
        assert_eq!(band.index(-1.0), None);
    }

    #[test]
    fn log_and_symlog_scales_map_and_invert_across_decades() {
        let log = Log::new((1.0, 1000.0), (0.0, 300.0), 10.0);
        assert!((log.at(10.0) - 100.0).abs() < 1e-3);
        assert!((log.at(100.0) - 200.0).abs() < 1e-3);
        assert!((log.value(150.0) - 10f64.powf(1.5)).abs() < 1e-6);
        let (dom, ticks) = nice_log(3.0, 850.0, 10.0, 4);
        assert_eq!(dom, (1.0, 1000.0));
        assert!(ticks.contains(&10.0) && ticks.contains(&100.0));

        let symlog = Symlog::new((-100.0, 100.0), (0.0, 200.0), 1.0);
        assert!((symlog.at(0.0) - 100.0).abs() < 1e-3);
        assert!(((symlog.at(25.0) - 100.0) - (100.0 - symlog.at(-25.0))).abs() < 1e-3);
        assert!((symlog.value(symlog.at(-42.0)) + 42.0).abs() < 1e-4);
    }
}
