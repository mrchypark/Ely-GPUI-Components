//! Axis configuration, scaling, and formatting.

use std::sync::Arc;

use crate::charts::advanced::view::Range;

/// Axis scale mapping mode.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum AxisScale {
    /// Standard linear mapping.
    #[default]
    Linear,
    /// Logarithmic mapping with the given base (must be > 0 and != 1).
    Log(f64),
    /// Symmetric logarithmic mapping handling negative values and zero.
    Symlog(f64),
}

/// Formatter for axis tick labels. Use [`AxisFormatter::Custom`] to provide a locale-aware or domain-specific
#[derive(Clone, Default)]
pub enum AxisFormatter {
    /// Default numeric formatter.
    #[default]
    Default,
    /// Custom formatter callback. The function must be thread-safe because plots can be rendered from
    Custom(Arc<dyn Fn(f64) -> String + Send + Sync>),
}

impl AxisFormatter {
    /// Format a value for display.
    pub fn format(&self, value: f64) -> String {
        match self {
            Self::Default => format_default(value),
            Self::Custom(formatter) => formatter(value),
        }
    }
}

fn format_default(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let normalized = if value.abs() < 1e-12 { 0.0 } else { value };
    let raw = format!("{normalized:.6}");
    if let Some(trimmed) = raw.strip_suffix(".000000") {
        trimmed.to_string()
    } else if raw.contains('.') {
        raw.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        raw
    }
}

impl std::fmt::Debug for AxisFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "AxisFormatter::Default"),
            Self::Custom(_) => write!(f, "AxisFormatter::Custom(..)"),
        }
    }
}

/// Axis configuration shared across all series in a plot.
#[derive(Debug, Clone)]
pub struct AxisConfig {
    title: Option<String>,
    units: Option<String>,
    formatter: AxisFormatter,
    tick_config: TickConfig,
    scale: AxisScale,
    time_offset_seconds: Option<i32>,
    show_grid: bool,
    show_minor_grid: bool,
    show_zero_line: bool,
    show_border: bool,
    label_size: f32,
}

impl AxisConfig {
    /// Create a new axis configuration. Use [`AxisConfig::builder`] for a fluent configuration style.
    pub fn new() -> Self {
        Self {
            title: None,
            units: None,
            formatter: AxisFormatter::default(),
            tick_config: TickConfig::default(),
            scale: AxisScale::Linear,
            time_offset_seconds: None,
            show_grid: true,
            show_minor_grid: false,
            show_zero_line: false,
            show_border: true,
            label_size: 12.0,
        }
    }

    /// Start building an axis configuration.
    pub fn builder() -> AxisConfigBuilder {
        AxisConfigBuilder { axis: Self::new() }
    }

    /// Access the axis title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Access the axis units.
    pub fn units(&self) -> Option<&str> {
        self.units.as_deref()
    }

    /// Access the formatter.
    pub fn formatter(&self) -> &AxisFormatter {
        &self.formatter
    }

    /// Format a value for display using the configured formatter.
    pub fn format_value(&self, value: f64) -> String {
        self.formatter.format(value)
    }

    /// Access the tick configuration.
    pub fn tick_config(&self) -> TickConfig {
        self.tick_config
    }

    /// Access the axis scale mode.
    pub fn scale(&self) -> AxisScale {
        self.scale
    }

    /// Check if major grid lines are enabled.
    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    /// Check if minor grid lines are enabled.
    pub fn show_minor_grid(&self) -> bool {
        self.show_minor_grid
    }

    /// Check if the zero line is enabled.
    pub fn show_zero_line(&self) -> bool {
        self.show_zero_line
    }

    /// Check if the axis border is enabled.
    pub fn show_border(&self) -> bool {
        self.show_border
    }

    /// Access the tick label font size.
    pub fn label_size(&self) -> f32 {
        self.label_size
    }
}

/// Builder for [`AxisConfig`].
#[derive(Debug, Clone)]
pub struct AxisConfigBuilder {
    axis: AxisConfig,
}

