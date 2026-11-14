use super::*;

pub(super) fn generate_linear_ticks(
    axis: &AxisConfig,
    range: Range,
    pixel_length: f32,
) -> Vec<Tick> {
    let target = (pixel_length / axis.tick_config().pixel_spacing).max(2.0);
    let raw_step = range.span() / target as f64;
    let step = match axis.time_offset_seconds {
        Some(_) => nice_time_step(raw_step),
        None => nice_step(raw_step),
    };
    if !step.is_finite() || step <= 0.0 {
        return Vec::new();
    }

    let minor_count = axis.tick_config().minor_count;
    let minor_step = step / (minor_count as f64 + 1.0);

    let mut ticks = Vec::new();
    let offset = axis.time_offset_seconds.unwrap_or(0) as f64;
    let mut value = ((range.min + offset) / step).floor() * step - offset;
    if value == -0.0 {
        value = 0.0;
    }
    let max_value = range.max + step * 0.5;

    while value <= max_value {
        if value >= range.min - step * 0.5 {
            ticks.push(Tick {
                value,
                label: axis.format_value(value),
                is_major: true,
            });
        }
        for i in 1..=minor_count {
            let minor = value + minor_step * i as f64;
            if minor >= range.min && minor <= range.max {
                ticks.push(Tick {
                    value: minor,
                    label: String::new(),
                    is_major: false,
                });
            }
        }
        value += step;
    }

    ticks
}

fn nice_step(step: f64) -> f64 {
    if step <= 0.0 {
        return 0.0;
    }
    let exp = step.log10().floor();
    let base = 10_f64.powf(exp);
    let fraction = step / base;
    let nice = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

fn nice_time_step(step: f64) -> f64 {
    const STEPS: [f64; 12] = [
        60.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 10800.0, 14400.0, 21600.0, 43200.0,
        86400.0,
    ];
    if step <= 0.0 {
        return 0.0;
    }
    for candidate in STEPS {
        if step <= candidate {
            return candidate;
        }
    }
    (step / 86400.0).ceil() * 86400.0
}