impl AxisConfigBuilder {
    /// Set the axis title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.axis.title = Some(title.into());
        self
    }

    /// Set the axis units.
    pub fn units(mut self, units: impl Into<String>) -> Self {
        self.axis.units = Some(units.into());
        self
    }

    /// Set the axis formatter. Custom formatters override the default numeric formatting.
    pub fn formatter(mut self, formatter: AxisFormatter) -> Self {
        self.axis.formatter = formatter;
        self
    }

    /// Set the tick configuration. The `pixel_spacing` hint determines how dense major ticks are.
    pub fn tick_config(mut self, config: TickConfig) -> Self {
        self.axis.tick_config = config;
        self
    }

    /// Set the axis scale mode.
    pub fn scale(mut self, scale: AxisScale) -> Self {
        self.axis.scale = scale;
        self
    }

    /// Use a logarithmic axis scale (`base > 0`, `base != 1`).
    pub fn log(mut self, base: f64) -> Self {
        self.axis.scale = AxisScale::Log(base);
        self
    }

    /// Use a symmetric logarithmic axis scale (`constant > 0`).
    pub fn symlog(mut self, constant: f64) -> Self {
        self.axis.scale = AxisScale::Symlog(constant);
        self
    }

    /// Align major ticks to clean wall-clock intervals at the given UTC offset in seconds.
    pub fn time_offset_seconds(mut self, offset_seconds: i32) -> Self {
        self.axis.time_offset_seconds = Some(offset_seconds);
        self
    }

    /// Enable or disable major grid lines.
    pub fn grid(mut self, enabled: bool) -> Self {
        self.axis.show_grid = enabled;
        self
    }

    /// Enable or disable minor grid lines.
    pub fn minor_grid(mut self, enabled: bool) -> Self {
        self.axis.show_minor_grid = enabled;
        self
    }

    /// Enable or disable the zero line.
    pub fn zero_line(mut self, enabled: bool) -> Self {
        self.axis.show_zero_line = enabled;
        self
    }

    /// Enable or disable the axis border.
    pub fn border(mut self, enabled: bool) -> Self {
        self.axis.show_border = enabled;
        self
    }

    /// Set the tick label font size.
    pub fn label_size(mut self, size: f32) -> Self {
        self.axis.label_size = size;
        self
    }

    /// Build the axis configuration.
    pub fn build(self) -> AxisConfig {
        self.axis
    }
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Tick generation configuration. The tick generator uses `pixel_spacing` as a target distance between
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickConfig {
    /// Target pixel spacing between major ticks.
    pub pixel_spacing: f32,
    /// Number of minor ticks between major ticks.
    pub minor_count: usize,
}

impl Default for TickConfig {
    fn default() -> Self {
        Self {
            pixel_spacing: 80.0,
            minor_count: 4,
        }
    }
}

/// Axis tick metadata.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tick {
    /// Tick value in data space.
    pub(crate) value: f64,
    /// Tick label.
    pub(crate) label: String,
    /// Whether the tick is a major tick.
    pub(crate) is_major: bool,
}

/// Layout information for axis labels and ticks.
#[derive(Debug, Clone)]
pub(crate) struct AxisLayout {
    /// Ticks to render.
    pub(crate) ticks: Vec<Tick>,
    /// Maximum tick label size (width, height).
    pub(crate) max_label_size: (f32, f32),
}

impl Default for AxisLayout {
    fn default() -> Self {
        Self {
            ticks: Vec::new(),
            max_label_size: (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AxisLayoutKey {
    range: Range,
    pixels: u32,
    tick_config: TickConfig,
    scale: AxisScale,
    time_offset_seconds: Option<i32>,
}

/// Cached layout for axis ticks and labels.
#[derive(Debug, Default, Clone)]
pub(crate) struct AxisLayoutCache {
    key: Option<AxisLayoutKey>,
    layout: AxisLayout,
}

impl AxisLayoutCache {
    /// Update the cache if inputs have changed.
    pub(crate) fn update(
        &mut self,
        axis: &AxisConfig,
        range: Range,
        pixels: u32,
        measurer: &impl TextMeasurer,
    ) -> &AxisLayout {
        let key = AxisLayoutKey {
            range,
            pixels,
            tick_config: axis.tick_config(),
            scale: axis.scale(),
            time_offset_seconds: axis.time_offset_seconds,
        };
        if self.key.as_ref() == Some(&key) {
            return &self.layout;
        }

        let ticks = generate_ticks(axis, range, pixels as f32);
        let mut max_size = (0.0_f32, 0.0_f32);
        for tick in &ticks {
            if tick.label.is_empty() {
                continue;
            }
            let (w, h) = measurer.measure(&tick.label, axis.label_size());
            max_size.0 = max_size.0.max(w);
            max_size.1 = max_size.1.max(h);
        }

        self.layout = AxisLayout {
            ticks,
            max_label_size: max_size,
        };
        self.key = Some(key);
        &self.layout
    }
}

/// Text measurement interface for layout.
pub(crate) trait TextMeasurer {
    /// Measure a text label at the given size.
    fn measure(&self, text: &str, size: f32) -> (f32, f32);
}

/// Generate axis ticks for a range and pixel length.
fn generate_ticks(axis: &AxisConfig, range: Range, pixel_length: f32) -> Vec<Tick> {
    if !range.is_valid() || pixel_length <= 0.0 {
        return Vec::new();
    }
    match axis.scale() {
        AxisScale::Linear => generate_linear_ticks(axis, range, pixel_length),
        AxisScale::Log(base) => generate_log_ticks(axis, range, base),
        AxisScale::Symlog(constant) => generate_symlog_ticks(axis, range, constant, pixel_length),
    }
}

fn generate_log_ticks(axis: &AxisConfig, range: Range, base: f64) -> Vec<Tick> {
    if !base.is_finite() || base <= 0.0 || (base - 1.0).abs() < 1e-12 {
        return Vec::new();
    }
    let min = range.min.max(1e-12);
    let max = range.max.max(min * 1.0001);
    let e_min = min.log(base).floor() as i32;
    let e_max = max.log(base).ceil() as i32;
    let mut ticks = Vec::new();
    for exp in e_min..=e_max {
        let major = base.powi(exp);
        if major.is_finite() && major >= min && major <= max {
            ticks.push(Tick {
                value: major,
                label: axis.format_value(major),
                is_major: true,
            });
        }
        if (base - 10.0).abs() < 1e-9 {
            for mult in 2..10 {
                let minor = major * mult as f64;
                if minor > min && minor < max {
                    ticks.push(Tick {
                        value: minor,
                        label: String::new(),
                        is_major: false,
                    });
                }
            }
        }
    }
    ticks
}

fn generate_symlog_ticks(
    axis: &AxisConfig,
    range: Range,
    constant: f64,
    pixel_length: f32,
) -> Vec<Tick> {
    let max_abs = range.min.abs().max(range.max.abs());
    if !constant.is_finite() || constant <= 0.0 || max_abs <= constant * 20.0 {
        return generate_linear_ticks(axis, range, pixel_length);
    }
    let mut values = Vec::new();
    if range.min <= 0.0 && range.max >= 0.0 {
        values.push(0.0);
    }
    let max_exp = (max_abs / constant).log10().ceil() as i32;
    for exp in 0..=max_exp {
        let mag = constant * 10f64.powi(exp);
        if mag >= range.min && mag <= range.max {
            values.push(mag);
        }
        if -mag >= range.min && -mag <= range.max {
            values.push(-mag);
        }
    }
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    values
        .into_iter()
        .map(|value| Tick {
            value,
            label: axis.format_value(value),
            is_major: true,
        })
        .collect()
}

mod ticks;
use ticks::generate_linear_ticks;
#[cfg(test)]
mod tests;
